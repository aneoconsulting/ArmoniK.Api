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
    pub(crate) style: StructStyle,
    pub(crate) fingerprint: u128,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StructStyle {
    Named,
    Tuple,
    Unit,
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
    for entry in &entries {
        match &entry.item {
            AttrItem::Message(lit) => proto_names.push((entry.span, lit.value())),
            AttrItem::Generic | AttrItem::Oneof(_) | AttrItem::Transparent => {
                errors.push(syn::Error::new(
                    entry.span,
                    "this armonik attribute mode is not implemented yet",
                ));
            }
            _ => errors.push(syn::Error::new(
                entry.span,
                "this armonik attribute is not valid at type level on a struct",
            )),
        }
    }
    if proto_names.is_empty() {
        errors.push(syn::Error::new(
            input.ident.span(),
            "missing #[armonik(message = \"full.proto.Name\")]",
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
        style: match &data.fields {
            syn::Fields::Named(_) => StructStyle::Named,
            syn::Fields::Unnamed(_) => StructStyle::Tuple,
            syn::Fields::Unit => StructStyle::Unit,
        },
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
    pub(crate) mode: EnumMode,
    pub(crate) fingerprint: u128,
}

pub(crate) enum EnumMode {
    /// The Rust enum is a proto enum, an `int32` varint on the wire.
    Plain { names: Vec<String> },
    /// The Rust enum stands for proto message(s) wrapping a single enum
    /// field at `inner_tag`.
    Transparent { names: Vec<String>, inner_tag: u32 },
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
        let mut inner_tag: Option<u32> = None;
        for (span, name) in &message_names {
            let Some(meta) = index.messages.get(name) else {
                errors.push(syn::Error::new(
                    *span,
                    format!("proto message `{name}` not found in the compiled descriptor set"),
                ));
                continue;
            };
            let [field] = meta.fields.as_slice() else {
                errors.push(syn::Error::new(
                    *span,
                    format!("`{name}` is not a single-field wrapper message"),
                ));
                continue;
            };
            let FieldKind::Enum(inner) = &field.kind else {
                errors.push(syn::Error::new(
                    *span,
                    format!("the single field of `{name}` is not an enum"),
                ));
                continue;
            };
            if *inner_tag.get_or_insert(field.tag) != field.tag {
                errors.push(syn::Error::new(
                    *span,
                    "transparent wrapper messages disagree on the inner field tag",
                ));
            }
            match index.enums.get(inner) {
                Some(enum_meta) => proto_enums.push((inner.clone(), enum_meta)),
                None => errors.push(syn::Error::new(
                    *span,
                    format!("proto enum `{inner}` not found in the compiled descriptor set"),
                )),
            }
        }
        let Some(inner_tag) = inner_tag else {
            return Err(errors);
        };
        EnumMode::Transparent {
            names: message_names.iter().map(|(_, name)| name.clone()).collect(),
            inner_tag,
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
    for variant in &data.variants {
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
