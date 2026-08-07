//! Resolution of a derived type against the protobuf descriptor: field
//! matching, tag/kind/cardinality extraction, and all the validation that
//! can be done at expansion time. The output is a plan consumed by
//! [`crate::codegen`].

use proc_macro2::Span;
use syn::spanned::Spanned;

use crate::attrs::{self, AttrItem, Errors};
use crate::descriptor::{Cardinality, DescriptorIndex, FieldKind, FieldMeta, MessageMeta};

/// Plan for a plain (non-oneof) message struct.
pub(crate) struct MessagePlan {
    pub(crate) ident: syn::Ident,
    /// Full proto names the type stands for (several for unified types).
    pub(crate) proto_names: Vec<String>,
    /// Fields sorted by tag (canonical encode order). In `transparent` mode
    /// this holds exactly the single delegate field.
    pub(crate) fields: Vec<FieldPlan>,
    pub(crate) generics: syn::Generics,
    pub(crate) fingerprint: u64,
    /// `#[armonik(transparent)]` on a struct: the type delegates its whole
    /// `prost::Message` impl to its single field.
    pub(crate) transparent: bool,
}

pub(crate) enum FieldAccess {
    Named(syn::Ident),
    Indexed(syn::Index),
}

pub(crate) enum FieldCodec {
    /// An ordinary field; `adapter` is the `#[armonik(with = "...")]` type
    /// when present (which skips the shape checks by design).
    Field { adapter: Option<Box<syn::Type>> },
    /// The field covers a whole oneof of the message and is encoded through
    /// `ProtoOneof`; `tags` are the member field tags routed to it.
    OneofGroup { tags: Vec<u32> },
}

/// Fieldless mirror of the codec-side `Cardinality`, tokenized into the
/// emitted shape asserts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Card {
    Singular,
    Optional,
    Repeated,
    Map,
}

/// Compile-time checks emitted alongside the implementation, mirroring the
/// codec-side `Expect` (one shape assert per checked field).
pub(crate) struct FieldChecks {
    /// `None` for map fields (their kinds live in `map_kinds`).
    pub(crate) kind: Option<FieldKind>,
    /// Acceptable runtime cardinalities (e.g. a singular message field may
    /// be either `Singular` or `Optional` in Rust).
    pub(crate) cardinalities: Vec<Card>,
    /// Expected proto type name for message/enum (element) kinds.
    pub(crate) name: Option<String>,
    /// Expected map key/value kinds.
    pub(crate) map_kinds: Option<(FieldKind, FieldKind)>,
}

impl FieldChecks {
    fn none() -> Self {
        Self {
            kind: None,
            cardinalities: Vec::new(),
            name: None,
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

/// Field-or-oneof lookup with coverage over one proto message: resolves
/// names, records what was consumed, reports misses with the sorted
/// "available:" list, and turns leftovers into completeness errors. One per
/// unified message in [`message_plan`]; also drives inline struct variants.
struct Matcher<'a> {
    message_name: &'a str,
    meta: &'a MessageMeta,
    consumed: Vec<bool>,
    consumed_oneofs: Vec<bool>,
}

enum Found<'a> {
    Field(&'a FieldMeta),
    Oneof { tags: Vec<u32> },
}

impl<'a> Matcher<'a> {
    fn new(message_name: &'a str, meta: &'a MessageMeta) -> Self {
        Self {
            message_name,
            meta,
            consumed: vec![false; meta.fields.len()],
            consumed_oneofs: vec![false; meta.oneofs.len()],
        }
    }

    /// Look `proto_name` up among the message's fields and oneofs, marking
    /// it consumed. `None` (with a spanned error) when nothing matches or
    /// the field can only be mapped through its oneof.
    fn find(&mut self, proto_name: &str, span: Span, errors: &mut Errors) -> Option<Found<'a>> {
        if let Some(position) = self
            .meta
            .fields
            .iter()
            .position(|field| field.name == proto_name)
        {
            self.consumed[position] = true;
            let field = &self.meta.fields[position];
            if field.oneof.is_some() {
                errors.push(syn::Error::new(
                    span,
                    format!(
                        "proto field `{}.{proto_name}` belongs to a oneof; \
                         map the whole oneof to one field named after it",
                        self.message_name
                    ),
                ));
                return None;
            }
            return Some(Found::Field(field));
        }
        if let Some((index, oneof)) = self.meta.oneof(proto_name) {
            self.consumed_oneofs[index] = true;
            let tags = oneof
                .fields
                .iter()
                .map(|&field| self.meta.fields[field].tag)
                .collect();
            return Some(Found::Oneof { tags });
        }
        let available = self
            .meta
            .fields
            .iter()
            .map(|field| field.name.clone())
            .chain(self.meta.oneofs.iter().map(|oneof| oneof.name.clone()))
            .collect();
        errors.push(unknown_name(
            span,
            "field or oneof",
            proto_name,
            &format!("proto message `{}`", self.message_name),
            available,
            "use #[armonik(rename = \"...\")] if the names differ",
        ));
        None
    }

    /// Completeness: every uncovered proto field and oneof is an error at
    /// `at`. A field is covered through its oneof when the oneof was mapped
    /// whole; a oneof is covered when every member was mapped individually.
    fn check_complete(&self, at: Span, errors: &mut Errors) {
        for (position, field) in self.meta.fields.iter().enumerate() {
            let in_oneof_group = field.oneof.is_some_and(|oneof| self.consumed_oneofs[oneof]);
            if !self.consumed[position] && !in_oneof_group {
                errors.push(syn::Error::new(
                    at,
                    format!(
                        "proto field `{}.{}` (tag {}) is not covered by any Rust field",
                        self.message_name, field.name, field.tag
                    ),
                ));
            }
        }
        for (index, oneof) in self.meta.oneofs.iter().enumerate() {
            let members_covered = oneof.fields.iter().all(|&field| self.consumed[field]);
            if !self.consumed_oneofs[index] && !members_covered {
                errors.push(syn::Error::new(
                    at,
                    format!(
                        "proto oneof `{}.{}` is not covered by any Rust field",
                        self.message_name, oneof.name
                    ),
                ));
            }
        }
    }
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
    let mut transparent = false;
    for entry in &entries {
        match &entry.item {
            AttrItem::Message(lit) => proto_names.push((entry.span, lit.value())),
            AttrItem::Generic => generic = true,
            AttrItem::Transparent => transparent = true,
            AttrItem::Oneof(_) => {
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
    if transparent {
        return transparent_plan(input, index, proto_names, errors);
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
            None => errors.push(not_found(*span, "message", name)),
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
    let mut matchers: Vec<Matcher> = messages
        .iter()
        .map(|(name, meta)| Matcher::new(name, meta))
        .collect();

    for (field_index, field) in data.fields.iter().enumerate() {
        let (span, access) = field_access(field, field_index);
        let Some((
            FieldAttrs {
                rename,
                tag: explicit_tag,
                with,
                ..
            },
            _,
        )) = scan_attrs(
            &field.attrs,
            Allowed {
                rename: true,
                tag: true,
                with: true,
                absorbs: true,
                ..Allowed::default()
            },
            "this armonik attribute is not valid on a message field",
            &mut errors,
        )
        else {
            continue;
        };
        let with = with.map(|(_, ty)| ty);

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
        let mut resolved: Option<Found> = None;
        let mut failed = false;
        for matcher in &mut matchers {
            let Some(found) = matcher.find(&proto_name, span, &mut errors) else {
                failed = true;
                continue;
            };
            match &resolved {
                None => resolved = Some(found),
                Some(previous) => {
                    let agree = match (previous, &found) {
                        (Found::Field(a), Found::Field(b)) => {
                            a.tag == b.tag && a.kind == b.kind && a.cardinality == b.cardinality
                        }
                        (Found::Oneof { tags: a }, Found::Oneof { tags: b }) => a == b,
                        _ => false,
                    };
                    if !agree {
                        errors.push(syn::Error::new(
                            span,
                            format!(
                                "unified messages disagree on `{proto_name}` \
                                 (tag/kind/cardinality differ); it cannot be derived"
                            ),
                        ));
                        failed = true;
                    }
                }
            }
        }
        let Some(resolved) = resolved else { continue };
        if failed {
            continue;
        }

        let proto_path = format!("{}.{proto_name}", messages[0].0);

        let (tag, checks) = match resolved {
            Found::Oneof { tags } => {
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
            Found::Field(field_meta) => {
                if let Some((tag_span, tag_value)) = explicit_tag {
                    if tag_value != field_meta.tag {
                        errors.push(syn::Error::new(
                            tag_span,
                            format!(
                                "tag {tag_value} does not match proto field \
                                 `{proto_path}` (= {})",
                                field_meta.tag
                            ),
                        ));
                        continue;
                    }
                }
                let checks = match &with {
                    Some(_) => FieldChecks::none(),
                    None => expected_checks(field_meta),
                };
                (field_meta.tag, checks)
            }
        };
        fields.push(FieldPlan {
            access,
            ty: field.ty.clone(),
            span,
            tag,
            codec: FieldCodec::Field {
                adapter: with.map(Box::new),
            },
            checks,
            proto_path,
        });
    }

    // Completeness: every proto field and oneof of every unified message
    // must be covered by a Rust field.
    for matcher in &matchers {
        matcher.check_complete(input.ident.span(), &mut errors);
    }

    errors.into_result()?;

    fields.sort_by_key(|field| field.tag);
    Ok(MessagePlan {
        ident: input.ident.clone(),
        proto_names: proto_names.into_iter().map(|(_, name)| name).collect(),
        fields,
        generics: input.generics.clone(),
        fingerprint: index.fingerprint,
        transparent: false,
    })
}

/// Plan for a `#[armonik(transparent)]` struct: a single-field newtype that
/// delegates its whole `prost::Message` impl to the field, so it is
/// wire-identical to the inner message. The field is not matched against the
/// descriptor (the inner type already validates itself); only the named proto
/// message is checked to exist, and it is used for registration.
fn transparent_plan(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
    proto_names: Vec<(Span, String)>,
    mut errors: Errors,
) -> Result<MessagePlan, Errors> {
    if !input.generics.params.is_empty() {
        errors.push(syn::Error::new(
            input.ident.span(),
            "#[armonik(transparent)] structs cannot be generic",
        ));
    }
    if proto_names.len() != 1 {
        errors.push(syn::Error::new(
            input.ident.span(),
            "#[armonik(transparent)] structs need exactly one \
             #[armonik(message = \"full.proto.Name\")]",
        ));
    }
    for (span, name) in &proto_names {
        if !index.messages.contains_key(name) {
            errors.push(not_found(*span, "message", name));
        }
    }
    let syn::Data::Struct(data) = &input.data else {
        errors.push(syn::Error::new(
            input.ident.span(),
            "#[armonik(transparent)] expects a struct",
        ));
        return Err(errors);
    };
    if data.fields.len() != 1 {
        errors.push(syn::Error::new(
            input.ident.span(),
            "#[armonik(transparent)] structs must have exactly one field, delegated to",
        ));
        return Err(errors);
    }
    let field = data.fields.iter().next().expect("one field");
    let (_, access) = field_access(field, 0);
    let delegate = FieldPlan {
        access,
        ty: field.ty.clone(),
        span: field.ty.span(),
        tag: 0,
        codec: FieldCodec::Field { adapter: None },
        checks: FieldChecks::none(),
        proto_path: String::new(),
    };

    errors.into_result()?;

    Ok(MessagePlan {
        ident: input.ident.clone(),
        proto_names: proto_names.into_iter().map(|(_, name)| name).collect(),
        fields: vec![delegate],
        generics: input.generics.clone(),
        fingerprint: index.fingerprint,
        transparent: true,
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
        let (span, access) = field_access(field, field_index);
        let Some((FieldAttrs { tag, with, .. }, _)) = scan_attrs(
            &field.attrs,
            Allowed {
                tag: true,
                with: true,
                ..Allowed::default()
            },
            "generic-mode fields only take tag = ... and with = ...",
            &mut errors,
        ) else {
            continue;
        };
        let tag = tag.map(|(_, tag)| tag);
        let with = with.map(|(_, ty)| ty);
        let Some(tag) = tag else {
            errors.push(syn::Error::new(
                span,
                "generic-mode fields need an explicit #[armonik(tag = ...)]",
            ));
            continue;
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
            codec: FieldCodec::Field {
                adapter: with.map(Box::new),
            },
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
        transparent: false,
    })
}

/// Compile-time checks for a plain field, derived from the descriptor.
fn expected_checks(field: &FieldMeta) -> FieldChecks {
    let mut checks = FieldChecks::none();
    // The Map arm is the outlier: it leaves `kind` unset, checks the
    // key/value kinds, and names the value type.
    checks.cardinalities = match &field.cardinality {
        Cardinality::Map { key, value } => {
            checks.map_kinds = Some((key.clone(), value.clone()));
            checks.name = type_name(value).map(str::to_owned);
            vec![Card::Map]
        }
        Cardinality::Repeated => vec![Card::Repeated],
        Cardinality::Optional => vec![Card::Optional],
        // Singular message fields may be either plain ("absent = default")
        // or `Option` (presence-significant) in Rust.
        Cardinality::Singular if matches!(field.kind, FieldKind::Message(_)) => {
            vec![Card::Singular, Card::Optional]
        }
        Cardinality::Singular => vec![Card::Singular],
    };
    if checks.map_kinds.is_none() {
        checks.kind = Some(field.kind.clone());
        checks.name = type_name(&field.kind).map(str::to_owned);
    }
    checks
}

fn type_name(kind: &FieldKind) -> Option<&str> {
    match kind {
        FieldKind::Message(name) | FieldKind::Enum(name) => Some(name),
        _ => None,
    }
}

fn unraw(ident: &syn::Ident) -> String {
    ident.to_string().trim_start_matches("r#").to_owned()
}

/// Parse + scan one field/variant's attributes per the site's [`Allowed`]
/// set; `None` (with the error pushed) when the attribute list itself does
/// not parse.
fn scan_attrs(
    attrs: &[syn::Attribute],
    allowed: Allowed,
    reject: &str,
    errors: &mut Errors,
) -> Option<(FieldAttrs, bool)> {
    match attrs::parse(attrs) {
        Ok(entries) => Some(scan_field_attrs(&entries, allowed, reject, errors)),
        Err(err) => {
            errors.push(err);
            None
        }
    }
}

/// Span and access path of a struct field (named, or by position).
fn field_access(field: &syn::Field, index: usize) -> (Span, FieldAccess) {
    let span = field
        .ident
        .as_ref()
        .map(|ident| ident.span())
        .unwrap_or_else(|| field.ty.span());
    let access = match &field.ident {
        Some(ident) => FieldAccess::Named(ident.clone()),
        None => FieldAccess::Indexed(syn::Index::from(index)),
    };
    (span, access)
}

/// "proto message/enum `name` not found in the compiled descriptor set".
fn not_found(span: Span, what: &str, name: &str) -> syn::Error {
    syn::Error::new(
        span,
        format!("proto {what} `{name}` not found in the compiled descriptor set"),
    )
}

/// "no <what> named `name` in <container> (available: ...); <hint>" — the
/// shared shape of every name-lookup miss.
fn unknown_name(
    span: Span,
    what: &str,
    name: &str,
    container: &str,
    mut available: Vec<String>,
    hint: &str,
) -> syn::Error {
    available.sort_unstable();
    syn::Error::new(
        span,
        format!(
            "no {what} named `{name}` in {container} (available: {}); {hint}",
            available.join(", ")
        ),
    )
}

/// Parse the adapter type in `#[armonik(with = "path::To::Adapter")]`,
/// pushing a spanned error (and returning `None`) when it does not parse.
fn parse_adapter_type(lit: &syn::LitStr, span: Span, errors: &mut Errors) -> Option<syn::Type> {
    match syn::parse_str::<syn::Type>(&lit.value()) {
        Ok(ty) => Some(ty),
        Err(err) => {
            errors.push(syn::Error::new(
                span,
                format!("invalid adapter type in with = ...: {err}"),
            ));
            None
        }
    }
}

/// The field/variant-level `#[armonik(...)]` keys collected by
/// [`scan_field_attrs`]. Each site reads only the keys it opted into through
/// [`Allowed`]; the rest stay at their defaults.
#[derive(Default)]
struct FieldAttrs {
    rename: Option<String>,
    tag: Option<(Span, u32)>,
    with: Option<(Span, syn::Type)>,
    present: bool,
}

/// The `#[armonik(...)]` keys a site accepts. Any key not enabled here is a
/// spanned `reject` error, so each site keeps rejecting exactly what it did
/// before — in particular `absorbs`, which is merely *tolerated* (it is
/// harvested separately by [`crate::expand`]) at the sites that enable it and
/// rejected like any stray key everywhere else.
#[derive(Clone, Copy, Default)]
struct Allowed {
    rename: bool,
    tag: bool,
    with: bool,
    present: bool,
    absorbs: bool,
}

/// Scan one field's or variant's `#[armonik(...)]` entries into a
/// [`FieldAttrs`], pushing `reject` (spanned) for any key outside `allowed`
/// and for a malformed `tag`/`with`. Returns the collected attributes and
/// whether every entry was accepted — callers that abandon a malformed field
/// gate on the bool; the rest rely on the pushed errors alone and ignore it.
fn scan_field_attrs(
    entries: &[attrs::AttrEntry],
    allowed: Allowed,
    reject: &str,
    errors: &mut Errors,
) -> (FieldAttrs, bool) {
    let mut collected = FieldAttrs::default();
    let mut ok = true;
    for entry in entries {
        let accepted = match &entry.item {
            AttrItem::Rename(lit) if allowed.rename => {
                collected.rename = Some(lit.value());
                true
            }
            AttrItem::Tag(lit) if allowed.tag => match lit.base10_parse::<u32>() {
                Ok(tag) => {
                    collected.tag = Some((entry.span, tag));
                    true
                }
                Err(err) => {
                    errors.push(syn::Error::new(entry.span, err));
                    false
                }
            },
            AttrItem::With(lit) if allowed.with => {
                match parse_adapter_type(lit, entry.span, errors) {
                    Some(ty) => {
                        collected.with = Some((entry.span, ty));
                        true
                    }
                    None => false,
                }
            }
            AttrItem::Present if allowed.present => {
                collected.present = true;
                true
            }
            // Harvested by `collect_absorbs` in lib.rs; only tolerated here.
            AttrItem::Absorbs(_) if allowed.absorbs => true,
            _ => {
                errors.push(syn::Error::new(entry.span, reject));
                false
            }
        };
        ok &= accepted;
    }
    (collected, ok)
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
    pub(crate) fingerprint: u64,
    /// Intermediate wrapper messages the transparent chain flattens away, so
    /// they have no Rust type of their own (see [`crate::codegen`]).
    pub(crate) absorbs: Vec<String>,
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
    // Intermediate wrapper messages walked through in transparent mode: they
    // have no Rust type, so they are registered as absorbed.
    let mut absorbs: Vec<String> = Vec::new();
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
                    errors.push(not_found(*span, "message", &current));
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
                    FieldKind::Message(inner) => {
                        // A wrapper layer between the root message and the
                        // enum: no Rust type stands for it.
                        absorbs.push(inner.clone());
                        current = inner.clone();
                    }
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
                None => errors.push(not_found(*span, "enum", &enum_name)),
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
                None => errors.push(not_found(*span, "enum", name)),
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
        let Some((FieldAttrs { rename, .. }, _)) = scan_attrs(
            &variant.attrs,
            Allowed {
                rename: true,
                ..Allowed::default()
            },
            "this armonik attribute is not valid on a derive(Enum) variant",
            &mut errors,
        ) else {
            continue;
        };

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
                    let available = meta
                        .values
                        .iter()
                        .map(|(value_name, _)| variant_name(simple, value_name))
                        .collect();
                    errors.push(unknown_name(
                        ident.span(),
                        "value",
                        proto_name,
                        &format!("proto enum `{enum_name}`"),
                        available,
                        "use #[armonik(rename = \"...\")] with the full proto value name if needed",
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
        absorbs,
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
    pub(crate) fingerprint: u64,
    /// Messages inlined into struct variants (their fields are spread into the
    /// variant), so they have no Rust type of their own.
    pub(crate) absorbs: Vec<String>,
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
    /// The member value, carried by `Variant(T)` — or, in a whole-message
    /// enum with sibling fields, by the `binding` field of
    /// `Variant { payload, ...siblings }`. Encoded through the type's
    /// `ProtoField` impl or a `ProtoAdapter` (`#[armonik(with = "...")]`,
    /// which skips the shape checks by design).
    Payload {
        ty: Box<syn::Type>,
        adapter: Option<Box<syn::Type>>,
        checks: Box<FieldChecks>,
        binding: Option<syn::Ident>,
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

/// Outcome of matching a struct variant's named fields against the
/// whole-message enum's sibling fields (see [`sibling_variant_fields`]).
enum SiblingSplit {
    /// Every field is a sibling: the attribute-less "no member set" variant.
    NoMemberSet,
    /// One field is the member payload: its ident, type, and optional
    /// `#[armonik(with = ...)]` adapter. Boxed: `syn::Type` dwarfs the other
    /// variants.
    Payload(Box<(syn::Ident, syn::Type, Option<syn::Type>)>),
    /// The variant is malformed; errors were already pushed.
    Failed,
}

/// Partition the named fields of a variant into the message's sibling
/// fields (updating/checking the cross-variant bindings) and at most one
/// remaining field, the member payload.
fn sibling_variant_fields(
    named: &syn::FieldsNamed,
    sibling_metas: &[&FieldMeta],
    sibling_bindings: &mut [Option<(syn::Ident, syn::Type)>],
    errors: &mut Errors,
    variant_span: Span,
    proto_name: &str,
) -> SiblingSplit {
    let mut failed = false;
    let mut seen = vec![false; sibling_metas.len()];
    let mut payload: Option<(syn::Ident, syn::Type, Option<syn::Type>)> = None;
    for field in &named.named {
        let ident = field.ident.clone().expect("named fields have idents");
        let Some((FieldAttrs { rename, with, .. }, entries_ok)) = scan_attrs(
            &field.attrs,
            Allowed {
                rename: true,
                with: true,
                absorbs: true,
                ..Allowed::default()
            },
            "this armonik attribute is not valid on a variant field",
            errors,
        ) else {
            failed = true;
            continue;
        };
        if !entries_ok {
            failed = true;
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
        SiblingSplit::Failed
    } else {
        match payload {
            Some(payload) => SiblingSplit::Payload(Box::new(payload)),
            None => SiblingSplit::NoMemberSet,
        }
    }
}

/// Read-only context shared by the per-shape variant resolvers below: the
/// variant being resolved and everything already known about the oneof member
/// it maps to. The mutable state each resolver touches (`errors`, and the
/// `absorbs`/`sibling_bindings` a particular shape feeds) is passed alongside.
struct VariantCtx<'a> {
    variant: &'a syn::Variant,
    field_meta: &'a FieldMeta,
    index: &'a DescriptorIndex,
    span: Span,
    proto_name: &'a str,
    proto_path: &'a str,
    member_name: &'a str,
    /// `#[armonik(present)]` was set on the variant.
    present: bool,
}

/// A resolver returns the variant's shape, or `Err(())` after pushing the
/// error(s) that make this variant unresolvable (the caller skips it).
type ResolvedShape = Result<OneofVariantShape, ()>;

/// Whole-message enum with sibling fields: a struct variant carrying one
/// member payload plus every non-oneof field.
fn resolve_sibling_variant(
    ctx: &VariantCtx,
    sibling_metas: &[&FieldMeta],
    sibling_bindings: &mut [Option<(syn::Ident, syn::Type)>],
    with: &Option<(Span, syn::Type)>,
    errors: &mut Errors,
) -> ResolvedShape {
    if ctx.present {
        errors.push(syn::Error::new(
            ctx.span,
            "#[armonik(present)] markers are not supported in whole-message \
             enums with sibling fields",
        ));
        return Err(());
    }
    if let Some((with_span, _)) = with {
        errors.push(syn::Error::new(
            *with_span,
            "in whole-message enums with sibling fields, put with = ... on the \
             member payload field",
        ));
        return Err(());
    }
    let syn::Fields::Named(named) = &ctx.variant.fields else {
        errors.push(syn::Error::new(
            ctx.span,
            "variants of a whole-message enum with sibling fields must be \
             struct variants carrying the sibling fields",
        ));
        return Err(());
    };
    let (payload, ty, adapter) = match sibling_variant_fields(
        named,
        sibling_metas,
        sibling_bindings,
        errors,
        ctx.span,
        ctx.proto_name,
    ) {
        SiblingSplit::Payload(payload) => *payload,
        SiblingSplit::NoMemberSet => {
            errors.push(syn::Error::new(
                ctx.span,
                format!(
                    "the variant needs a payload field for the member \
                     `{}`",
                    ctx.member_name
                ),
            ));
            return Err(());
        }
        SiblingSplit::Failed => return Err(()),
    };
    let checks = if adapter.is_some() {
        FieldChecks::none()
    } else {
        expected_checks(ctx.field_meta)
    };
    Ok(OneofVariantShape::Payload {
        ty: Box::new(ty),
        adapter: adapter.map(Box::new),
        checks: Box::new(checks),
        binding: Some(payload),
    })
}

/// `#[armonik(present)]` unit variant selected by a `bool` or empty-message
/// member.
fn resolve_marker_variant(
    ctx: &VariantCtx,
    with: &Option<(Span, syn::Type)>,
    errors: &mut Errors,
) -> ResolvedShape {
    if let Some((with_span, _)) = with {
        errors.push(syn::Error::new(
            *with_span,
            "with = ... and present cannot be combined on a oneof variant",
        ));
        return Err(());
    }
    if !matches!(ctx.variant.fields, syn::Fields::Unit) {
        errors.push(syn::Error::new(
            ctx.span,
            "#[armonik(present)] variants must be unit variants",
        ));
        return Err(());
    }
    match &ctx.field_meta.kind {
        FieldKind::Bool => Ok(OneofVariantShape::MarkerBool),
        FieldKind::Message(_) => Ok(OneofVariantShape::MarkerMessage),
        other => {
            errors.push(syn::Error::new(
                ctx.span,
                format!(
                    "#[armonik(present)] needs a bool or message member, but \
                     `{}` is {other:?}",
                    ctx.proto_path
                ),
            ));
            Err(())
        }
    }
}

/// The payload shapes: `Variant(T)` (a single-payload member, optionally
/// through a `with = "..."` adapter) or `Variant { .. }` (a struct variant
/// inlining the fields of a message member, whose message is therefore
/// absorbed).
fn resolve_plain_variant(
    ctx: &VariantCtx,
    with: Option<(Span, syn::Type)>,
    absorbs: &mut Vec<String>,
    errors: &mut Errors,
) -> ResolvedShape {
    match &ctx.variant.fields {
        syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let adapter = with.map(|(_, adapter)| Box::new(adapter));
            let checks = match &adapter {
                Some(_) => FieldChecks::none(),
                None => expected_checks(ctx.field_meta),
            };
            Ok(OneofVariantShape::Payload {
                ty: Box::new(fields.unnamed[0].ty.clone()),
                adapter,
                checks: Box::new(checks),
                binding: None,
            })
        }
        _ if with.is_some() => {
            let (with_span, _) = with.expect("checked above");
            errors.push(syn::Error::new(
                with_span,
                "with = ... needs a single-payload tuple variant",
            ));
            Err(())
        }
        syn::Fields::Named(named) => {
            let FieldKind::Message(inner_name) = &ctx.field_meta.kind else {
                errors.push(syn::Error::new(
                    ctx.span,
                    format!(
                        "struct variants inline a message member, but `{}` \
                         is not a message",
                        ctx.proto_path
                    ),
                ));
                return Err(());
            };
            let Some(inner) = ctx.index.messages.get(inner_name) else {
                errors.push(syn::Error::new(
                    ctx.span,
                    format!("proto message `{inner_name}` not found"),
                ));
                return Err(());
            };
            if !inner.oneofs.is_empty() {
                errors.push(syn::Error::new(
                    ctx.span,
                    format!(
                        "`{inner_name}` contains a oneof; it cannot be inlined into \
                         a struct variant"
                    ),
                ));
                return Err(());
            }
            let mut matcher = Matcher::new(inner_name, inner);
            let mut parts = Vec::new();
            for part in &named.named {
                let part_ident = part.ident.clone().expect("named fields have idents");
                let Some((
                    FieldAttrs {
                        rename: part_rename,
                        ..
                    },
                    _,
                )) = scan_attrs(
                    &part.attrs,
                    Allowed {
                        rename: true,
                        ..Allowed::default()
                    },
                    "this armonik attribute is not valid on a struct variant field",
                    errors,
                )
                else {
                    continue;
                };
                let part_name = part_rename.unwrap_or_else(|| unraw(&part_ident));
                // The message has no oneofs, so a hit is always a field.
                let Some(Found::Field(part_meta)) =
                    matcher.find(&part_name, part_ident.span(), errors)
                else {
                    continue;
                };
                parts.push(InlinePart {
                    span: part_ident.span(),
                    ident: part_ident,
                    ty: part.ty.clone(),
                    tag: part_meta.tag,
                    proto_path: format!("{inner_name}.{}", part_meta.name),
                    checks: expected_checks(part_meta),
                });
            }
            matcher.check_complete(ctx.span, errors);
            parts.sort_by_key(|part| part.tag);
            absorbs.push(inner_name.clone());
            Ok(OneofVariantShape::Inline { parts })
        }
        _ => {
            errors.push(syn::Error::new(
                ctx.span,
                "oneof variants must be `Variant(T)`, `Variant { .. }`, a \
                 #[armonik(present)] marker, or the attribute-less default",
            ));
            Err(())
        }
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
        errors.push(not_found(message_span, "message", &proto_name));
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
    // Messages inlined into struct variants: no Rust type stands for them.
    let mut absorbs: Vec<String> = Vec::new();
    for variant in &data.variants {
        let span = variant.ident.span();
        let Some((
            FieldAttrs {
                rename,
                with,
                present,
                ..
            },
            _,
        )) = scan_attrs(
            &variant.attrs,
            Allowed {
                rename: true,
                with: true,
                present: true,
                absorbs: true,
                ..Allowed::default()
            },
            "this armonik attribute is not valid on a oneof variant",
            &mut errors,
        )
        else {
            continue;
        };

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
                    SiblingSplit::NoMemberSet => {
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
                    SiblingSplit::Payload(..) => {}
                    SiblingSplit::Failed => continue,
                }
            }
        }
        let Some((position, field_meta)) = member else {
            let available = oneof
                .fields
                .iter()
                .map(|&field| meta.fields[field].name.clone())
                .collect();
            errors.push(unknown_name(
                span,
                "member",
                &member_name,
                &format!("oneof `{proto_name}.{}`", oneof.name),
                available,
                "use #[armonik(rename = \"...\")] if the names differ",
            ));
            continue;
        };
        covered[position] = true;
        let proto_path = format!("{proto_name}.{}", field_meta.name);

        let ctx = VariantCtx {
            variant,
            field_meta,
            index,
            span,
            proto_name: &proto_name,
            proto_path: &proto_path,
            member_name: &member_name,
            present,
        };
        let resolved = if !sibling_metas.is_empty() {
            resolve_sibling_variant(
                &ctx,
                &sibling_metas,
                &mut sibling_bindings,
                &with,
                &mut errors,
            )
        } else if present {
            resolve_marker_variant(&ctx, &with, &mut errors)
        } else {
            resolve_plain_variant(&ctx, with, &mut absorbs, &mut errors)
        };
        let shape = match resolved {
            Ok(shape) => shape,
            Err(()) => continue,
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
        absorbs,
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

#[cfg(test)]
mod tests {
    //! Guards for [`scan_field_attrs`], the one place that decides which
    //! `#[armonik(...)]` keys each field/variant site accepts. The full
    //! derives can only be exercised inside the `armonik` crate (they read the
    //! build-script descriptor), and the differential harness only fuzzes
    //! *valid* input — so the per-site *rejection* rules, which the shared
    //! collector could silently weaken, are pinned here instead.

    use proc_macro2::Span;

    use super::*;

    fn entry(item: AttrItem) -> attrs::AttrEntry {
        attrs::AttrEntry {
            span: Span::call_site(),
            item,
        }
    }

    fn lit(value: &str) -> syn::LitStr {
        syn::LitStr::new(value, Span::call_site())
    }

    fn scan(entries: &[attrs::AttrEntry], allowed: Allowed) -> (FieldAttrs, bool, bool) {
        let mut errors = Errors::new();
        let (collected, ok) = scan_field_attrs(entries, allowed, "reject", &mut errors);
        (collected, ok, errors.into_result().is_ok())
    }

    /// `absorbs` must stay rejected where a site does not opt in, and be
    /// tolerated (no error — it is harvested by `collect_absorbs` in lib.rs)
    /// where it does. This is the exact rule a naive shared collector would
    /// drop, and there is no other test that would catch it.
    #[test]
    fn absorbs_is_gated_per_site() {
        let (_, ok, clean) = scan(
            &[entry(AttrItem::Absorbs(lit("some.Msg")))],
            Allowed {
                absorbs: true,
                ..Allowed::default()
            },
        );
        assert!(ok && clean, "absorbs tolerated where opted in");

        let (_, ok, clean) = scan(
            &[entry(AttrItem::Absorbs(lit("some.Msg")))],
            Allowed::default(),
        );
        assert!(!ok && !clean, "absorbs rejected where not opted in");
    }

    #[test]
    fn collects_only_enabled_keys() {
        let (collected, ok, clean) = scan(
            &[
                entry(AttrItem::Rename(lit("proto_name"))),
                entry(AttrItem::Present),
            ],
            Allowed {
                rename: true,
                present: true,
                ..Allowed::default()
            },
        );
        assert!(ok && clean);
        assert_eq!(collected.rename.as_deref(), Some("proto_name"));
        assert!(collected.present);
    }

    #[test]
    fn disallowed_key_is_rejected_and_not_collected() {
        // `present` at a site that only accepts `rename`.
        let (collected, ok, clean) = scan(
            &[entry(AttrItem::Present)],
            Allowed {
                rename: true,
                ..Allowed::default()
            },
        );
        assert!(!ok && !clean);
        assert!(!collected.present);
    }
}
