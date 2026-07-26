//! Resolution of a derived type against the protobuf descriptor: field
//! matching, tag/kind/cardinality extraction, and all the validation that
//! can be done at expansion time. The output is a plan consumed by
//! [`crate::codegen`].

use proc_macro2::Span;
use syn::spanned::Spanned;

use crate::attrs::{self, AttrItem};
use crate::descriptor::{DescriptorIndex, FieldMeta, MessageMeta};
use crate::errors::Errors;
use crate::kind::{Cardinality, FieldKind};

/// Plan for a plain (non-oneof) message struct.
pub(crate) struct MessagePlan {
    pub(crate) ident: syn::Ident,
    /// Full proto names the type stands for (several for unified types).
    pub(crate) proto_names: Vec<String>,
    /// Fields sorted by tag (canonical encode order).
    pub(crate) fields: Vec<FieldPlan>,
    pub(crate) generics: syn::Generics,
    pub(crate) fingerprint: u128,
}

pub(crate) enum FieldAccess {
    Named(syn::Ident),
    Indexed(syn::Index),
}

pub(crate) enum FieldCodec {
    /// Encoded through `ProtoField`.
    Plain,
    /// Encoded through `ProtoAdapter` (`#[armonik(with = "...")]`); skips
    /// kind checks by design.
    Adapter(Box<syn::Type>),
    /// The field covers a whole oneof of the message and is encoded through
    /// `ProtoOneof`; `tags` are the member field tags routed to it.
    OneofGroup { tags: Vec<u32> },
}

/// Compile-time checks emitted alongside the implementation.
pub(crate) struct FieldChecks {
    pub(crate) kind: Option<FieldKind>,
    /// Acceptable runtime cardinalities (e.g. a singular message field may
    /// be either `Singular` or `Optional` in Rust).
    pub(crate) cardinalities: Vec<Cardinality>,
    /// Expected proto type names for message/enum (element) kinds; the
    /// field type's `NAMES` must cover each.
    pub(crate) names: Vec<String>,
    /// Expected map key/value kinds.
    pub(crate) map_kinds: Option<(FieldKind, FieldKind)>,
}

impl FieldChecks {
    fn none() -> Self {
        Self {
            kind: None,
            cardinalities: Vec::new(),
            names: Vec::new(),
            map_kinds: None,
        }
    }
}

pub(crate) struct FieldPlan {
    pub(crate) access: FieldAccess,
    pub(crate) ty: syn::Type,
    pub(crate) span: Span,
    /// Tag of the field (or the lowest member tag for oneof groups), used
    /// for ordering.
    pub(crate) tag: u32,
    pub(crate) codec: FieldCodec,
    pub(crate) checks: FieldChecks,
    /// `TypeName.field_name` of the proto field, for diagnostics.
    pub(crate) proto_path: String,
}

pub(crate) fn message_plan(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
) -> Result<MessagePlan, Errors> {
    let mut errors = Errors::new();

    let entries = match attrs::parse(&input.attrs) {
        Ok(entries) => entries,
        Err(err) => return Err(Errors::from(err)),
    };

    let mut proto_names: Vec<(Span, String)> = Vec::new();
    let mut generic = false;
    for entry in &entries {
        match &entry.item {
            AttrItem::Message(lit) => proto_names.push((entry.span, lit.value())),
            AttrItem::Generic => generic = true,
            AttrItem::Oneof(_) | AttrItem::Transparent => {
                errors.push(syn::Error::new(
                    entry.span,
                    "this armonik attribute mode is not valid here",
                ));
            }
            _ => errors.push(syn::Error::new(
                entry.span,
                "this armonik attribute is not valid at type level on a struct",
            )),
        }
    }
    if generic {
        if !proto_names.is_empty() {
            errors.push(syn::Error::new(
                input.ident.span(),
                "#[armonik(generic)] types are not validated against the descriptor; \
                 remove the message attribute",
            ));
            return Err(errors);
        }
        return generic_plan(input, index, errors);
    }
    if proto_names.is_empty() {
        errors.push(syn::Error::new(
            input.ident.span(),
            "missing #[armonik(message = \"full.proto.Name\")] \
             (or #[armonik(generic)] with explicit tags)",
        ));
        return Err(errors);
    }
    if !input.generics.params.is_empty() {
        errors.push(syn::Error::new(
            input.ident.span(),
            "descriptor-validated types cannot be generic; use #[armonik(generic)]",
        ));
        return Err(errors);
    }

    let mut messages: Vec<(&str, &MessageMeta)> = Vec::new();
    for (span, name) in &proto_names {
        match index.messages.get(name) {
            Some(meta) => messages.push((name, meta)),
            None => errors.push(syn::Error::new(
                *span,
                format!("proto message `{name}` not found in the compiled descriptor set"),
            )),
        }
    }
    if messages.is_empty() {
        return Err(errors);
    }

    let syn::Data::Struct(data) = &input.data else {
        errors.push(syn::Error::new(
            input.ident.span(),
            "#[derive(armonik::Message)] with `message = ...` expects a struct \
             (use `oneof = ...` for flattened oneofs)",
        ));
        return Err(errors);
    };

    let mut fields = Vec::new();
    // Proto fields consumed per message, for the completeness check
    // (indices into `MessageMeta::fields`).
    let mut consumed: Vec<Vec<bool>> = messages
        .iter()
        .map(|(_, meta)| vec![false; meta.fields.len()])
        .collect();
    let mut consumed_oneofs: Vec<Vec<bool>> = messages
        .iter()
        .map(|(_, meta)| vec![false; meta.oneofs.len()])
        .collect();

    for (field_index, field) in data.fields.iter().enumerate() {
        let span = field
            .ident
            .as_ref()
            .map(|ident| ident.span())
            .unwrap_or_else(|| field.ty.span());
        let access = match &field.ident {
            Some(ident) => FieldAccess::Named(ident.clone()),
            None => FieldAccess::Indexed(syn::Index::from(field_index)),
        };

        let field_entries = match attrs::parse(&field.attrs) {
            Ok(entries) => entries,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let mut rename = None;
        let mut explicit_tag = None;
        let mut with = None;
        for entry in &field_entries {
            match &entry.item {
                AttrItem::Rename(lit) => rename = Some(lit.value()),
                AttrItem::Tag(lit) => match lit.base10_parse::<u32>() {
                    Ok(tag) => explicit_tag = Some((entry.span, tag)),
                    Err(err) => errors.push(syn::Error::new(entry.span, err)),
                },
                AttrItem::With(lit) => match syn::parse_str::<syn::Type>(&lit.value()) {
                    Ok(ty) => with = Some(ty),
                    Err(err) => errors.push(syn::Error::new(
                        entry.span,
                        format!("invalid adapter type in with = ...: {err}"),
                    )),
                },
                _ => errors.push(syn::Error::new(
                    entry.span,
                    "this armonik attribute is not valid on a message field",
                )),
            }
        }

        let proto_name = match (&rename, &field.ident) {
            (Some(name), _) => name.clone(),
            (None, Some(ident)) => unraw(ident),
            (None, None) => {
                errors.push(syn::Error::new(
                    span,
                    "tuple struct fields need #[armonik(rename = \"proto_field_name\")]",
                ));
                continue;
            }
        };

        // Resolve against every proto message the type stands for; all of
        // them must agree on the wire contract.
        let mut resolved: Option<(u32, &FieldMeta)> = None;
        let mut oneof_group: Option<Vec<u32>> = None;
        let mut failed = false;
        for (message_index, (message_name, meta)) in messages.iter().enumerate() {
            if let Some(field_meta) = meta.field(&proto_name) {
                let position = meta
                    .fields
                    .iter()
                    .position(|candidate| candidate.name == proto_name)
                    .expect("field was found by name");
                consumed[message_index][position] = true;

                if field_meta.oneof.is_some() {
                    errors.push(syn::Error::new(
                        span,
                        format!(
                            "proto field `{message_name}.{proto_name}` belongs to a oneof; \
                             map the whole oneof to one field named after it"
                        ),
                    ));
                    failed = true;
                } else if let Some((_, previous)) = &resolved {
                    if previous.tag != field_meta.tag
                        || previous.kind != field_meta.kind
                        || previous.cardinality != field_meta.cardinality
                    {
                        errors.push(syn::Error::new(
                            span,
                            format!(
                                "unified messages disagree on field `{proto_name}` \
                                 (tag/kind/cardinality differ); it cannot be derived"
                            ),
                        ));
                        failed = true;
                    }
                } else {
                    resolved = Some((field_meta.tag, field_meta));
                }
            } else if let Some((oneof_index, oneof)) = meta.oneof(&proto_name) {
                consumed_oneofs[message_index][oneof_index] = true;
                let tags: Vec<u32> = oneof
                    .fields
                    .iter()
                    .map(|&field| meta.fields[field].tag)
                    .collect();
                if let Some(previous) = &oneof_group {
                    if *previous != tags {
                        errors.push(syn::Error::new(
                            span,
                            format!(
                                "unified messages disagree on oneof `{proto_name}`; \
                                 it cannot be derived"
                            ),
                        ));
                        failed = true;
                    }
                } else {
                    oneof_group = Some(tags);
                }
            } else {
                let mut available: Vec<&str> = meta
                    .fields
                    .iter()
                    .map(|field| field.name.as_str())
                    .chain(meta.oneofs.iter().map(|oneof| oneof.name.as_str()))
                    .collect();
                available.sort_unstable();
                errors.push(syn::Error::new(
                    span,
                    format!(
                        "no field or oneof named `{proto_name}` in proto message \
                         `{message_name}` (available: {}); use \
                         #[armonik(rename = \"...\")] if the names differ",
                        available.join(", ")
                    ),
                ));
                failed = true;
            }
        }
        if failed {
            continue;
        }

        let proto_path = format!("{}.{proto_name}", messages[0].0);

        if let Some(tags) = oneof_group {
            if with.is_some() || explicit_tag.is_some() {
                errors.push(syn::Error::new(
                    span,
                    "with/tag attributes are not supported on oneof fields",
                ));
                continue;
            }
            let min_tag = tags.iter().copied().min().unwrap_or_default();
            fields.push(FieldPlan {
                access,
                ty: field.ty.clone(),
                span,
                tag: min_tag,
                codec: FieldCodec::OneofGroup { tags },
                checks: FieldChecks::none(),
                proto_path,
            });
            continue;
        }

        let Some((tag, field_meta)) = resolved else {
            continue;
        };

        if let Some((tag_span, tag_value)) = explicit_tag {
            if tag_value != tag {
                errors.push(syn::Error::new(
                    tag_span,
                    format!("tag {tag_value} does not match proto field `{proto_path}` (= {tag})"),
                ));
                continue;
            }
        }

        let (codec, checks) = if let Some(adapter) = with {
            (FieldCodec::Adapter(Box::new(adapter)), FieldChecks::none())
        } else {
            (FieldCodec::Plain, expected_checks(field_meta))
        };

        fields.push(FieldPlan {
            access,
            ty: field.ty.clone(),
            span,
            tag,
            codec,
            checks,
            proto_path,
        });
    }

    // Completeness: every proto field and oneof of every unified message
    // must be covered by a Rust field.
    for (message_index, (message_name, meta)) in messages.iter().enumerate() {
        for (position, field_meta) in meta.fields.iter().enumerate() {
            let in_oneof_group = field_meta
                .oneof
                .is_some_and(|oneof| consumed_oneofs[message_index][oneof]);
            if !consumed[message_index][position] && !in_oneof_group {
                errors.push(syn::Error::new(
                    input.ident.span(),
                    format!(
                        "proto field `{message_name}.{}` (tag {}) is not covered by any \
                         Rust field",
                        field_meta.name, field_meta.tag
                    ),
                ));
            }
        }
        for (oneof_index, oneof) in meta.oneofs.iter().enumerate() {
            let members_covered = oneof
                .fields
                .iter()
                .all(|&field| consumed[message_index][field]);
            if !consumed_oneofs[message_index][oneof_index] && !members_covered {
                errors.push(syn::Error::new(
                    input.ident.span(),
                    format!(
                        "proto oneof `{message_name}.{}` is not covered by any Rust field",
                        oneof.name
                    ),
                ));
            }
        }
    }

    errors.into_result()?;

    fields.sort_by_key(|field| field.tag);
    Ok(MessagePlan {
        ident: input.ident.clone(),
        proto_names: proto_names.into_iter().map(|(_, name)| name).collect(),
        fields,
        generics: input.generics.clone(),
        fingerprint: index.fingerprint,
    })
}

/// Plan for a generic type: no descriptor validation, explicit tags; the
/// concrete instantiations are covered by the differential harness.
fn generic_plan(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
    mut errors: Errors,
) -> Result<MessagePlan, Errors> {
    let syn::Data::Struct(data) = &input.data else {
        errors.push(syn::Error::new(
            input.ident.span(),
            "#[armonik(generic)] expects a struct",
        ));
        return Err(errors);
    };

    let mut fields = Vec::new();
    for (field_index, field) in data.fields.iter().enumerate() {
        let span = field
            .ident
            .as_ref()
            .map(|ident| ident.span())
            .unwrap_or_else(|| field.ty.span());
        let access = match &field.ident {
            Some(ident) => FieldAccess::Named(ident.clone()),
            None => FieldAccess::Indexed(syn::Index::from(field_index)),
        };

        let field_entries = match attrs::parse(&field.attrs) {
            Ok(entries) => entries,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let mut tag = None;
        let mut with = None;
        for entry in &field_entries {
            match &entry.item {
                AttrItem::Tag(lit) => match lit.base10_parse::<u32>() {
                    Ok(value) => tag = Some(value),
                    Err(err) => errors.push(syn::Error::new(entry.span, err)),
                },
                AttrItem::With(lit) => match syn::parse_str::<syn::Type>(&lit.value()) {
                    Ok(ty) => with = Some(ty),
                    Err(err) => errors.push(syn::Error::new(
                        entry.span,
                        format!("invalid adapter type in with = ...: {err}"),
                    )),
                },
                _ => errors.push(syn::Error::new(
                    entry.span,
                    "generic-mode fields only take tag = ... and with = ...",
                )),
            }
        }
        let Some(tag) = tag else {
            errors.push(syn::Error::new(
                span,
                "generic-mode fields need an explicit #[armonik(tag = ...)]",
            ));
            continue;
        };

        let codec = match with {
            Some(adapter) => FieldCodec::Adapter(Box::new(adapter)),
            None => FieldCodec::Plain,
        };
        let proto_path = format!(
            "{}.{}",
            input.ident,
            field
                .ident
                .as_ref()
                .map(|ident| ident.to_string())
                .unwrap_or_else(|| field_index.to_string())
        );
        fields.push(FieldPlan {
            access,
            ty: field.ty.clone(),
            span,
            tag,
            codec,
            checks: FieldChecks::none(),
            proto_path,
        });
    }

    errors.into_result()?;

    fields.sort_by_key(|field| field.tag);
    Ok(MessagePlan {
        ident: input.ident.clone(),
        proto_names: Vec::new(),
        fields,
        generics: input.generics.clone(),
        fingerprint: index.fingerprint,
    })
}

/// Compile-time checks for a plain field, derived from the descriptor.
fn expected_checks(field: &FieldMeta) -> FieldChecks {
    let mut checks = FieldChecks::none();
    match &field.cardinality {
        Cardinality::Map { key, value } => {
            checks.cardinalities = vec![Cardinality::map_marker()];
            checks.map_kinds = Some((key.clone(), value.clone()));
            if let Some(name) = type_name(value) {
                checks.names.push(name.to_owned());
            }
        }
        Cardinality::Repeated { .. } => {
            checks.cardinalities = vec![Cardinality::Repeated { packed: false }];
            checks.kind = Some(field.kind.clone());
            if let Some(name) = type_name(&field.kind) {
                checks.names.push(name.to_owned());
            }
        }
        Cardinality::Optional => {
            checks.cardinalities = vec![Cardinality::Optional];
            checks.kind = Some(field.kind.clone());
            if let Some(name) = type_name(&field.kind) {
                checks.names.push(name.to_owned());
            }
        }
        Cardinality::Singular => {
            // Singular message fields may be either plain ("absent =
            // default") or `Option` (presence-significant) in Rust.
            checks.cardinalities = if matches!(field.kind, FieldKind::Message(_)) {
                vec![Cardinality::Singular, Cardinality::Optional]
            } else {
                vec![Cardinality::Singular]
            };
            checks.kind = Some(field.kind.clone());
            if let Some(name) = type_name(&field.kind) {
                checks.names.push(name.to_owned());
            }
        }
    }
    checks
}

fn type_name(kind: &FieldKind) -> Option<&str> {
    match kind {
        FieldKind::Message(name) | FieldKind::Enum(name) => Some(name),
        _ => None,
    }
}

impl Cardinality {
    /// Marker used by [`expected_checks`]; the payload is irrelevant for
    /// runtime cardinality patterns.
    fn map_marker() -> Self {
        Cardinality::Map {
            key: FieldKind::String,
            value: FieldKind::String,
        }
    }
}

fn unraw(ident: &syn::Ident) -> String {
    ident.to_string().trim_start_matches("r#").to_owned()
}

/// Plan for a protobuf enum (or a transparent single-enum-field wrapper).
pub(crate) struct EnumPlan {
    pub(crate) ident: syn::Ident,
    /// The catch-all variant (`Other`) and its payload struct, which the
    /// derive emits.
    pub(crate) other_variant: syn::Ident,
    pub(crate) payload: syn::Ident,
    /// Named variants with their proto numbers.
    pub(crate) named: Vec<(syn::Ident, i32)>,
    /// Named variant covering 0, when there is one; otherwise the derive
    /// emits an `UNSPECIFIED` const based on the catch-all.
    pub(crate) zero_variant: Option<syn::Ident>,
    /// Whether a variant carries the standard `#[default]` attribute, in
    /// which case the user derives `Default` and the macro must not.
    pub(crate) has_std_default: bool,
    pub(crate) mode: EnumMode,
    pub(crate) fingerprint: u128,
}

pub(crate) enum EnumMode {
    /// The Rust enum is a proto enum, an `int32` varint on the wire.
    Plain { names: Vec<String> },
    /// The Rust enum stands for proto message(s) wrapping an enum field
    /// through a chain of single-field wrappers; `path` holds the tags from
    /// the outermost wrapper down to the enum field.
    Transparent { names: Vec<String>, path: Vec<u32> },
}

pub(crate) fn enum_plan(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
) -> Result<EnumPlan, Errors> {
    let mut errors = Errors::new();

    let entries = match attrs::parse(&input.attrs) {
        Ok(entries) => entries,
        Err(err) => return Err(Errors::from(err)),
    };

    let mut enum_names: Vec<(Span, String)> = Vec::new();
    let mut message_names: Vec<(Span, String)> = Vec::new();
    let mut transparent = false;
    for entry in &entries {
        match &entry.item {
            AttrItem::Enum(lit) => enum_names.push((entry.span, lit.value())),
            AttrItem::Message(lit) => message_names.push((entry.span, lit.value())),
            AttrItem::Transparent => transparent = true,
            _ => errors.push(syn::Error::new(
                entry.span,
                "this armonik attribute is not valid at type level on derive(Enum)",
            )),
        }
    }

    // Resolve the proto enum(s) the variants are matched against, and the
    // wrapper tag in transparent mode.
    let mut proto_enums: Vec<(String, &crate::descriptor::EnumMeta)> = Vec::new();
    let mode = if transparent {
        if message_names.is_empty() {
            errors.push(syn::Error::new(
                input.ident.span(),
                "#[armonik(transparent)] requires #[armonik(message = \"full.proto.Name\")] \
                 naming the single-field wrapper message",
            ));
            return Err(errors);
        }
        let mut wrapper_path: Option<Vec<u32>> = None;
        for (span, name) in &message_names {
            // Follow the chain of single-field wrappers down to the enum.
            let mut current = name.clone();
            let mut path = Vec::new();
            let enum_name = loop {
                let Some(meta) = index.messages.get(&current) else {
                    errors.push(syn::Error::new(
                        *span,
                        format!(
                            "proto message `{current}` not found in the compiled descriptor set"
                        ),
                    ));
                    break None;
                };
                let [field] = meta.fields.as_slice() else {
                    errors.push(syn::Error::new(
                        *span,
                        format!("`{current}` is not a single-field wrapper message"),
                    ));
                    break None;
                };
                path.push(field.tag);
                match &field.kind {
                    FieldKind::Enum(inner) => break Some(inner.clone()),
                    FieldKind::Message(inner) => current = inner.clone(),
                    other => {
                        errors.push(syn::Error::new(
                            *span,
                            format!(
                                "the single field of `{current}` is neither an enum nor a \
                                 wrapper message ({other:?})"
                            ),
                        ));
                        break None;
                    }
                }
            };
            let Some(enum_name) = enum_name else {
                continue;
            };
            if let Some(previous) = &wrapper_path {
                if *previous != path {
                    errors.push(syn::Error::new(
                        *span,
                        "transparent wrapper messages disagree on the wrapper tag path",
                    ));
                }
            } else {
                wrapper_path = Some(path);
            }
            match index.enums.get(&enum_name) {
                Some(enum_meta) => proto_enums.push((enum_name.clone(), enum_meta)),
                None => errors.push(syn::Error::new(
                    *span,
                    format!("proto enum `{enum_name}` not found in the compiled descriptor set"),
                )),
            }
        }
        let Some(path) = wrapper_path else {
            return Err(errors);
        };
        EnumMode::Transparent {
            names: message_names.iter().map(|(_, name)| name.clone()).collect(),
            path,
        }
    } else {
        if enum_names.is_empty() {
            errors.push(syn::Error::new(
                input.ident.span(),
                "missing #[armonik(enum = \"full.proto.Name\")]",
            ));
            return Err(errors);
        }
        for (span, name) in &enum_names {
            match index.enums.get(name) {
                Some(meta) => proto_enums.push((name.clone(), meta)),
                None => errors.push(syn::Error::new(
                    *span,
                    format!("proto enum `{name}` not found in the compiled descriptor set"),
                )),
            }
        }
        EnumMode::Plain {
            names: enum_names.iter().map(|(_, name)| name.clone()).collect(),
        }
    };
    if proto_enums.is_empty() {
        return Err(errors);
    }

    let syn::Data::Enum(data) = &input.data else {
        errors.push(syn::Error::new(
            input.ident.span(),
            "#[derive(armonik::Enum)] expects an enum",
        ));
        return Err(errors);
    };

    // Collect variants: unit variants matched by name, plus exactly one
    // catch-all tuple variant whose payload struct the derive emits.
    let mut named: Vec<(syn::Ident, String)> = Vec::new();
    let mut other: Option<(syn::Ident, syn::Ident)> = None;
    let mut has_std_default = false;
    for variant in &data.variants {
        has_std_default |= variant
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("default"));
        let variant_entries = match attrs::parse(&variant.attrs) {
            Ok(entries) => entries,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let mut rename = None;
        for entry in &variant_entries {
            match &entry.item {
                AttrItem::Rename(lit) => rename = Some(lit.value()),
                _ => errors.push(syn::Error::new(
                    entry.span,
                    "this armonik attribute is not valid on a derive(Enum) variant",
                )),
            }
        }

        match &variant.fields {
            syn::Fields::Unit => {
                let proto_name = rename.unwrap_or_else(|| unraw(&variant.ident));
                named.push((variant.ident.clone(), proto_name));
            }
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let payload = match &fields.unnamed[0].ty {
                    syn::Type::Path(path) if path.qself.is_none() => path.path.get_ident().cloned(),
                    _ => None,
                };
                let Some(payload) = payload else {
                    errors.push(syn::Error::new(
                        variant.ident.span(),
                        "the catch-all payload must be a bare type name; the derive emits \
                         that struct",
                    ));
                    continue;
                };
                if other.replace((variant.ident.clone(), payload)).is_some() {
                    errors.push(syn::Error::new(
                        variant.ident.span(),
                        "derive(Enum) expects exactly one catch-all tuple variant",
                    ));
                }
            }
            _ => errors.push(syn::Error::new(
                variant.ident.span(),
                "derive(Enum) variants must be unit variants or the single catch-all \
                 tuple variant",
            )),
        }
    }
    let Some((other_variant, payload)) = other else {
        errors.push(syn::Error::new(
            input.ident.span(),
            "derive(Enum) requires a catch-all tuple variant, e.g. `Other(OtherTaskStatus)`",
        ));
        return Err(errors);
    };

    // Match every named variant against every proto enum; they must agree.
    let mut resolved: Vec<(syn::Ident, i32)> = Vec::new();
    let mut zero_variant = None;
    for (ident, proto_name) in &named {
        let mut number: Option<i32> = None;
        for (enum_name, meta) in &proto_enums {
            let simple = enum_name.rsplit('.').next().unwrap_or(enum_name);
            let matched = meta.values.iter().find(|(value_name, _)| {
                value_name == proto_name || variant_name(simple, value_name) == *proto_name
            });
            match matched {
                Some((_, value)) => {
                    if *number.get_or_insert(*value) != *value {
                        errors.push(syn::Error::new(
                            ident.span(),
                            format!("unified proto enums disagree on the value of `{proto_name}`"),
                        ));
                    }
                }
                None => {
                    let mut available: Vec<String> = meta
                        .values
                        .iter()
                        .map(|(value_name, _)| variant_name(simple, value_name))
                        .collect();
                    available.sort_unstable();
                    errors.push(syn::Error::new(
                        ident.span(),
                        format!(
                            "no value named `{proto_name}` in proto enum `{enum_name}` \
                             (available: {}); use #[armonik(rename = \"...\")] with the full \
                             proto value name if needed",
                            available.join(", ")
                        ),
                    ));
                }
            }
        }
        if let Some(number) = number {
            if number == 0 {
                zero_variant = Some(ident.clone());
            }
            resolved.push((ident.clone(), number));
        }
    }

    // Completeness: every proto value needs a named variant, except a
    // conventional `*_UNSPECIFIED = 0`, which the catch-all covers.
    for (enum_name, meta) in &proto_enums {
        let simple = enum_name.rsplit('.').next().unwrap_or(enum_name);
        for (value_name, value) in &meta.values {
            let mapped = variant_name(simple, value_name);
            let covered = named
                .iter()
                .any(|(_, proto_name)| *proto_name == mapped || proto_name == value_name);
            if !(covered || (*value == 0 && mapped == "Unspecified")) {
                errors.push(syn::Error::new(
                    input.ident.span(),
                    format!(
                        "proto enum value `{enum_name}.{value_name}` (= {value}) is not \
                         covered by any Rust variant"
                    ),
                ));
            }
        }
    }

    errors.into_result()?;

    Ok(EnumPlan {
        ident: input.ident.clone(),
        other_variant,
        payload,
        named: resolved,
        zero_variant,
        has_std_default,
        mode,
        fingerprint: index.fingerprint,
    })
}

/// prost-style variant name for a proto enum value: upper-camel the value
/// name, then strip the enum-name prefix when present.
fn variant_name(enum_simple_name: &str, value_name: &str) -> String {
    let camel = upper_camel(value_name);
    match camel.strip_prefix(enum_simple_name) {
        Some(stripped)
            if !stripped.is_empty() && !stripped.starts_with(|c: char| c.is_ascii_digit()) =>
        {
            stripped.to_owned()
        }
        _ => camel,
    }
}

fn upper_camel(screaming_snake: &str) -> String {
    screaming_snake
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let first = chars.next().map(|c| c.to_ascii_uppercase());
            first
                .into_iter()
                .chain(chars.map(|c| c.to_ascii_lowercase()))
                .collect::<String>()
        })
        .collect()
}

/// Plan for a oneof-shaped enum: either a whole message whose fields are a
/// single oneof plus optional sibling fields (`message = ...` alone), or
/// just the oneof `oneof_name` of the message, to be embedded in a struct
/// (`message = ...` + `oneof = ...`).
pub(crate) struct OneofPlan {
    pub(crate) ident: syn::Ident,
    pub(crate) proto_name: String,
    /// All member tags, for routing by containers and the whole-message
    /// implementation.
    pub(crate) tags: Vec<u32>,
    /// Whether the enum stands for the whole message (annotation without
    /// `oneof = ...`), in which case it gets `prost::Message` +
    /// `ProtoField` implementations.
    pub(crate) whole_message: bool,
    /// Non-oneof fields of the message, replicated in every variant
    /// (whole-message enums only; empty when the oneof is the only field).
    pub(crate) siblings: Vec<SiblingPlan>,
    pub(crate) variants: Vec<OneofVariant>,
    /// The attribute-less variant standing for "no member set", if any: a
    /// unit variant, or a struct variant carrying exactly the sibling
    /// fields when there are siblings.
    pub(crate) default_variant: Option<syn::Ident>,
    pub(crate) fingerprint: u128,
}

/// A non-oneof field of a whole-message enum, present in every variant
/// under the same name and type.
pub(crate) struct SiblingPlan {
    pub(crate) ident: syn::Ident,
    pub(crate) ty: syn::Type,
    pub(crate) span: Span,
    pub(crate) tag: u32,
    pub(crate) proto_path: String,
    pub(crate) checks: FieldChecks,
}

pub(crate) struct OneofVariant {
    pub(crate) ident: syn::Ident,
    pub(crate) span: Span,
    pub(crate) tag: u32,
    pub(crate) proto_path: String,
    pub(crate) shape: OneofVariantShape,
}

pub(crate) enum OneofVariantShape {
    /// `Variant(T)` where `T: ProtoField` carries the member value.
    Payload {
        ty: Box<syn::Type>,
        checks: Box<FieldChecks>,
    },
    /// `Variant(T)` encoded through a `ProtoAdapter`
    /// (`#[armonik(with = "...")]`); skips kind checks by design.
    Adapter {
        ty: Box<syn::Type>,
        adapter: Box<syn::Type>,
    },
    /// `Variant { payload, ...siblings }` in a whole-message enum with
    /// sibling fields: one member payload plus every non-oneof field.
    SiblingPayload {
        payload: syn::Ident,
        ty: Box<syn::Type>,
        /// `#[armonik(with = "...")]` on the payload field.
        adapter: Option<Box<syn::Type>>,
        checks: Box<FieldChecks>,
    },
    /// `#[armonik(present)]` unit variant selected by a `bool` member.
    MarkerBool,
    /// `#[armonik(present)]` unit variant selected by an empty-message member.
    MarkerMessage,
    /// `Variant { field, ... }` inlining the fields of the member's message.
    Inline { parts: Vec<InlinePart> },
}

pub(crate) struct InlinePart {
    pub(crate) ident: syn::Ident,
    pub(crate) ty: syn::Type,
    pub(crate) span: Span,
    pub(crate) tag: u32,
    pub(crate) proto_path: String,
    pub(crate) checks: FieldChecks,
}

/// Partition the named fields of a variant into the message's sibling
/// fields (updating/checking the cross-variant bindings) and at most one
/// remaining field, the member payload. Returns `Ok(None)` when every field
/// is a sibling (the "no member set" shape), `Err(())` after pushing errors.
#[allow(clippy::type_complexity)]
fn sibling_variant_fields(
    named: &syn::FieldsNamed,
    sibling_metas: &[&FieldMeta],
    sibling_bindings: &mut [Option<(syn::Ident, syn::Type)>],
    errors: &mut Errors,
    variant_span: Span,
    proto_name: &str,
) -> Result<Option<(syn::Ident, syn::Type, Option<syn::Type>)>, ()> {
    let mut failed = false;
    let mut seen = vec![false; sibling_metas.len()];
    let mut payload: Option<(syn::Ident, syn::Type, Option<syn::Type>)> = None;
    for field in &named.named {
        let ident = field.ident.clone().expect("named fields have idents");
        let field_entries = match attrs::parse(&field.attrs) {
            Ok(entries) => entries,
            Err(err) => {
                errors.push(err);
                failed = true;
                continue;
            }
        };
        let mut rename = None;
        let mut with = None;
        for entry in &field_entries {
            match &entry.item {
                AttrItem::Rename(lit) => rename = Some(lit.value()),
                AttrItem::With(lit) => match syn::parse_str::<syn::Type>(&lit.value()) {
                    Ok(ty) => with = Some((entry.span, ty)),
                    Err(err) => {
                        errors.push(syn::Error::new(
                            entry.span,
                            format!("invalid adapter type in with = ...: {err}"),
                        ));
                        failed = true;
                    }
                },
                _ => {
                    errors.push(syn::Error::new(
                        entry.span,
                        "this armonik attribute is not valid on a variant field",
                    ));
                    failed = true;
                }
            }
        }

        let name = rename.unwrap_or_else(|| unraw(&ident));
        if let Some(position) = sibling_metas.iter().position(|meta| meta.name == name) {
            if let Some((with_span, _)) = with {
                errors.push(syn::Error::new(
                    with_span,
                    "with = ... is only valid on the member payload field, not on a \
                     sibling field",
                ));
                failed = true;
            }
            seen[position] = true;
            match &sibling_bindings[position] {
                None => sibling_bindings[position] = Some((ident, field.ty.clone())),
                Some((bound_ident, bound_ty)) => {
                    if *bound_ident != ident {
                        errors.push(syn::Error::new(
                            ident.span(),
                            format!(
                                "sibling field `{name}` must use the same name in every \
                                 variant (`{bound_ident}` elsewhere)"
                            ),
                        ));
                        failed = true;
                    }
                    if quote::quote!(#bound_ty).to_string() != {
                        let ty = &field.ty;
                        quote::quote!(#ty).to_string()
                    } {
                        errors.push(syn::Error::new(
                            field.ty.span(),
                            format!(
                                "sibling field `{name}` must use the same type in every \
                                 variant"
                            ),
                        ));
                        failed = true;
                    }
                }
            }
        } else if payload.is_some() {
            errors.push(syn::Error::new(
                ident.span(),
                format!(
                    "only one field of the variant may be the member payload; the others \
                     must match the non-oneof fields of `{proto_name}` (use \
                     #[armonik(rename = \"...\")] if the names differ)"
                ),
            ));
            failed = true;
        } else {
            payload = Some((ident, field.ty.clone(), with.map(|(_, ty)| ty)));
        }
    }
    for (position, field_seen) in seen.iter().enumerate() {
        if !field_seen {
            errors.push(syn::Error::new(
                variant_span,
                format!(
                    "the variant must carry the sibling field `{}` of `{proto_name}` \
                     (every variant of a whole-message enum declares all non-oneof \
                     fields)",
                    sibling_metas[position].name
                ),
            ));
            failed = true;
        }
    }
    if failed {
        Err(())
    } else {
        Ok(payload)
    }
}

pub(crate) fn oneof_plan(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
) -> Result<OneofPlan, Errors> {
    let mut errors = Errors::new();

    let entries = match attrs::parse(&input.attrs) {
        Ok(entries) => entries,
        Err(err) => return Err(Errors::from(err)),
    };

    let mut proto_name: Option<(Span, String)> = None;
    let mut oneof_name: Option<(Span, String)> = None;
    for entry in &entries {
        match &entry.item {
            AttrItem::Message(lit) => {
                if proto_name.replace((entry.span, lit.value())).is_some() {
                    errors.push(syn::Error::new(
                        entry.span,
                        "flattened oneofs support a single proto message",
                    ));
                }
            }
            AttrItem::Oneof(lit) => oneof_name = Some((entry.span, lit.value())),
            _ => errors.push(syn::Error::new(
                entry.span,
                "this armonik attribute is not valid at type level on a flattened oneof",
            )),
        }
    }
    let Some((message_span, proto_name)) = proto_name else {
        errors.push(syn::Error::new(
            input.ident.span(),
            "oneof-shaped enums need #[armonik(message = \"...\")]",
        ));
        return Err(errors);
    };

    let Some(meta) = index.messages.get(&proto_name) else {
        errors.push(syn::Error::new(
            message_span,
            format!("proto message `{proto_name}` not found in the compiled descriptor set"),
        ));
        return Err(errors);
    };
    // `message = ...` alone: the enum stands for the whole message, whose
    // single oneof is inferred and whose non-oneof fields become siblings
    // replicated in every variant. `oneof = ...` declares a partial enum
    // embedded in a struct, and is rejected when the oneof is the whole
    // message so the two shapes stay visually distinct.
    let (oneof, whole_message) = match &oneof_name {
        Some((oneof_span, oneof_name)) => {
            let Some((oneof_index, oneof)) = meta.oneof(oneof_name) else {
                errors.push(syn::Error::new(
                    *oneof_span,
                    format!("no oneof named `{oneof_name}` in proto message `{proto_name}`"),
                ));
                return Err(errors);
            };
            if meta
                .fields
                .iter()
                .all(|field| field.oneof == Some(oneof_index))
            {
                errors.push(syn::Error::new(
                    *oneof_span,
                    format!(
                        "the oneof `{oneof_name}` covers the whole message `{proto_name}`; \
                         drop the oneof attribute: #[armonik(message = ...)] alone declares \
                         a whole-message enum"
                    ),
                ));
                return Err(errors);
            }
            (oneof, false)
        }
        None => match meta.oneofs.len() {
            1 => (&meta.oneofs[0], true),
            0 => {
                errors.push(syn::Error::new(
                    input.ident.span(),
                    format!(
                        "proto message `{proto_name}` has no oneof; a message without a \
                         oneof is derived on a struct"
                    ),
                ));
                return Err(errors);
            }
            n => {
                errors.push(syn::Error::new(
                    input.ident.span(),
                    format!(
                        "proto message `{proto_name}` has {n} oneofs; an enum can stand \
                         for the whole message only when there is exactly one — declare \
                         one enum per oneof with #[armonik(oneof = \"...\")] and compose \
                         them in a struct"
                    ),
                ));
                return Err(errors);
            }
        },
    };
    let tags: Vec<u32> = oneof
        .fields
        .iter()
        .map(|&field| meta.fields[field].tag)
        .collect();

    // Non-oneof fields of a whole-message enum, replicated in every variant.
    let sibling_metas: Vec<&FieldMeta> = if whole_message {
        meta.fields
            .iter()
            .filter(|field| field.oneof.is_none())
            .collect()
    } else {
        Vec::new()
    };
    // Rust-side binding of each sibling (ident + type), fixed by the first
    // variant that declares it and checked for consistency in the others.
    let mut sibling_bindings: Vec<Option<(syn::Ident, syn::Type)>> =
        (0..sibling_metas.len()).map(|_| None).collect();

    let syn::Data::Enum(data) = &input.data else {
        errors.push(syn::Error::new(
            input.ident.span(),
            "#[armonik(oneof = ...)] expects an enum",
        ));
        return Err(errors);
    };

    let mut variants = Vec::new();
    let mut default_variant: Option<syn::Ident> = None;
    let mut covered = vec![false; oneof.fields.len()];
    for variant in &data.variants {
        let span = variant.ident.span();
        let variant_entries = match attrs::parse(&variant.attrs) {
            Ok(entries) => entries,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let mut rename = None;
        let mut present = false;
        let mut with = None;
        for entry in &variant_entries {
            match &entry.item {
                AttrItem::Rename(lit) => rename = Some(lit.value()),
                AttrItem::Present => present = true,
                AttrItem::With(lit) => match syn::parse_str::<syn::Type>(&lit.value()) {
                    Ok(ty) => with = Some((entry.span, ty)),
                    Err(err) => errors.push(syn::Error::new(
                        entry.span,
                        format!("invalid adapter type in with = ...: {err}"),
                    )),
                },
                _ => errors.push(syn::Error::new(
                    entry.span,
                    "this armonik attribute is not valid on a oneof variant",
                )),
            }
        }

        // The attribute-less unit variant is "no member set"; with sibling
        // fields, that case is a struct variant carrying exactly them and is
        // detected below, after member-name matching fails.
        if matches!(variant.fields, syn::Fields::Unit)
            && !present
            && rename.is_none()
            && sibling_metas.is_empty()
        {
            if default_variant.replace(variant.ident.clone()).is_some() {
                errors.push(syn::Error::new(
                    span,
                    "at most one attribute-less unit variant (the \"no member set\" case) \
                     is allowed",
                ));
            }
            continue;
        }

        let member_name = rename
            .clone()
            .unwrap_or_else(|| snake_case(&unraw(&variant.ident)));
        let member = oneof
            .fields
            .iter()
            .enumerate()
            .find_map(|(position, &field)| {
                (meta.fields[field].name == member_name).then_some((position, &meta.fields[field]))
            });
        if member.is_none() && !sibling_metas.is_empty() && !present && rename.is_none() {
            if let syn::Fields::Named(named) = &variant.fields {
                match sibling_variant_fields(
                    named,
                    &sibling_metas,
                    &mut sibling_bindings,
                    &mut errors,
                    span,
                    &proto_name,
                ) {
                    // All fields are siblings: the "no member set" variant.
                    Ok(None) => {
                        if default_variant.replace(variant.ident.clone()).is_some() {
                            errors.push(syn::Error::new(
                                span,
                                "at most one attribute-less variant (the \"no member \
                                 set\" case) is allowed",
                            ));
                        }
                        continue;
                    }
                    // A payload is present but the name matches no member:
                    // fall through to the member error below.
                    Ok(Some(_)) => {}
                    Err(()) => continue,
                }
            }
        }
        let Some((position, field_meta)) = member else {
            let mut available: Vec<&str> = oneof
                .fields
                .iter()
                .map(|&field| meta.fields[field].name.as_str())
                .collect();
            available.sort_unstable();
            errors.push(syn::Error::new(
                span,
                format!(
                    "no member named `{member_name}` in oneof `{proto_name}.{}` \
                     (available: {}); use #[armonik(rename = \"...\")] if the names differ",
                    oneof.name,
                    available.join(", ")
                ),
            ));
            continue;
        };
        covered[position] = true;
        let proto_path = format!("{proto_name}.{}", field_meta.name);

        let shape = if !sibling_metas.is_empty() {
            if present {
                errors.push(syn::Error::new(
                    span,
                    "#[armonik(present)] markers are not supported in whole-message \
                     enums with sibling fields",
                ));
                continue;
            }
            if let Some((with_span, _)) = &with {
                errors.push(syn::Error::new(
                    *with_span,
                    "in whole-message enums with sibling fields, put with = ... on the \
                     member payload field",
                ));
                continue;
            }
            let syn::Fields::Named(named) = &variant.fields else {
                errors.push(syn::Error::new(
                    span,
                    "variants of a whole-message enum with sibling fields must be \
                     struct variants carrying the sibling fields",
                ));
                continue;
            };
            let (payload, ty, adapter) = match sibling_variant_fields(
                named,
                &sibling_metas,
                &mut sibling_bindings,
                &mut errors,
                span,
                &proto_name,
            ) {
                Ok(Some(payload)) => payload,
                Ok(None) => {
                    errors.push(syn::Error::new(
                        span,
                        format!(
                            "the variant needs a payload field for the member \
                             `{member_name}`"
                        ),
                    ));
                    continue;
                }
                Err(()) => continue,
            };
            let checks = if adapter.is_some() {
                FieldChecks::none()
            } else {
                expected_checks(field_meta)
            };
            OneofVariantShape::SiblingPayload {
                payload,
                ty: Box::new(ty),
                adapter: adapter.map(Box::new),
                checks: Box::new(checks),
            }
        } else if let Some((with_span, adapter)) = with {
            if present {
                errors.push(syn::Error::new(
                    with_span,
                    "with = ... and present cannot be combined on a oneof variant",
                ));
                continue;
            }
            match &variant.fields {
                syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    OneofVariantShape::Adapter {
                        ty: Box::new(fields.unnamed[0].ty.clone()),
                        adapter: Box::new(adapter),
                    }
                }
                _ => {
                    errors.push(syn::Error::new(
                        with_span,
                        "with = ... needs a single-payload tuple variant",
                    ));
                    continue;
                }
            }
        } else if present {
            if !matches!(variant.fields, syn::Fields::Unit) {
                errors.push(syn::Error::new(
                    span,
                    "#[armonik(present)] variants must be unit variants",
                ));
                continue;
            }
            match &field_meta.kind {
                FieldKind::Bool => OneofVariantShape::MarkerBool,
                FieldKind::Message(_) => OneofVariantShape::MarkerMessage,
                other => {
                    errors.push(syn::Error::new(
                        span,
                        format!(
                            "#[armonik(present)] needs a bool or message member, but \
                             `{proto_path}` is {other:?}"
                        ),
                    ));
                    continue;
                }
            }
        } else {
            match &variant.fields {
                syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    OneofVariantShape::Payload {
                        ty: Box::new(fields.unnamed[0].ty.clone()),
                        checks: Box::new(expected_checks(field_meta)),
                    }
                }
                syn::Fields::Named(named) => {
                    let FieldKind::Message(inner_name) = &field_meta.kind else {
                        errors.push(syn::Error::new(
                            span,
                            format!(
                                "struct variants inline a message member, but `{proto_path}` \
                                 is not a message"
                            ),
                        ));
                        continue;
                    };
                    let Some(inner) = index.messages.get(inner_name) else {
                        errors.push(syn::Error::new(
                            span,
                            format!("proto message `{inner_name}` not found"),
                        ));
                        continue;
                    };
                    if !inner.oneofs.is_empty() {
                        errors.push(syn::Error::new(
                            span,
                            format!(
                                "`{inner_name}` contains a oneof; it cannot be inlined into \
                                 a struct variant"
                            ),
                        ));
                        continue;
                    }
                    let mut parts = Vec::new();
                    let mut inner_covered = vec![false; inner.fields.len()];
                    for part in &named.named {
                        let part_ident = part.ident.clone().expect("named fields have idents");
                        let part_entries = match attrs::parse(&part.attrs) {
                            Ok(entries) => entries,
                            Err(err) => {
                                errors.push(err);
                                continue;
                            }
                        };
                        let mut part_rename = None;
                        for entry in &part_entries {
                            match &entry.item {
                                AttrItem::Rename(lit) => part_rename = Some(lit.value()),
                                _ => errors.push(syn::Error::new(
                                    entry.span,
                                    "this armonik attribute is not valid on a struct \
                                     variant field",
                                )),
                            }
                        }
                        let part_name = part_rename.unwrap_or_else(|| unraw(&part_ident));
                        let Some(part_position) = inner
                            .fields
                            .iter()
                            .position(|field| field.name == part_name)
                        else {
                            errors.push(syn::Error::new(
                                part_ident.span(),
                                format!(
                                    "no field named `{part_name}` in proto message \
                                     `{inner_name}`"
                                ),
                            ));
                            continue;
                        };
                        inner_covered[part_position] = true;
                        let part_meta = &inner.fields[part_position];
                        parts.push(InlinePart {
                            span: part_ident.span(),
                            ident: part_ident,
                            ty: part.ty.clone(),
                            tag: part_meta.tag,
                            proto_path: format!("{inner_name}.{}", part_meta.name),
                            checks: expected_checks(part_meta),
                        });
                    }
                    for (position, part_covered) in inner_covered.iter().enumerate() {
                        if !part_covered {
                            errors.push(syn::Error::new(
                                span,
                                format!(
                                    "proto field `{inner_name}.{}` (tag {}) is not covered \
                                     by the struct variant",
                                    inner.fields[position].name, inner.fields[position].tag
                                ),
                            ));
                        }
                    }
                    parts.sort_by_key(|part| part.tag);
                    OneofVariantShape::Inline { parts }
                }
                _ => {
                    errors.push(syn::Error::new(
                        span,
                        "oneof variants must be `Variant(T)`, `Variant { .. }`, a \
                         #[armonik(present)] marker, or the attribute-less default",
                    ));
                    continue;
                }
            }
        };

        variants.push(OneofVariant {
            ident: variant.ident.clone(),
            span,
            tag: field_meta.tag,
            proto_path,
            shape,
        });
    }

    for (position, member_covered) in covered.iter().enumerate() {
        if !member_covered {
            let field = &meta.fields[oneof.fields[position]];
            errors.push(syn::Error::new(
                input.ident.span(),
                format!(
                    "oneof member `{proto_name}.{}` (tag {}) is not covered by any variant",
                    field.name, field.tag
                ),
            ));
        }
    }

    errors.into_result()?;

    let mut siblings = Vec::new();
    for (meta_field, binding) in sibling_metas.iter().zip(&sibling_bindings) {
        // Missing bindings are only possible when every variant errored;
        // those errors were reported above.
        let Some((ident, ty)) = binding else { continue };
        siblings.push(SiblingPlan {
            span: ident.span(),
            ident: ident.clone(),
            ty: ty.clone(),
            tag: meta_field.tag,
            proto_path: format!("{proto_name}.{}", meta_field.name),
            checks: expected_checks(meta_field),
        });
    }
    siblings.sort_by_key(|sibling| sibling.tag);

    variants.sort_by_key(|variant| variant.tag);
    Ok(OneofPlan {
        ident: input.ident.clone(),
        proto_name,
        tags,
        whole_message,
        siblings,
        variants,
        default_variant,
        fingerprint: index.fingerprint,
    })
}

fn snake_case(camel: &str) -> String {
    let mut out = String::with_capacity(camel.len() + 4);
    for (i, c) in camel.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}
