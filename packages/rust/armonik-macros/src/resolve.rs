//! Resolution: from the annotated item and the descriptor to one [`Ir`].
//!
//! Every shape `#[armonik_macros::message]` accepts lands in the same plan; what varies is how its
//! slots are found. A struct's fields resolve by name against one message ([`plain_ir`]), or carry
//! explicit tags when the type is generic and names no message ([`generic_ir`]), or delegate whole
//! to the single field of a `transparent` newtype ([`transparent_ir`]). An enum's variants resolve
//! against a oneof's members, with the message's non-oneof fields as a possibly-empty shared set:
//! "no sibling fields" is the same case as "sibling fields, and there are zero of them", and a
//! plain struct is the same case again with no oneof at all.

use proc_macro2::Span;
use syn::spanned::Spanned;

use crate::attrs::{scan_attrs, unraw, Allowed, AttrEntry, AttrItem, Errors, FieldAttrs};
use crate::descriptor::{DescriptorIndex, FieldKind, FieldMeta};
use crate::matcher::{not_found, unknown_name, Found, Matcher};
use crate::plan::{Arm, Discr, Expectation, FieldAccess, Ir, Slot, SlotCodec};

/// Pick the shape `#[armonik_macros::message]` is standing for and resolve it.
///
/// The single home of that decision, and of the type-level attribute scan the shapes are chosen
/// from: a shape resolver is handed what it needs and never rescans.
pub(crate) fn resolve_message(input: &syn::DeriveInput) -> Result<Ir, Errors> {
    let index = index(input)?;
    let entries = crate::attrs::parse(&input.attrs)?;

    let mut proto_names: Vec<(Span, String)> = Vec::new();
    let mut stray: Vec<Span> = Vec::new();
    let mut oneof_attr = false;
    // Spans, not flags: the two are mutually exclusive and the rejection points at one of them.
    let mut generic: Option<Span> = None;
    let mut transparent: Option<Span> = None;
    for entry in &entries {
        match &entry.item {
            AttrItem::Message(lit) => proto_names.push((entry.span, lit.value())),
            AttrItem::Oneof(_) => oneof_attr = true,
            AttrItem::Generic => generic = Some(entry.span),
            AttrItem::Transparent => transparent = Some(entry.span),
            _ => stray.push(entry.span),
        }
    }

    // Enums are oneof-shaped: `message = ...` alone stands for a whole message with a single
    // inferred oneof, `oneof = ...` for one oneof of a message, embedded in a struct. Dispatched on
    // before anything is reported, because a oneof reads the same entries for itself and rejects a
    // stray key in its own words.
    if oneof_attr || (matches!(input.data, syn::Data::Enum(_)) && generic.is_none()) {
        return oneof_ir(input, &index, &entries);
    }

    let mut errors = Errors::new();
    if let (Some(_), Some(transparent_span)) = (generic, transparent) {
        errors.at(
            transparent_span,
            "generic and transparent cannot be combined: transparent flattens a single-field \
             wrapper message into the type, generic skips descriptor validation because a \
             generic type names no proto message, and there is no wrapper to flatten without one",
        );
        return Err(errors);
    }
    for span in stray {
        errors.at(
            span,
            "this armonik attribute is not valid at type level on a struct",
        );
    }
    if generic.is_some() {
        if !proto_names.is_empty() {
            errors.at(
                input.ident.span(),
                "#[armonik(generic)] types are not validated against the descriptor; \
                 remove the message attribute",
            );
            return Err(errors);
        }
        return generic_ir(input, &index, errors);
    }
    if transparent.is_some() {
        return transparent_ir(input, &index, proto_names, errors);
    }
    plain_ir(input, &index, proto_names, errors)
}

/// The compiled descriptor set, or a spanned error naming the type that wanted it.
///
/// Loaded here rather than by the entry points, so that a descriptor which fails to load reads as
/// the reason this type could not be resolved, and both macros stay free of `?`.
pub(crate) fn index(input: &syn::DeriveInput) -> Result<std::sync::Arc<DescriptorIndex>, Errors> {
    crate::descriptor::index()
        .map_err(|message| syn::Error::new(input.ident.span(), message).into())
}

/// The discriminant-less [`Ir`] every struct shape shares.
fn struct_ir(input: &syn::DeriveInput, fingerprint: u64, shared: Vec<Slot>) -> Ir {
    Ir {
        ident: input.ident.clone(),
        generics: input.generics.clone(),
        fingerprint,
        names: Vec::new(),
        fragment_of: None,
        docs: Vec::new(),
        absorbs: Vec::new(),
        generic: false,
        shared,
        discr: None,
    }
}

// ---- Plain struct: every field is a field of one proto message ----

fn plain_ir(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
    proto_names: Vec<(Span, String)>,
    mut errors: Errors,
) -> Result<Ir, Errors> {
    if proto_names.is_empty() {
        errors.at(
            input.ident.span(),
            "missing #[armonik(message = \"full.proto.Name\")] \
             (or #[armonik(generic)] with explicit tags)",
        );
        return Err(errors);
    }
    if !input.generics.params.is_empty() {
        errors.at(
            input.ident.span(),
            "descriptor-validated types cannot be generic; use #[armonik(generic)]",
        );
        return Err(errors);
    }

    // One proto message per struct. `message = ...` is repeatable on an *enum*, where a unified
    // type stands for several identical protos; a struct resolves against exactly one.
    for (span, _) in proto_names.iter().skip(1) {
        errors.at(
            *span,
            "a struct stands for one proto message; declare one #[armonik(message = ...)]",
        );
    }
    let (name, meta) = {
        let (span, name) = &proto_names[0];
        match index.messages.get(name) {
            Some(meta) => (name.as_str(), meta),
            None => {
                errors.push(not_found(*span, "message", name));
                return Err(errors);
            }
        }
    };

    let syn::Data::Struct(data) = &input.data else {
        errors.at(
            input.ident.span(),
            "#[armonik_macros::message] with `message = ...` expects a struct \
             (use `oneof = ...` for flattened oneofs)",
        );
        return Err(errors);
    };

    let mut fields = Vec::new();
    // Messages a `with` adapter flattens away, so no Rust type stands for them.
    let mut absorbs = Vec::new();
    let mut matcher = Matcher::new(name, meta);

    for (field_index, field) in data.fields.iter().enumerate() {
        let (span, access) = field_access(field, field_index);
        // No `tag`: a descriptor-validated field takes its tag from the descriptor, and every one
        // of the six `tag = ...` sites in the crate is inside an `#[armonik(generic)]` struct,
        // which `generic_ir` handles. Spelling one here only ever restated what the proto says.
        let Some(FieldAttrs {
            rename,
            with,
            absorbs: declared,
            ..
        }) = scan_attrs(
            &field.attrs,
            Allowed {
                rename: true,
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
        absorbs.extend(declared);
        let with = with.map(|(_, ty)| ty);

        let proto_name = match (&rename, &field.ident) {
            (Some(name), _) => name.clone(),
            (None, Some(ident)) => unraw(ident),
            (None, None) => {
                errors.at(
                    span,
                    "tuple struct fields need #[armonik(rename = \"proto_field_name\")]",
                );
                continue;
            }
        };

        let Some(resolved) = matcher.find(&proto_name, span, &mut errors) else {
            continue;
        };

        let proto_path = format!("{name}.{proto_name}");
        match resolved {
            Found::Oneof { tags } => {
                if with.is_some() {
                    errors.at(
                        span,
                        "with/tag attributes are not supported on oneof fields",
                    );
                    continue;
                }
                fields.push(Slot {
                    access: Some(access),
                    span,
                    tag: tags.iter().copied().min().unwrap_or_default(),
                    codec: SlotCodec::Delegate {
                        ty: Box::new(field.ty.clone()),
                        tags: Some(tags),
                    },
                    checks: None,
                    proto_path,
                    // A oneof is reached through a Rust field named after the *declaration*, which
                    // carries no comment of its own in the descriptor.
                    docs: Vec::new(),
                });
            }
            Found::Field(field_meta) => fields.push(Slot {
                access: Some(access),
                span,
                tag: field_meta.tag,
                checks: with.is_none().then(|| Expectation::of(field_meta)),
                codec: SlotCodec::Field {
                    ty: Box::new(field.ty.clone()),
                    adapter: with.map(Box::new),
                },
                proto_path,
                docs: field_meta.docs.clone(),
            }),
        }
    }

    // Completeness: every proto field and oneof must be covered by a Rust field.
    matcher.check_complete(input.ident.span(), &mut errors);

    errors.into_result()?;

    fields.sort_by_key(|field| field.tag);
    Ok(Ir {
        names: proto_names.into_iter().map(|(_, name)| name).collect(),
        docs: meta.docs.clone(),
        absorbs,
        ..struct_ir(input, index.fingerprint, fields)
    })
}

// ---- Transparent struct: a single-field newtype delegating its whole impl to that field ----

/// The field is not matched against the descriptor (the inner type already validates itself); only
/// the named proto message is checked to exist, and the emitted assert checks the delegate is
/// wire-identical to it.
fn transparent_ir(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
    proto_names: Vec<(Span, String)>,
    mut errors: Errors,
) -> Result<Ir, Errors> {
    if !input.generics.params.is_empty() {
        errors.at(
            input.ident.span(),
            "#[armonik(transparent)] structs cannot be generic",
        );
    }
    if proto_names.len() != 1 {
        errors.at(
            input.ident.span(),
            "#[armonik(transparent)] structs need exactly one \
             #[armonik(message = \"full.proto.Name\")]",
        );
    }
    for (span, name) in &proto_names {
        if !index.messages.contains_key(name) {
            errors.push(not_found(*span, "message", name));
        }
    }
    let syn::Data::Struct(data) = &input.data else {
        errors.at(
            input.ident.span(),
            "#[armonik(transparent)] expects a struct",
        );
        return Err(errors);
    };
    if data.fields.len() != 1 {
        errors.at(
            input.ident.span(),
            "#[armonik(transparent)] structs must have exactly one field, delegated to",
        );
        return Err(errors);
    }
    let field = data.fields.iter().next().expect("one field");
    let (_, access) = field_access(field, 0);
    let delegate = Slot {
        access: Some(access),
        span: field.ty.span(),
        tag: 0,
        codec: SlotCodec::Delegate {
            ty: Box::new(field.ty.clone()),
            tags: None,
        },
        checks: None,
        proto_path: String::new(),
        // The delegate is not matched against the descriptor; the inner type documents itself.
        docs: Vec::new(),
    };

    errors.into_result()?;

    let docs = proto_names
        .first()
        .and_then(|(_, name)| index.messages.get(name))
        .map(|meta| meta.docs.clone())
        .unwrap_or_default();
    Ok(Ir {
        names: proto_names.into_iter().map(|(_, name)| name).collect(),
        docs,
        ..struct_ir(input, index.fingerprint, vec![delegate])
    })
}

// ---- Generic struct: no descriptor to validate against ----

/// Every field carries its own tag, and the concrete instantiations are covered through their
/// `#[armonik_macros::alias]` sites and the differential harness.
fn generic_ir(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
    mut errors: Errors,
) -> Result<Ir, Errors> {
    let syn::Data::Struct(data) = &input.data else {
        errors.at(input.ident.span(), "#[armonik(generic)] expects a struct");
        return Err(errors);
    };

    let mut fields = Vec::new();
    for (field_index, field) in data.fields.iter().enumerate() {
        let (span, access) = field_access(field, field_index);
        // No `with`: the only check a generic type gets is the field-shape comparison at each
        // `#[armonik_macros::alias]`, which reads `ProtoField::SHAPE` per field. An adapter has no
        // shape to report -- it exists because the Rust representation is deliberately not the
        // proto's -- so a field carrying one would have nothing to put in `GenericFields::FIELDS`.
        let Some(FieldAttrs { tag, .. }) = scan_attrs(
            &field.attrs,
            Allowed {
                tag: true,
                ..Allowed::default()
            },
            "generic-mode fields only take tag = ...",
            &mut errors,
        ) else {
            continue;
        };
        let Some((_, tag)) = tag else {
            errors.at(
                span,
                "generic-mode fields need an explicit #[armonik(tag = ...)]",
            );
            continue;
        };

        let field_name = field
            .ident
            .as_ref()
            .map(|ident| ident.to_string())
            .unwrap_or_else(|| field_index.to_string());
        fields.push(Slot {
            access: Some(access),
            span,
            tag,
            codec: SlotCodec::Field {
                ty: Box::new(field.ty.clone()),
                adapter: None,
            },
            checks: None,
            proto_path: format!("{}.{field_name}", input.ident),
            // A generic type names no proto message, so there is nothing to harvest.
            docs: Vec::new(),
        });
    }

    errors.into_result()?;

    fields.sort_by_key(|field| field.tag);
    Ok(Ir {
        generic: true,
        ..struct_ir(input, index.fingerprint, fields)
    })
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

// ---- Oneof-shaped enums ----
//
// One variant per member, either narrowing a single oneof of a larger message or standing for a
// whole message whose non-oneof fields every variant carries.

/// The message's non-oneof fields, and the one Rust binding (name and type) every variant must
/// agree on for each, fixed by the first variant that declares it and checked in the others, since
/// the emitted patterns spell each sibling exactly one way.
///
/// Possibly empty, which is what makes the resolution one code path rather than two: an enum
/// standing for a whole message with sibling fields and one narrowing a single oneof differ only
/// in how many entries land here.
struct Siblings<'a> {
    proto_name: &'a str,
    entries: Vec<(&'a FieldMeta, Option<(syn::Ident, syn::Type)>)>,
}

impl<'a> Siblings<'a> {
    fn new(selected: &'a Selected<'_>) -> Self {
        let metas = selected
            .whole_message
            .then(|| {
                selected
                    .meta
                    .fields
                    .iter()
                    .filter(|field| field.oneof.is_none())
            })
            .into_iter()
            .flatten();
        Self {
            proto_name: &selected.proto_name,
            entries: metas.map(|meta| (meta, None)).collect(),
        }
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Claim the sibling named `name` for one variant's field: `None` when no sibling has that
    /// name, so the field is a leftover belonging to the member. `Some(false)` when the binding
    /// disagrees with the other variants', with the errors pushed.
    fn claim(
        &mut self,
        seen: &mut [bool],
        name: &str,
        ident: &syn::Ident,
        ty: &syn::Type,
        errors: &mut Errors,
    ) -> Option<bool> {
        let position = self
            .entries
            .iter()
            .position(|(meta, _)| meta.name == name)?;
        seen[position] = true;
        let mut ok = true;
        match &self.entries[position].1 {
            None => self.entries[position].1 = Some((ident.clone(), ty.clone())),
            Some((bound_ident, bound_ty)) => {
                if bound_ident != ident {
                    errors.at(
                        ident.span(),
                        format!(
                            "sibling field `{name}` must use the same name in every \
                             variant (`{bound_ident}` elsewhere)"
                        ),
                    );
                    ok = false;
                }
                if quote::quote!(#bound_ty).to_string() != quote::quote!(#ty).to_string() {
                    errors.at(
                        ty.span(),
                        format!(
                            "sibling field `{name}` must use the same type in every \
                             variant"
                        ),
                    );
                    ok = false;
                }
            }
        }
        Some(ok)
    }

    /// Report every sibling the variant failed to declare; `seen` marks the ones it did. Returns
    /// whether the variant is complete.
    fn require_all(&self, seen: &[bool], variant_span: Span, errors: &mut Errors) -> bool {
        let mut complete = true;
        for (position, field_seen) in seen.iter().enumerate() {
            if !field_seen {
                errors.at(
                    variant_span,
                    format!(
                        "the variant must carry the sibling field `{}` of `{}` \
                         (every variant of a whole-message enum declares all non-oneof \
                         fields)",
                        self.entries[position].0.name, self.proto_name
                    ),
                );
                complete = false;
            }
        }
        complete
    }

    /// The siblings as the plan's shared slots, in tag order: one per sibling that some variant
    /// bound. A missing binding is only possible when every variant errored, and those errors are
    /// already reported.
    fn into_slots(self) -> Vec<Slot> {
        let proto_name = self.proto_name;
        let mut slots: Vec<Slot> = self
            .entries
            .into_iter()
            .filter_map(|(meta, binding)| {
                let (ident, ty) = binding?;
                Some(Slot {
                    span: ident.span(),
                    access: Some(FieldAccess::Named(ident)),
                    tag: meta.tag,
                    codec: SlotCodec::Field {
                        ty: Box::new(ty),
                        adapter: None,
                    },
                    proto_path: format!("{proto_name}.{}", meta.name),
                    checks: Some(Expectation::of(meta)),
                    docs: meta.docs.clone(),
                })
            })
            .collect();
        slots.sort_by_key(|slot| slot.tag);
        slots
    }
}

/// What a variant carries beyond the message's non-oneof fields, whatever its syntactic shape.
///
/// Both questions about a variant are answered from this one fact: whether it means "the oneof has
/// no member set", and, once it names a member, how that member is reached.
enum Carried {
    /// A struct variant's fields beyond the shared ones: the member carried whole, or the member
    /// message's own fields under `inline`. Empty means the variant carries nothing of its own,
    /// which is the "no member set" case when it names no member either. A unit variant is the empty
    /// case by construction.
    Fields(Vec<Leftover>),
    /// A tuple variant's payload, which is always a member and never a sibling, so a tuple variant
    /// never means "no member set".
    Payload,
}

impl Carried {
    /// Whether the variant carries nothing of its own.
    fn is_empty(&self) -> bool {
        matches!(self, Carried::Fields(leftovers) if leftovers.is_empty())
    }

    /// The struct-variant leftovers; empty for the shapes that have none, which do not read them.
    fn into_leftovers(self) -> Vec<Leftover> {
        match self {
            Carried::Fields(leftovers) => leftovers,
            Carried::Payload => Vec::new(),
        }
    }
}

/// Sort a variant's fields between the siblings it must carry and the leftovers belonging to the
/// member. A unit variant is the named case over zero fields, so where the message has siblings it
/// gets the same missing-field diagnosis as a struct variant that dropped them, instead of falling
/// through to an "unknown member" error naming the wrong problem.
///
/// `None` when the variant is malformed; the errors are already pushed.
fn carried(
    fields: &syn::Fields,
    siblings: &mut Siblings<'_>,
    absorbs: &mut Vec<String>,
    errors: &mut Errors,
    variant_span: Span,
) -> Option<Carried> {
    let named: Vec<&syn::Field> = match fields {
        syn::Fields::Named(named) => named.named.iter().collect(),
        syn::Fields::Unit => Vec::new(),
        syn::Fields::Unnamed(_) => return Some(Carried::Payload),
    };

    let mut failed = false;
    let mut seen = vec![false; siblings.entries.len()];
    let mut leftovers: Vec<Leftover> = Vec::new();
    for field in named {
        let ident = field.ident.clone().expect("named fields have idents");
        let Some(FieldAttrs {
            rename,
            with,
            absorbs: declared,
            ..
        }) = scan_attrs(
            &field.attrs,
            Allowed {
                rename: true,
                with: true,
                absorbs: true,
                ..Allowed::default()
            },
            "this armonik attribute is not valid on a struct variant field",
            errors,
        )
        else {
            failed = true;
            continue;
        };
        absorbs.extend(declared);

        let name = rename.unwrap_or_else(|| unraw(&ident));
        match siblings.claim(&mut seen, &name, &ident, &field.ty, errors) {
            Some(ok) => {
                if let Some((with_span, _)) = with {
                    errors.at(
                        with_span,
                        "with = ... is only valid on the member payload field, not on a \
                         sibling field",
                    );
                    failed = true;
                }
                failed |= !ok;
            }
            None => leftovers.push(Leftover {
                span: ident.span(),
                ident,
                name,
                ty: field.ty.clone(),
                with: with.map(|(_, ty)| ty),
            }),
        }
    }

    failed |= !siblings.require_all(&seen, variant_span, errors);
    (!failed).then_some(Carried::Fields(leftovers))
}

/// A field of a struct variant that is not one of the message's non-oneof fields, so it belongs to
/// the oneof member: the member carried whole, or one of its own fields under `inline`.
struct Leftover {
    ident: syn::Ident,
    /// The proto name it matches by: the Rust name, or `rename`.
    name: String,
    ty: syn::Type,
    span: Span,
    with: Option<syn::Type>,
}

/// How a variant says its member is carried, folded from the `present`, `inline` and `with` keys.
///
/// Folded in one place because the three name three different carriers, so any two of them together
/// is one mistake with one message, whatever the pair.
enum Carrier {
    /// The member carried whole (the default), optionally through a variant-level `with` adapter
    /// (the span is the `with` key's, where a misplaced adapter is reported).
    Whole(Option<(Span, Box<syn::Type>)>),
    /// `#[armonik(present)]`: carried by presence alone.
    Present,
    /// `#[armonik(inline)]`: the member message's own fields, spread into the variant.
    Inline(Span),
}

fn carrier(
    variant_span: Span,
    with: Option<(Span, syn::Type)>,
    present: bool,
    inline: Option<Span>,
    errors: &mut Errors,
) -> Result<Carrier, ()> {
    let mut named = Vec::new();
    if present {
        named.push((variant_span, "present"));
    }
    if let Some(span) = inline {
        named.push((span, "inline"));
    }
    if let Some((span, _)) = &with {
        named.push((*span, "with = ..."));
    }
    if let [_, (second_span, _), ..] = named.as_slice() {
        let keys = named
            .iter()
            .map(|(_, key)| format!("`{key}`"))
            .collect::<Vec<_>>()
            .join(" and ");
        errors.at(
            *second_span,
            format!(
                "{keys} each say how the member is carried (present: by presence alone; \
                 inline: its message's fields spread into the variant; with: through an \
                 adapter), so they cannot be combined"
            ),
        );
        return Err(());
    }
    Ok(if present {
        Carrier::Present
    } else if let Some(span) = inline {
        Carrier::Inline(span)
    } else {
        Carrier::Whole(with.map(|(span, ty)| (span, Box::new(ty))))
    })
}

/// Read-only context shared by the per-carrier variant resolvers below: the variant being resolved
/// and everything already known about the oneof member it maps to. The mutable state each resolver
/// touches (`errors`, and the `absorbs`/`Siblings` a particular shape feeds) is passed
/// alongside.
struct VariantCtx<'a> {
    variant: &'a syn::Variant,
    field_meta: &'a FieldMeta,
    index: &'a DescriptorIndex,
    span: Span,
    proto_name: &'a str,
    proto_path: &'a str,
    member_name: &'a str,
}

/// A resolver returns the variant's shape, or `Err(())` after pushing the error(s) that make this
/// variant unresolvable (the caller skips it).
/// What a variant resolved to: how it carries the member, through which codec, and what the shape
/// assert should check. The caller assembles them into the variant's [`Slot`].
type ResolvedShape = Result<(Option<FieldAccess>, SlotCodec, Option<Expectation>), ()>;

/// Resolve one variant against the oneof member it names.
///
/// One function for every shape, with the message's non-oneof fields as a possibly-empty set. What
/// the shape is read off is the variant's own syntax and its [`Carrier`], never how many siblings
/// the enum happens to have.
fn resolve_variant(
    ctx: &VariantCtx,
    carrier: Carrier,
    leftovers: Vec<Leftover>,
    has_siblings: bool,
    absorbs: &mut Vec<String>,
    errors: &mut Errors,
) -> ResolvedShape {
    match carrier {
        Carrier::Present => {
            // `present` needs a unit variant, and a message with non-oneof fields needs every
            // variant to carry them. Both constraints are real and they cannot both be met, so say
            // that here rather than let the marker resolver demand a unit variant and the
            // completeness check then demand the fields back, three variants away.
            if has_siblings {
                errors.at(
                    ctx.span,
                    format!(
                        "#[armonik(present)] needs a unit variant, but `{}` has non-oneof \
                         fields that every variant must carry; give the variant an empty \
                         member type instead",
                        ctx.proto_name
                    ),
                );
                return Err(());
            }
            resolve_marker_variant(ctx, errors)
        }
        Carrier::Inline(inline_span) => {
            // Rejected rather than supported. The two sets of fields would share one variant and
            // one binding namespace, and their tags come from different messages, so a part at tag
            // 4 and a sibling at tag 4 both bind `__f4`: supporting this needs a second naming
            // scheme, for a shape no site wants. Without the check the resolver accepts it and the
            // emitted patterns do not compile, pointing rustc's "append `, ..`" suggestion at the
            // attribute.
            if has_siblings {
                errors.at(
                    inline_span,
                    format!(
                        "inline and the non-oneof fields of `{}` cannot be combined: every variant \
                         carries those fields, and inline spreads the member's own into the same \
                         variant; carry the member whole in a field of its own instead",
                        ctx.proto_name
                    ),
                );
                return Err(());
            }
            if !matches!(ctx.variant.fields, syn::Fields::Named(_)) {
                errors.at(
                    inline_span,
                    "inline needs a struct variant: there is nothing to spread the \
                     member's fields into",
                );
                return Err(());
            }
            resolve_inline_member(ctx, leftovers, absorbs, errors)
        }
        Carrier::Whole(with) => match &ctx.variant.fields {
            // `Variant(T)`: the member carried whole, optionally through an adapter. It carries no
            // sibling fields, so the enum must have none.
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                if has_siblings {
                    errors.at(
                        ctx.span,
                        format!(
                            "`{}` has non-oneof fields, so every variant must be a struct \
                             variant carrying them",
                            ctx.proto_name
                        ),
                    );
                    return Err(());
                }
                let adapter = with.map(|(_, ty)| ty);
                let checks = adapter.is_none().then(|| Expectation::of(ctx.field_meta));
                Ok((
                    Some(FieldAccess::Indexed(syn::Index::from(0))),
                    SlotCodec::Field {
                        ty: Box::new(fields.unnamed[0].ty.clone()),
                        adapter,
                    },
                    checks,
                ))
            }
            syn::Fields::Named(_) => {
                if let Some((with_span, _)) = with {
                    errors.at(
                        with_span,
                        "in a struct variant, put with = ... on the field carrying the member",
                    );
                    return Err(());
                }
                match <[Leftover; 1]>::try_from(leftovers) {
                    Ok([payload]) => {
                        let adapter = payload.with.map(Box::new);
                        let checks = adapter.is_none().then(|| Expectation::of(ctx.field_meta));
                        Ok((
                            Some(FieldAccess::Named(payload.ident)),
                            SlotCodec::Field {
                                ty: Box::new(payload.ty),
                                adapter,
                            },
                            checks,
                        ))
                    }
                    Err(leftovers) if leftovers.is_empty() => {
                        errors.at(
                            ctx.span,
                            format!(
                                "the variant needs a field carrying the member `{}`",
                                ctx.member_name
                            ),
                        );
                        Err(())
                    }
                    Err(leftovers) => {
                        errors.at(
                            leftovers[1].span,
                            format!(
                                "only one field of the variant may carry the member `{}`; \
                                 add #[armonik(inline)] to the variant if these are the \
                                 member message's own fields, spread into it",
                                ctx.member_name
                            ),
                        );
                        Err(())
                    }
                }
            }
            _ => {
                errors.at(
                    ctx.span,
                    "oneof variants must be `Variant(T)`, `Variant { .. }`, a \
                     #[armonik(present)] marker, or the attribute-less default",
                );
                Err(())
            }
        },
    }
}

/// `#[armonik(present)]` unit variant selected by a `bool` or empty-message member.
///
/// A codec substitution like any other adapter: the value type is `()` (the member carries nothing
/// but its own presence), and the one decision made here is which presence adapter the member's
/// kind calls for.
fn resolve_marker_variant(ctx: &VariantCtx, errors: &mut Errors) -> ResolvedShape {
    if !matches!(ctx.variant.fields, syn::Fields::Unit) {
        errors.at(
            ctx.span,
            "#[armonik(present)] variants must be unit variants",
        );
        return Err(());
    }
    let adapter: syn::Type = match &ctx.field_meta.kind {
        FieldKind::Bool => syn::parse_quote!(crate::codec::adapters::BoolPresence),
        FieldKind::Message(_) => syn::parse_quote!(crate::codec::adapters::EmptyPresence),
        other => {
            errors.at(
                ctx.span,
                format!(
                    "#[armonik(present)] needs a bool or message member, but \
                     `{}` is {other:?}",
                    ctx.proto_path
                ),
            );
            return Err(());
        }
    };
    Ok((
        None,
        SlotCodec::Field {
            ty: Box::new(syn::parse_quote!(())),
            adapter: Some(Box::new(adapter)),
        },
        None,
    ))
}

/// `#[armonik(inline)]`: the variant's leftover fields are the member message's own fields, spread
/// into the variant, so the member message has no Rust type and is absorbed.
fn resolve_inline_member(
    ctx: &VariantCtx,
    leftovers: Vec<Leftover>,
    absorbs: &mut Vec<String>,
    errors: &mut Errors,
) -> ResolvedShape {
    let FieldKind::Message(inner_name) = &ctx.field_meta.kind else {
        errors.at(
            ctx.span,
            format!(
                "inline spreads a message member's fields, but `{}` is not a message",
                ctx.proto_path
            ),
        );
        return Err(());
    };
    let Some(inner) = ctx.index.messages.get(inner_name) else {
        errors.at(ctx.span, format!("proto message `{inner_name}` not found"));
        return Err(());
    };
    if !inner.oneofs.is_empty() {
        errors.at(
            ctx.span,
            format!("`{inner_name}` contains a oneof; it cannot be inlined into a struct variant"),
        );
        return Err(());
    }

    let mut matcher = Matcher::new(inner_name, inner);
    let mut parts = Vec::new();
    for leftover in leftovers {
        if leftover.with.is_some() {
            errors.at(
                leftover.span,
                "with = ... is not supported on an inlined field",
            );
            continue;
        }
        // The message has no oneofs, so a hit is always a field.
        let Some(Found::Field(part_meta)) = matcher.find(&leftover.name, leftover.span, errors)
        else {
            continue;
        };
        parts.push(Slot {
            span: leftover.span,
            access: Some(FieldAccess::Named(leftover.ident)),
            tag: part_meta.tag,
            codec: SlotCodec::Field {
                ty: Box::new(leftover.ty),
                adapter: None,
            },
            proto_path: format!("{inner_name}.{}", part_meta.name),
            checks: Some(Expectation::of(part_meta)),
            // The member message's own field: looking it up in the *containing* message finds
            // nothing, silently.
            docs: part_meta.docs.clone(),
        });
    }
    matcher.check_complete(ctx.span, errors);
    parts.sort_by_key(|part| part.tag);
    absorbs.push(inner_name.clone());
    Ok((None, SlotCodec::Group { parts }, None))
}

/// Which oneof the enum stands for, and whether it stands for the whole message.
struct Selected<'a> {
    /// Full proto name of the message.
    proto_name: String,
    meta: &'a crate::descriptor::MessageMeta,
    oneof: &'a crate::descriptor::OneofMeta,
    /// `message = ...` alone: the enum is the message, and its non-oneof fields become siblings
    /// replicated in every variant.
    whole_message: bool,
}

/// Read the type-level attributes and answer the two questions that fix the shape: which proto
/// message, and which of its oneofs.
///
/// Four ways to get it wrong, all of them about the schema rather than about the Rust item, which is
/// why this is worth reading on its own: no such oneof, a oneof that covers the whole message (use
/// the whole-message shape), a message with no oneof at all (use a struct), and a message with
/// several (declare one enum each).
fn select_oneof<'a>(
    input: &syn::DeriveInput,
    index: &'a DescriptorIndex,
    entries: &[AttrEntry],
    errors: &mut Errors,
) -> Result<Selected<'a>, ()> {
    let mut proto_name: Option<(Span, String)> = None;
    let mut oneof_name: Option<(Span, String)> = None;
    for entry in entries {
        match &entry.item {
            AttrItem::Message(lit) => {
                if proto_name.replace((entry.span, lit.value())).is_some() {
                    errors.at(
                        entry.span,
                        "flattened oneofs support a single proto message",
                    );
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
        errors.at(
            input.ident.span(),
            "oneof-shaped enums need #[armonik(message = \"...\")]",
        );
        return Err(());
    };

    let Some(meta) = index.messages.get(&proto_name) else {
        errors.push(not_found(message_span, "message", &proto_name));
        return Err(());
    };
    // `message = ...` alone: the enum stands for the whole message, whose single oneof is inferred
    // and whose non-oneof fields become siblings replicated in every variant. `oneof = ...`
    // declares a partial enum embedded in a struct, and is rejected when the oneof is the whole
    // message so the two shapes stay visually distinct.
    let (oneof, whole_message) = match &oneof_name {
        Some((oneof_span, oneof_name)) => {
            let Some((oneof_index, oneof)) = meta.oneof(oneof_name) else {
                errors.at(
                    *oneof_span,
                    format!("no oneof named `{oneof_name}` in proto message `{proto_name}`"),
                );
                return Err(());
            };
            if meta
                .fields
                .iter()
                .all(|field| field.oneof == Some(oneof_index))
            {
                errors.at(
                    *oneof_span,
                    format!(
                        "the oneof `{oneof_name}` covers the whole message `{proto_name}`; \
                         drop the oneof attribute: #[armonik(message = ...)] alone declares \
                         a whole-message enum"
                    ),
                );
                return Err(());
            }
            (oneof, false)
        }
        None => match meta.oneofs.len() {
            1 => (&meta.oneofs[0], true),
            0 => {
                errors.at(
                    input.ident.span(),
                    format!(
                        "proto message `{proto_name}` has no oneof; a message without a \
                         oneof is derived on a struct"
                    ),
                );
                return Err(());
            }
            n => {
                errors.at(
                    input.ident.span(),
                    format!(
                        "proto message `{proto_name}` has {n} oneofs; an enum can stand \
                         for the whole message only when there is exactly one. Declare \
                         one enum per oneof with #[armonik(oneof = \"...\")] and compose \
                         them in a struct"
                    ),
                );
                return Err(());
            }
        },
    };

    Ok(Selected {
        proto_name,
        meta,
        oneof,
        whole_message,
    })
}

/// What one variant resolved to.
enum VariantOutcome {
    /// It names the member at this position in the oneof.
    ///
    /// `arm` is `None` when it named the member but could not be resolved. The member is still
    /// covered: the author did write a variant for it, so reporting the enum as leaving it uncovered
    /// on top of the real error would make one mistake read as two. Boxed because the payload dwarfs
    /// the other outcome, and this is a transient per-variant value.
    Member {
        position: usize,
        arm: Option<Box<Arm>>,
    },
    /// It means "the oneof has no member set". The caller owns the at-most-one rule, since that is a
    /// fact about the enum rather than about this variant.
    NoMemberSet,
}

/// Resolve one variant of a oneof-shaped enum. `None` once its errors are pushed and it names no
/// member to attribute them to.
fn resolve_one_variant(
    variant: &syn::Variant,
    selected: &Selected<'_>,
    index: &DescriptorIndex,
    siblings: &mut Siblings<'_>,
    absorbs: &mut Vec<String>,
    errors: &mut Errors,
) -> Option<VariantOutcome> {
    let span = variant.ident.span();
    let FieldAttrs {
        rename,
        with,
        present,
        inline,
        absorbs: declared,
        ..
    } = scan_attrs(
        &variant.attrs,
        Allowed {
            rename: true,
            with: true,
            present: true,
            inline: true,
            absorbs: true,
            ..Allowed::default()
        },
        "this armonik attribute is not valid on a oneof variant",
        errors,
    )?;
    absorbs.extend(declared);

    // Split once, before anything asks what the variant means.
    //
    // `#[armonik(present)]` is the exception, and answers without looking: the member is carried
    // by presence alone, so the variant carries nothing whatever its shape. Asking the fields
    // would report a `present` unit variant in a message with non-oneof fields as having dropped
    // them, when the mistake the author made is `present` itself, which `resolve_variant` says
    // in those terms.
    let carried = if present {
        Carried::Fields(Vec::new())
    } else {
        carried(&variant.fields, siblings, absorbs, errors, span)?
    };

    let member_name = rename
        .clone()
        .unwrap_or_else(|| crate::names::snake_case(&unraw(&variant.ident)));
    let member = selected
        .oneof
        .fields
        .iter()
        .enumerate()
        .find_map(|(position, &field)| {
            (selected.meta.fields[field].name == member_name)
                .then_some((position, &selected.meta.fields[field]))
        });

    // A variant means "the oneof has no member set" when it names no member and carries nothing of
    // its own once the shared fields are accounted for. The unit variant of a sibling-free enum and
    // the struct variant carrying exactly the siblings are that one case at two sibling counts.
    if member.is_none()
        && carried.is_empty()
        && rename.is_none()
        && !present
        && inline.is_none()
        && with.is_none()
    {
        return Some(VariantOutcome::NoMemberSet);
    }
    let Some((position, field_meta)) = member else {
        let available = selected
            .oneof
            .fields
            .iter()
            .map(|&field| selected.meta.fields[field].name.clone())
            .collect();
        errors.push(unknown_name(
            span,
            "member",
            &member_name,
            &format!("oneof `{}.{}`", selected.proto_name, selected.oneof.name),
            available,
            "use #[armonik(rename = \"...\")] if the names differ",
        ));
        return None;
    };
    let proto_path = format!("{}.{}", selected.proto_name, field_meta.name);

    // The carrier is folded here, once the member is known: a variant whose keys conflict, or
    // whose shape does not fit its carrier, did still name its member, so the member reads as
    // covered (`arm: None`) and one mistake reads as one error.
    let resolved = {
        let ctx = VariantCtx {
            variant,
            field_meta,
            index,
            span,
            proto_name: &selected.proto_name,
            proto_path: &proto_path,
            member_name: &member_name,
        };
        carrier(span, with, present, inline, errors).and_then(|carrier| {
            resolve_variant(
                &ctx,
                carrier,
                carried.into_leftovers(),
                !siblings.is_empty(),
                absorbs,
                errors,
            )
        })
    };
    let arm = resolved.ok().map(|(access, codec, checks)| {
        Box::new(Arm {
            ident: variant.ident.clone(),
            own: Slot {
                access,
                span,
                tag: field_meta.tag,
                codec,
                checks,
                proto_path,
                docs: field_meta.docs.clone(),
            },
        })
    });
    Some(VariantOutcome::Member { position, arm })
}

fn oneof_ir(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
    entries: &[AttrEntry],
) -> Result<Ir, Errors> {
    let mut errors = Errors::new();

    let Ok(selected) = select_oneof(input, index, entries, &mut errors) else {
        return Err(errors);
    };
    // Non-oneof fields of a whole-message enum, replicated in every variant.
    let mut siblings = Siblings::new(&selected);

    let syn::Data::Enum(data) = &input.data else {
        errors.at(
            input.ident.span(),
            "#[armonik(oneof = ...)] expects an enum",
        );
        return Err(errors);
    };

    let mut arms = Vec::new();
    let mut default_arm: Option<syn::Ident> = None;
    let mut covered = vec![false; selected.oneof.fields.len()];
    // Messages no Rust type stands for: the ones inlined into struct variants, and the ones a
    // `with` adapter flattens away, declared through `#[armonik(absorbs = "...")]`.
    let mut absorbs: Vec<String> = Vec::new();
    for variant in &data.variants {
        match resolve_one_variant(
            variant,
            &selected,
            index,
            &mut siblings,
            &mut absorbs,
            &mut errors,
        ) {
            Some(VariantOutcome::Member { position, arm }) => {
                covered[position] = true;
                arms.extend(arm.map(|arm| *arm));
            }
            // At most one of them, which is a fact about the enum rather than about any one
            // variant, so it is checked here rather than by the resolver.
            Some(VariantOutcome::NoMemberSet)
                if default_arm.replace(variant.ident.clone()).is_some() =>
            {
                errors.at(
                    variant.ident.span(),
                    "at most one attribute-less variant (the \"no member set\" case) is allowed",
                );
            }
            Some(VariantOutcome::NoMemberSet) => {}
            None => {}
        }
    }

    for (position, member_covered) in covered.iter().enumerate() {
        if !member_covered {
            let field = &selected.meta.fields[selected.oneof.fields[position]];
            errors.at(
                input.ident.span(),
                format!(
                    "oneof member `{}.{}` (tag {}) is not covered by any variant",
                    selected.proto_name, field.name, field.tag
                ),
            );
        }
    }

    errors.into_result()?;

    let shared = siblings.into_slots();
    arms.sort_by_key(|arm| arm.own.tag);
    Ok(Ir {
        ident: input.ident.clone(),
        generics: input.generics.clone(),
        fingerprint: index.fingerprint,
        fragment_of: (!selected.whole_message)
            .then(|| format!("{}.{}", selected.proto_name, selected.oneof.name)),
        docs: selected.meta.docs.clone(),
        names: vec![selected.proto_name],
        absorbs,
        generic: false,
        shared,
        discr: Some(Discr { arms, default_arm }),
    })
}

#[cfg(test)]
mod tests {
    //! The emitter's bindings, checked on the expansion rather than by compiling it.
    //!
    //! A case that *resolves* exercises the whole emission surface, so the compile-fail suite
    //! cannot host it: its crate-root stand-in has no codec to satisfy the shape asserts. What
    //! matters here is not that the output compiles but what it is named, and that reads straight
    //! off the tokens.

    use super::*;

    /// Compile the compile-fail suite's fixture schema and point the descriptor loader at it.
    ///
    /// Once for the whole binary: tests run on their own threads, and two of them writing this file
    /// while `descriptor::index` reads it hands one a truncated `FileDescriptorSet`, which decodes
    /// to a prefix rather than erroring and is then cached by (mtime, len).
    fn fixture_index() -> std::sync::Arc<DescriptorIndex> {
        use prost::Message as _;

        static INDEX: std::sync::OnceLock<std::sync::Arc<DescriptorIndex>> =
            std::sync::OnceLock::new();
        std::sync::Arc::clone(INDEX.get_or_init(|| {
            let dir = std::env::temp_dir().join("armonik-macros-oneof-fixture");
            std::fs::create_dir_all(&dir).expect("create the fixture directory");
            let descriptor = protox::compile(["tests/fixture.proto"], ["tests"])
                .expect("compile tests/fixture.proto")
                .encode_to_vec();
            std::fs::write(dir.join("descriptor.bin"), &descriptor)
                .expect("write the descriptor set");
            std::env::set_var("OUT_DIR", &dir);
            crate::descriptor::index().expect("the fixture index loads")
        }))
    }

    fn resolve(input: &syn::DeriveInput) -> Ir {
        let _ = fixture_index();
        match resolve_message(input) {
            Ok(ir) => ir,
            Err(errors) => panic!("the fixture resolves: {}", errors.into_syn_error()),
        }
    }

    /// A variant's fields are bound under `__f<tag>`, never under the name the user gave them.
    ///
    /// They share a scope with the emitter's own `buf`, `len`, `value` and `body_len`, so a proto
    /// field named like one of those would shadow it: not a wrong encoding but an unimplementable
    /// message, whose errors point into expanded code. `fixture.Hostile` is named to collide.
    #[test]
    fn variant_fields_are_bound_out_of_the_way() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[armonik(message = "fixture.Choice", oneof = "choice")]
            pub enum Choice {
                Text(String),
                Simple(String),
                #[armonik(present)]
                Flag,
                #[armonik(inline)]
                Hostile {
                    buf: String,
                    len: i32,
                    value: String,
                    body_len: String,
                },
            }
        };
        let emitted = crate::emit::message(&resolve(&input)).to_string();

        // Each field appears only as a *pattern key* renaming it out of the way (`buf : __f1`).
        // Binding it under its own name is what would shadow the emitter's `buf`, `len`, `value`
        // or `body_len`, all of which are live in the same scope.
        for (field, tag) in [("buf", 1), ("len", 2), ("value", 3), ("body_len", 4)] {
            let renamed = format!("{field} : __f{tag}");
            assert!(
                emitted.contains(&renamed),
                "expected `{renamed}` in the expansion",
            );
            assert!(
                !emitted.contains(&format!("{{ {field} ,")),
                "`{field}` is bound under its own name somewhere",
            );
        }
    }

    /// A shared field between two member tags is written in tag order, not rejected.
    ///
    /// Each variant's arm writes the fields that variant carries, ordered by tag, so a shared field
    /// straddling the oneof's members needs nothing special. `fixture.Straddled` puts `token` at
    /// tag 2 between members at 1 and 3.
    #[test]
    fn a_shared_field_between_members_is_written_in_tag_order() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[armonik(message = "fixture.Straddled")]
            pub enum Straddled {
                Text { token: String, text: String },
                Other { token: String, other: String },
            }
        };
        let emitted = crate::emit::message(&resolve(&input)).to_string();

        // In the `Text` arm the member is tag 1 and the shared field tag 2, so the member is written
        // first; in `Other` the member is tag 3, so the shared field is.
        let text = emitted
            .split("Self :: Text")
            .nth(1)
            .expect("a Text arm is emitted");
        assert!(
            text.find("(1u32").expect("the member is written")
                < text.find("(2u32").expect("the shared field is written"),
            "tag 1 before tag 2 in the Text arm: {text}",
        );
        let other = emitted
            .split("Self :: Other")
            .nth(1)
            .expect("an Other arm is emitted");
        assert!(
            other.find("(2u32").expect("the shared field is written")
                < other.find("(3u32").expect("the member is written"),
            "tag 2 before tag 3 in the Other arm: {other}",
        );
    }

    /// An embedded oneof records which oneof it stands for; a whole-message enum does not.
    ///
    /// Not a `trybuild` case: this fires at const-eval against the real `codec`, which the
    /// compile-fail suite deliberately does not host (see `tests/ui.rs`). Pinned at the token level
    /// instead, which is what the suite's excluded classes get.
    #[test]
    fn an_embedded_oneof_records_the_oneof_it_stands_for() {
        let embedded: syn::DeriveInput = syn::parse_quote! {
            #[armonik(message = "fixture.Choice", oneof = "choice")]
            pub enum Choice {
                Text(String),
                Simple(String),
                #[armonik(present)]
                Flag,
                Hostile(String),
            }
        };
        let emitted = crate::emit::message(&resolve(&embedded)).to_string();
        assert!(
            emitted.contains("impl crate :: codec :: Oneof for Choice"),
            "the marker is emitted: {emitted}",
        );
        assert!(
            emitted.contains("\"fixture.Choice.choice\""),
            "the marker names the oneof: {emitted}",
        );

        // The whole-message shape is a message, and says which one through `Msg::NAMES`.
        let whole: syn::DeriveInput = syn::parse_quote! {
            #[armonik(message = "fixture.OnlyOneof")]
            pub enum OnlyOneof {
                First(String),
                Second(String),
            }
        };
        let emitted = crate::emit::message(&resolve(&whole)).to_string();
        assert!(
            !emitted.contains("crate :: codec :: Oneof for"),
            "a whole-message enum gets no oneof marker: {emitted}",
        );
    }
}
