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
