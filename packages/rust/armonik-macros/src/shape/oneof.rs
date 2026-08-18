//! Oneof-shaped enums: one variant per member, either narrowing a single oneof of a larger message
//! or standing for a whole message whose non-oneof fields every variant carries.
//!
//! The two are one code path with a possibly-empty shared set (see [`resolve_variant`]).

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;

use crate::attr_site::{scan_attrs, unraw, Allowed, FieldAttrs};
use crate::attrs::{self, AttrItem, Errors};
use crate::descriptor::{DescriptorIndex, FieldKind, FieldMeta};
use crate::emit::{
    message_shaped, slot_asserts, slot_local, slot_merge_in_place, slot_write, MessageBodies,
};
use crate::matcher::{not_found, unknown_name, Found, Matcher};
use crate::plan::{Expectation, FieldAccess, OneofPlan, OneofVariant, Slot, SlotCodec};

/// Partition a struct variant's named fields into the message's non-oneof fields and everything
/// left over.
///
/// The non-oneof set is possibly empty, which is what makes this one function rather than two: an
/// enum standing for a whole message with sibling fields and one narrowing a single oneof differ
/// only in how many fields land on the left. Cross-variant bindings are checked here, since a
/// sibling must be spelled the same way in every variant.
///
/// `None` when the variant is malformed; the errors are already pushed.
pub(crate) fn split_variant_fields(
    named: &syn::FieldsNamed,
    sibling_metas: &[&FieldMeta],
    sibling_bindings: &mut [Option<(syn::Ident, syn::Type)>],
    absorbs: &mut Vec<String>,
    errors: &mut Errors,
    variant_span: Span,
    proto_name: &str,
) -> Option<Vec<Leftover>> {
    let mut failed = false;
    let mut seen = vec![false; sibling_metas.len()];
    let mut leftovers: Vec<Leftover> = Vec::new();

    for field in &named.named {
        let ident = field.ident.clone().expect("named fields have idents");
        let Some((
            FieldAttrs {
                rename,
                with,
                absorbs: declared,
                ..
            },
            _,
        )) = scan_attrs(
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
        if let Some(position) = sibling_metas.iter().position(|meta| meta.name == name) {
            if let Some((with_span, _)) = with {
                errors.at(
                    with_span,
                    "with = ... is only valid on the member payload field, not on a \
                     sibling field",
                );
                failed = true;
            }
            seen[position] = true;
            match &sibling_bindings[position] {
                None => sibling_bindings[position] = Some((ident, field.ty.clone())),
                Some((bound_ident, bound_ty)) => {
                    if *bound_ident != ident {
                        errors.at(
                            ident.span(),
                            format!(
                                "sibling field `{name}` must use the same name in every \
                                 variant (`{bound_ident}` elsewhere)"
                            ),
                        );
                        failed = true;
                    }
                    if quote::quote!(#bound_ty).to_string() != {
                        let ty = &field.ty;
                        quote::quote!(#ty).to_string()
                    } {
                        errors.at(
                            field.ty.span(),
                            format!(
                                "sibling field `{name}` must use the same type in every \
                                 variant"
                            ),
                        );
                        failed = true;
                    }
                }
            }
        } else {
            leftovers.push(Leftover {
                span: ident.span(),
                ident,
                name,
                ty: field.ty.clone(),
                with: with.map(|(_, ty)| ty),
            });
        }
    }

    failed |= !carries_every_sibling(&seen, sibling_metas, errors, variant_span, proto_name);

    (!failed).then_some(leftovers)
}

/// Report every sibling field the variant failed to declare; `seen` marks the ones it did, and is
/// parallel to `sibling_metas`. Returns whether the variant is complete.
///
/// Shared with the unit-variant case, which declares none of them: passing an all-false `seen` is
/// how a unit variant gets the same diagnosis as a struct variant that dropped a field, instead of
/// falling through to an "unknown member" error naming the wrong problem.
fn carries_every_sibling(
    seen: &[bool],
    sibling_metas: &[&FieldMeta],
    errors: &mut Errors,
    variant_span: Span,
    proto_name: &str,
) -> bool {
    let mut complete = true;
    for (position, field_seen) in seen.iter().enumerate() {
        if !field_seen {
            errors.at(
                variant_span,
                format!(
                    "the variant must carry the sibling field `{}` of `{proto_name}` \
                     (every variant of a whole-message enum declares all non-oneof \
                     fields)",
                    sibling_metas[position].name
                ),
            );
            complete = false;
        }
    }
    complete
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

fn carried(
    fields: &syn::Fields,
    sibling_metas: &[&FieldMeta],
    sibling_bindings: &mut [Option<(syn::Ident, syn::Type)>],
    absorbs: &mut Vec<String>,
    errors: &mut Errors,
    variant_span: Span,
    proto_name: &str,
) -> Option<Carried> {
    match fields {
        syn::Fields::Named(named) => split_variant_fields(
            named,
            sibling_metas,
            sibling_bindings,
            absorbs,
            errors,
            variant_span,
            proto_name,
        )
        .map(Carried::Fields),
        // A unit variant carries nothing, which is only correct where the message has no non-oneof
        // fields. Where it has them, the variant is missing all of them, and says so through the
        // same check a struct variant gets.
        syn::Fields::Unit => {
            let seen = vec![false; sibling_metas.len()];
            carries_every_sibling(&seen, sibling_metas, errors, variant_span, proto_name)
                .then(|| Carried::Fields(Vec::new()))
        }
        syn::Fields::Unnamed(_) => Some(Carried::Payload),
    }
}

/// A field of a struct variant that is not one of the message's non-oneof fields, so it belongs to
/// the oneof member: the member carried whole, or one of its own fields under `inline`.
pub(crate) struct Leftover {
    pub(crate) ident: syn::Ident,
    /// The proto name it matches by: the Rust name, or `rename`.
    pub(crate) name: String,
    pub(crate) ty: syn::Type,
    pub(crate) span: Span,
    pub(crate) with: Option<syn::Type>,
}

/// Read-only context shared by the per-shape variant resolvers below: the variant being resolved
/// and everything already known about the oneof member it maps to. The mutable state each resolver
/// touches (`errors`, and the `absorbs`/`sibling_bindings` a particular shape feeds) is passed
/// alongside.
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
    /// The span of `#[armonik(inline)]`, if set on the variant.
    inline: Option<Span>,
}

/// A resolver returns the variant's shape, or `Err(())` after pushing the error(s) that make this
/// variant unresolvable (the caller skips it).
/// What a variant resolved to: how it carries the member, through which codec, and what the shape
/// assert should check. The caller assembles them into the variant's [`Slot`].
type ResolvedShape = Result<(Option<FieldAccess>, SlotCodec, Option<Expectation>), ()>;

/// Resolve one variant against the oneof member it names.
///
/// One function for every shape, with the message's non-oneof fields as a possibly-empty set: "no
/// sibling fields" is the same case as "sibling fields, and there are zero of them". What the shape
/// is read off is the variant's own syntax and its `#[armonik(...)]` keys, never how many siblings
/// the enum happens to have.
fn resolve_variant(
    ctx: &VariantCtx,
    leftovers: Vec<Leftover>,
    sibling_metas: &[&FieldMeta],
    with: Option<(Span, syn::Type)>,
    absorbs: &mut Vec<String>,
    errors: &mut Errors,
) -> ResolvedShape {
    if let Some(inline_span) = ctx.inline {
        if ctx.present {
            errors.at(
                inline_span,
                "inline and present cannot be combined: present records that a member was \
                 set and carries nothing, inline spreads the member's own fields",
            );
            return Err(());
        }
        if with.is_some() {
            errors.at(
                inline_span,
                "inline and with = ... cannot be combined: with names a codec for a member \
                 carried whole, inline spreads the member's own fields",
            );
            return Err(());
        }
        // Rejected rather than supported. The two sets of fields would share one variant and one
        // binding namespace, and their tags come from different messages, so a part at tag 4 and a
        // sibling at tag 4 both bind `__f4`: supporting this needs a second naming scheme, for a
        // shape no site wants. Without the check the resolver accepts it and the emitted patterns
        // do not compile, pointing rustc's "append `, ..`" suggestion at the attribute.
        if !sibling_metas.is_empty() {
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
    }
    if ctx.present {
        // `present` needs a unit variant, and a message with non-oneof fields needs every variant
        // to carry them. Both constraints are real and they cannot both be met, so say that here
        // rather than let `resolve_marker_variant` demand a unit variant and the completeness check
        // then demand the fields back, three variants away.
        if !sibling_metas.is_empty() {
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
        return resolve_marker_variant(ctx, &with, errors);
    }

    match &ctx.variant.fields {
        // `Variant(T)`: the member carried whole, optionally through an adapter. It carries no
        // sibling fields, so the enum must have none.
        syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            if let Some(inline_span) = ctx.inline {
                errors.at(
                    inline_span,
                    "inline needs a struct variant: there is nothing to spread the \
                     member's fields into",
                );
                return Err(());
            }
            if !sibling_metas.is_empty() {
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
            let adapter = with.map(|(_, adapter)| Box::new(adapter));
            let checks = match &adapter {
                Some(_) => None,
                None => Expectation::of(ctx.field_meta),
            };
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
            if let Some((with_span, _)) = &with {
                errors.at(
                    *with_span,
                    "in a struct variant, put with = ... on the field carrying the member",
                );
                return Err(());
            }

            if ctx.inline.is_some() {
                return resolve_inline_member(ctx, leftovers, absorbs, errors);
            }
            match <[Leftover; 1]>::try_from(leftovers) {
                Ok([payload]) => {
                    let adapter = payload.with.map(Box::new);
                    let checks = match &adapter {
                        Some(_) => None,
                        None => Expectation::of(ctx.field_meta),
                    };
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
    }
}

/// `#[armonik(present)]` unit variant selected by a `bool` or empty-message member.
fn resolve_marker_variant(
    ctx: &VariantCtx,
    with: &Option<(Span, syn::Type)>,
    errors: &mut Errors,
) -> ResolvedShape {
    if let Some((with_span, _)) = with {
        errors.at(
            *with_span,
            "with = ... and present cannot be combined on a oneof variant",
        );
        return Err(());
    }
    if !matches!(ctx.variant.fields, syn::Fields::Unit) {
        errors.at(
            ctx.span,
            "#[armonik(present)] variants must be unit variants",
        );
        return Err(());
    }
    match &ctx.field_meta.kind {
        FieldKind::Bool => Ok((
            None,
            SlotCodec::Marker {
                empty_message: false,
            },
            None,
        )),
        FieldKind::Message(_) => Ok((
            None,
            SlotCodec::Marker {
                empty_message: true,
            },
            None,
        )),
        other => {
            errors.at(
                ctx.span,
                format!(
                    "#[armonik(present)] needs a bool or message member, but \
                     `{}` is {other:?}",
                    ctx.proto_path
                ),
            );
            Err(())
        }
    }
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
        if let Some(adapter) = &leftover.with {
            let _ = adapter;
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
            checks: Expectation::of(part_meta),
            // The member message's own field: looking it up in the *containing* message finds
            // nothing, silently.
            docs: part_meta.docs.clone(),
        });
    }
    matcher.check_complete(ctx.span, errors);
    parts.sort_by_key(|part| part.tag);
    absorbs.push(inner_name.clone());
    Ok((None, SlotCodec::Inline { parts }, None))
}

/// The message's non-oneof fields as slots, in tag order: one per sibling that some variant bound.
///
/// Pure assembly, run once the variants have agreed on a name and type for each. A missing binding
/// is only possible when every variant errored, and those errors are already reported.
fn collect_siblings(
    sibling_metas: &[&FieldMeta],
    sibling_bindings: &[Option<(syn::Ident, syn::Type)>],
    proto_name: &str,
) -> Vec<Slot> {
    let mut siblings: Vec<Slot> = sibling_metas
        .iter()
        .zip(sibling_bindings)
        .filter_map(|(meta_field, binding)| {
            let (ident, ty) = binding.as_ref()?;
            Some(Slot {
                span: ident.span(),
                access: Some(FieldAccess::Named(ident.clone())),
                tag: meta_field.tag,
                codec: SlotCodec::Field {
                    ty: Box::new(ty.clone()),
                    adapter: None,
                },
                proto_path: format!("{proto_name}.{}", meta_field.name),
                checks: Expectation::of(meta_field),
                docs: meta_field.docs.clone(),
            })
        })
        .collect();
    siblings.sort_by_key(|sibling| sibling.tag);
    siblings
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
    errors: &mut Errors,
) -> Result<Selected<'a>, ()> {
    let entries = match attrs::parse(&input.attrs) {
        Ok(entries) => entries,
        Err(err) => {
            errors.push(err);
            return Err(());
        }
    };

    let mut proto_name: Option<(Span, String)> = None;
    let mut oneof_name: Option<(Span, String)> = None;
    for entry in &entries {
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
    /// `variant` is `None` when it named the member but could not be resolved. The member is still
    /// covered: the author did write a variant for it, so reporting the enum as leaving it uncovered
    /// on top of the real error would make one mistake read as two. Boxed because the payload dwarfs
    /// the other outcome, and this is a transient per-variant value.
    Member {
        position: usize,
        variant: Option<Box<OneofVariant>>,
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
    sibling_metas: &[&FieldMeta],
    sibling_bindings: &mut [Option<(syn::Ident, syn::Type)>],
    absorbs: &mut Vec<String>,
    errors: &mut Errors,
) -> Option<VariantOutcome> {
    let span = variant.ident.span();
    let (
        FieldAttrs {
            rename,
            with,
            present,
            inline,
            absorbs: declared,
            ..
        },
        _,
    ) = scan_attrs(
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
        carried(
            &variant.fields,
            sibling_metas,
            sibling_bindings,
            absorbs,
            errors,
            span,
            &selected.proto_name,
        )?
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
    if member.is_none() && carried.is_empty() && !present && rename.is_none() {
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

    let ctx = VariantCtx {
        variant,
        field_meta,
        index,
        span,
        proto_name: &selected.proto_name,
        proto_path: &proto_path,
        member_name: &member_name,
        present,
        inline,
    };
    let (access, codec, checks) = match resolve_variant(
        &ctx,
        carried.into_leftovers(),
        sibling_metas,
        with,
        absorbs,
        errors,
    ) {
        Ok(resolved) => resolved,
        Err(()) => {
            return Some(VariantOutcome::Member {
                position,
                variant: None,
            })
        }
    };

    Some(VariantOutcome::Member {
        position,
        variant: Some(Box::new(OneofVariant {
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
        })),
    })
}

pub(crate) fn oneof_plan(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
) -> Result<OneofPlan, Errors> {
    let mut errors = Errors::new();

    let Ok(selected) = select_oneof(input, index, &mut errors) else {
        return Err(errors);
    };

    // Non-oneof fields of a whole-message enum, replicated in every variant.
    let sibling_metas: Vec<&FieldMeta> = if selected.whole_message {
        selected
            .meta
            .fields
            .iter()
            .filter(|field| field.oneof.is_none())
            .collect()
    } else {
        Vec::new()
    };
    // Rust-side binding of each sibling (ident + type), fixed by the first variant that declares it
    // and checked for consistency in the others.
    let mut sibling_bindings: Vec<Option<(syn::Ident, syn::Type)>> =
        (0..sibling_metas.len()).map(|_| None).collect();

    let syn::Data::Enum(data) = &input.data else {
        errors.at(
            input.ident.span(),
            "#[armonik(oneof = ...)] expects an enum",
        );
        return Err(errors);
    };

    let mut variants = Vec::new();
    let mut default_variant: Option<syn::Ident> = None;
    let mut covered = vec![false; selected.oneof.fields.len()];
    // Messages no Rust type stands for: the ones inlined into struct variants, and the ones a
    // `with` adapter flattens away, declared through `#[armonik(absorbs = "...")]`.
    let mut absorbs: Vec<String> = Vec::new();
    for variant in &data.variants {
        match resolve_one_variant(
            variant,
            &selected,
            index,
            &sibling_metas,
            &mut sibling_bindings,
            &mut absorbs,
            &mut errors,
        ) {
            Some(VariantOutcome::Member { position, variant }) => {
                covered[position] = true;
                variants.extend(variant.map(|variant| *variant));
            }
            // At most one of them, which is a fact about the enum rather than about any one
            // variant, so it is checked here rather than by the resolver.
            Some(VariantOutcome::NoMemberSet)
                if default_variant.replace(variant.ident.clone()).is_some() =>
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

    let siblings = collect_siblings(&sibling_metas, &sibling_bindings, &selected.proto_name);
    variants.sort_by_key(|variant| variant.own.tag);
    Ok(OneofPlan {
        ident: input.ident.clone(),
        docs: selected.meta.docs.clone(),
        oneof_path: (!selected.whole_message)
            .then(|| format!("{}.{}", selected.proto_name, selected.oneof.name)),
        proto_name: selected.proto_name,
        whole_message: selected.whole_message,
        siblings,
        variants,
        default_variant,
        fingerprint: index.fingerprint,
        absorbs,
    })
}

/// Emission for oneof-shaped enums: one `prost::Message` impl either way, plus registration and the
/// `Msg` marker when the enum stands for a whole message. With sibling fields (non-oneof fields of
/// a whole-message enum), every variant carries all of them, the "no member set" default included,
/// which keeps the per-field merge stateless and order-independent: a sibling occurrence merges
/// into the current variant's slot, a member occurrence switches variants while carrying the
/// siblings over. A sibling-free enum is the degenerate case with an empty sibling list.
pub(crate) fn oneof(plan: &OneofPlan) -> TokenStream {
    let ident = &plan.ident;
    let proto_name = &plan.proto_name;

    // Sibling machinery (empty and inert without siblings): all-variant patterns binding a subset
    // of the siblings, plus the sibling fields' fragments. Every variant carries every sibling, so
    // the fragments are emitted once *around* the member match rather than inside each of its arms,
    // and the arms only have to deal with the member.
    let sib_idents: Vec<&syn::Ident> = plan
        .siblings
        .iter()
        .map(|sibling| match sibling.access.as_ref() {
            Some(FieldAccess::Named(ident)) => ident,
            _ => unreachable!("a sibling is a named field of every variant"),
        })
        .collect();
    // Bound under `__f<tag>`, never under the user's field name: these locals sit in the same scope
    // as the emitter's own `buf`, `len`, `value`, `tag`, `wire_type` and `ctx`, and a proto field
    // named like any of those would otherwise shadow one.
    let sib_locals: Vec<syn::Ident> = plan.siblings.iter().map(slot_local).collect();
    let variant_idents: Vec<&syn::Ident> = plan
        .variants
        .iter()
        .map(|variant| &variant.ident)
        .chain(plan.default_variant.iter())
        .collect();
    // `bound` selects which siblings the pattern binds, by index.
    let pats = |bound: &[usize]| -> Vec<TokenStream> {
        let fields = bound.iter().map(|&i| sib_idents[i]);
        let locals = bound.iter().map(|&i| &sib_locals[i]);
        let binds: Vec<TokenStream> = fields
            .zip(locals)
            .map(|(field, local)| quote!(#field: #local))
            .collect();
        variant_idents
            .iter()
            .map(|variant| quote!(Self::#variant { #(#binds,)* .. }))
            .collect()
    };
    let all_siblings: Vec<usize> = (0..plan.siblings.len()).collect();
    let all_pats = pats(&all_siblings);
    // Binds every sibling by reference, whatever the variant.
    let bind_siblings = (!sib_locals.is_empty()).then(|| {
        let all = pats(&all_siblings);
        quote! {
            #[allow(unused_parens)]
            let (#(#sib_locals),*) = match value {
                #(#all)|* => (#(#sib_locals),*),
            };
        }
    });
    // Ascending tags across the whole message: the siblings below the oneof's tags are written
    // before the member, the ones above it after. (The shapes the derive accepts never interleave
    // the two.)
    let min_member_tag = plan.variants.iter().map(|variant| variant.own.tag).min();
    let (low, high): (Vec<_>, Vec<_>) = plan
        .siblings
        .iter()
        .zip(&sib_locals)
        .map(|(sibling, local)| {
            let written = slot_write(sibling, &quote!(#local));
            (sibling.tag, written.encode, written.len)
        })
        .partition(|(tag, _, _)| min_member_tag.is_some_and(|member| *tag < member));
    let sib_encode = |entries: &[(u32, TokenStream, TokenStream)]| -> Vec<TokenStream> {
        entries
            .iter()
            .map(|(_, encode, _)| encode.clone())
            .collect()
    };
    let sib_len = |entries: &[(u32, TokenStream, TokenStream)]| -> Vec<TokenStream> {
        entries
            .iter()
            .map(|(_, _, len)| quote! { len += #len; })
            .collect()
    };
    let (low_encode, low_len) = (sib_encode(&low), sib_len(&low));
    let (high_encode, high_len) = (sib_encode(&high), sib_len(&high));

    let mut encode_arms = Vec::new();
    let mut len_arms = Vec::new();
    let mut merge_arms = Vec::new();
    let mut asserts = TokenStream::new();
    let mut normalize_fragments = Vec::new();

    for sibling in &plan.siblings {
        asserts.extend(slot_asserts(sibling, ident));
    }

    for variant in &plan.variants {
        let own = &variant.own;
        asserts.extend(slot_asserts(own, ident));
        let ctx = EmitCtx {
            var: &variant.ident,
            own,
            sib_idents: &sib_idents,
            sib_locals: &sib_locals,
            take_pats: &all_pats,
        };
        let arms = match &own.codec {
            SlotCodec::Field { .. } => emit_payload_variant(&ctx),
            SlotCodec::Marker { empty_message } => emit_marker_variant(&ctx, *empty_message),
            SlotCodec::Inline { parts } => emit_inline_variant(&ctx, parts),
            // A variant carries one member, never a whole oneof: an enum standing for a oneof *is*
            // the flattened form, so there is nothing left to flatten.
            SlotCodec::Oneof { .. } => {
                unreachable!("a oneof variant carries a member, not another oneof")
            }
        };
        encode_arms.push(arms.encode);
        len_arms.push(arms.len);
        merge_arms.push(arms.merge);
        normalize_fragments.extend(arms.normalize);
    }

    // A sibling occurrence merges in place, whatever the current variant.
    for (position, sibling) in plan.siblings.iter().enumerate() {
        let local = &sib_locals[position];
        let sty = sibling.ty().expect("a sibling carries a value");
        let stag = sibling.tag;
        let self_pats = pats(&[position]);
        merge_arms.push(quote! {
            #stag => {
                match value {
                    #(#self_pats)|* => {
                        <#sty as crate::codec::ProtoField>::merge_field(wire_type, #local, buf, ctx)
                    }
                }
            }
        });
    }

    // The "no member set" variant has no member to write; its siblings are written outside the
    // match like every other variant's. `{ .. }` matches whatever shape the variant has (unit, or
    // carrying the siblings).
    let default_encode_arm = plan
        .default_variant
        .as_ref()
        .map(|var| quote! { Self::#var { .. } => {} });
    let default_len_arm = plan
        .default_variant
        .as_ref()
        .map(|var| quote! { Self::#var { .. } => 0, });

    // A whole-message enum is additionally the message itself: it registers and gets the `Msg`
    // marker, which is what makes it usable as an RPC message and as a field of another message. An
    // embedded oneof is a fragment of a message rather than one, so it gets neither. Its
    // `prost::Message` impl is the same either way; nothing is layered on top, because there is
    // nothing left to add. The old forwarding layer wrapped the same match in a second one whose
    // default arm was `skip_field`, which the inner match already ends with. `Normalize` is emitted
    // both ways: the containing message's delegates to it, since the members live on the parent's
    // dynamic message.
    //
    // `let value = self;` is the whole cost of sharing the bodies: they are written against a
    // `value` binding, and `prost::Message` takes a receiver where the deleted `ProtoOneof` took an
    // argument.
    //
    // The `Oneof` marker goes on the embedded shape only, and says which oneof this stands for: a
    // whole-message enum is a message and says so through `Msg::NAMES` already.
    let marker = plan.oneof_path.as_ref().map(|path| {
        quote! {
            impl crate::codec::Oneof for #ident {
                const ONEOF: &'static [&'static str] = &[#path];
            }
        }
    });
    let expansion = message_shaped(
        ident,
        &syn::Generics::default(),
        plan.fingerprint,
        std::slice::from_ref(proto_name),
        plan.whole_message,
        asserts,
        MessageBodies {
            encode_raw: quote! {
                let value = self;
                #bind_siblings
                #(#low_encode)*
                match value {
                    #(#encode_arms)*
                    #default_encode_arm
                }
                #(#high_encode)*
            },
            merge_field: quote! {
                let value = self;
                match tag {
                    #(#merge_arms)*
                    _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            },
            encoded_len: quote! {
                let value = self;
                #bind_siblings
                let mut len = match value {
                    #(#len_arms)*
                    #default_len_arm
                };
                #(#low_len)*
                #(#high_len)*
                len
            },
            normalize: normalize_fragments,
        },
    );

    quote! {
        #expansion
        #marker
    }
}

/// The arms one variant contributes: one to each of the encode, length and merge walks, plus the
/// `Normalize` projection its representation implies.
struct VariantArms {
    encode: TokenStream,
    len: TokenStream,
    merge: TokenStream,
    normalize: Option<TokenStream>,
}

/// What a variant emitter needs about the enum around it: the variant, the slot it owns, and the
/// shared fields every arm has to carry along.
///
/// Resolution already names its three cases (`resolve_variant`, `resolve_marker_variant`,
/// `resolve_inline_member`); these are the same three on the emission side, so a shape can be read
/// end to end without unpicking one long match.
struct EmitCtx<'a> {
    var: &'a syn::Ident,
    own: &'a Slot,
    sib_idents: &'a [&'a syn::Ident],
    sib_locals: &'a [syn::Ident],
    /// All-variant patterns binding every shared field, for the take-and-rebuild merge.
    take_pats: &'a [TokenStream],
}

/// A variant carrying the member whole: `Variant(T)`, or one named field of a struct variant.
fn emit_payload_variant(ctx: &EmitCtx<'_>) -> VariantArms {
    let EmitCtx {
        var,
        own,
        sib_idents,
        sib_locals,
        take_pats,
    } = ctx;
    let tag = own.tag;

    let merge = slot_merge_in_place(own, &quote!(&mut payload));
    let binding = match own.access.as_ref() {
        Some(FieldAccess::Named(field)) => Some(field),
        _ => None,
    };

    // The active member carries the oneof's presence, so it is emitted even with a
    // default payload, like every other field.
    let written = slot_write(own, &quote!(payload));
    let (encode, len) = (written.encode, written.len);
    let normalize = written.normalize;

    // Matching binds the member as `payload` and ignores the siblings; constructing one
    // needs them, so merging a member takes them along.
    let pattern = match binding {
        None => quote!(Self::#var(payload)),
        Some(field) => quote!(Self::#var { #field: payload, .. }),
    };
    let (construct, take) = match binding {
        None => (quote!(Self::#var(payload)), None),
        Some(field) => (
            quote!(Self::#var { #field: payload, #(#sib_idents: #sib_locals),* }),
            Some(quote! {
                #[allow(unused_parens)]
                let (#(#sib_locals),*) = match value {
                    #(#take_pats)|* => (#(::std::mem::take(#sib_locals)),*),
                };
            }),
        ),
    };

    let encode = quote! { #pattern => { #encode } };
    let len = quote! { #pattern => #len, };
    let merge_arm = quote! {
        #tag => {
            #take
            let mut payload = if let #pattern = value {
                ::std::mem::take(payload)
            } else {
                ::core::default::Default::default()
            };
            #merge?;
            *value = #construct;
            ::core::result::Result::Ok(())
        }
    };

    VariantArms {
        encode,
        len,
        merge: merge_arm,
        normalize,
    }
}

/// A `#[armonik(present)]` variant: the member is carried by its presence alone.
fn emit_marker_variant(ctx: &EmitCtx<'_>, empty_message: bool) -> VariantArms {
    let var = ctx.var;
    let tag = ctx.own.tag;
    if empty_message {
        let encode = quote! {
            Self::#var => {
                crate::codec::empty_body::encode(#tag, buf);
            }
        };
        let len = quote! {
            Self::#var => crate::codec::empty_body::encoded_len(#tag),
        };
        let merge_arm = quote! {
            #tag => {
                crate::codec::empty_body::merge(wire_type, buf, ctx)?;
                *value = Self::#var;
                ::core::result::Result::Ok(())
            }
        };
        VariantArms {
            encode,
            len,
            merge: merge_arm,
            normalize: None,
        }
    } else {
        // Only the member's presence survives (an explicit `false` reads as set).
        let normalize = Some(quote! {
            crate::differential::bool_marker(message, #tag);
        });
        let encode = quote! {
            Self::#var => {
                <bool as crate::codec::ProtoField>::encode_field(#tag, &true, buf);
            }
        };
        let len = quote! {
            Self::#var => <bool as crate::codec::ProtoField>::encoded_len_field(#tag, &true),
        };
        let merge_arm = quote! {
            #tag => {
                let mut marker = false;
                <bool as crate::codec::ProtoField>::merge_field(
                    wire_type, &mut marker, buf, ctx,
                )?;
                *value = Self::#var;
                ::core::result::Result::Ok(())
            }
        };
        VariantArms {
            encode,
            len,
            merge: merge_arm,
            normalize,
        }
    }
}

/// A `#[armonik(inline)]` variant: the member message's own fields, spread into it and framed here.
fn emit_inline_variant(ctx: &EmitCtx<'_>, parts: &[Slot]) -> VariantArms {
    let var = ctx.var;
    let own = ctx.own;
    let tag = own.tag;

    let part_idents: Vec<&syn::Ident> = parts
        .iter()
        .map(|part| match part.access.as_ref() {
            Some(FieldAccess::Named(ident)) => ident,
            _ => unreachable!("an inlined part is a named field of the variant"),
        })
        .collect();
    // Bound under `__f<tag>` like the shared fields, and for the same reason: these
    // locals share a scope with `buf`, `value`, `parts` and `body_len`.
    let part_locals: Vec<syn::Ident> = parts.iter().map(slot_local).collect();
    let part_tys: Vec<&syn::Type> = parts
        .iter()
        .map(|part| part.ty().expect("an inlined part carries a value"))
        .collect();
    let part_tags: Vec<u32> = parts.iter().map(|part| part.tag).collect();
    let part_seeds: Vec<_> = parts
        .iter()
        .map(|part| {
            let ty = part.ty().expect("an inlined part carries a value");
            quote!(<#ty as ::core::default::Default>::default())
        })
        .collect();
    // The parts are ordinary fields, written by the shared walk against the bindings
    // this arm's pattern introduces; only the framing around them is hand-rolled, since
    // the member message is absorbed and has no Rust type to delegate to.
    // No value expression of its own: an inlined member names its parts through the
    // bindings this arm's pattern introduces.
    let written = slot_write(own, &TokenStream::new());
    let (encode, len) = (written.encode, written.len);

    let encode = quote! {
        Self::#var { #(#part_idents: #part_locals),* } => { #encode }
    };
    let len = quote! {
        Self::#var { #(#part_idents: #part_locals),* } => #len,
    };
    let merge_arm = quote! {
        #tag => {
            ::prost::encoding::check_wire_type(
                ::prost::encoding::WireType::LengthDelimited,
                wire_type,
            )?;
            #[allow(unused_parens)]
            let (#(mut #part_locals),*) =
                if let Self::#var { #(#part_idents: #part_locals),* } = value {
                    (#(::std::mem::take(#part_locals)),*)
                } else {
                    (#(#part_seeds),*)
                };
            // Through prost's own framing, which brings the recursion and length
            // limits `ctx` carries and rejects a body that runs past its declared end.
            #[allow(unused_parens)]
            let mut __parts = (#(#part_locals),*);
            ::prost::encoding::merge_loop(
                &mut __parts,
                buf,
                ctx,
                |__parts, buf, ctx| {
                    let (tag, wire_type) = ::prost::encoding::decode_key(buf)?;
                    #[allow(unused_parens)]
                    let (#(#part_locals),*) = __parts;
                    match tag {
                        #(
                            #part_tags => <#part_tys as crate::codec::ProtoField>::merge_field(
                                wire_type, #part_locals, buf, ctx,
                            ),
                        )*
                        _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
                    }
                },
            )?;
            #[allow(unused_parens)]
            let (#(#part_locals),*) = __parts;
            *value = Self::#var { #(#part_idents: #part_locals),* };
            ::core::result::Result::Ok(())
        }
    };

    VariantArms {
        encode,
        len,
        merge: merge_arm,
        normalize: None,
    }
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

    /// A variant's fields are bound under `__f<tag>`, never under the name the user gave them.
    ///
    /// They share a scope with the emitter's own `buf`, `len`, `value` and `body_len`, so a proto
    /// field named like one of those would shadow it: not a wrong encoding but an unimplementable
    /// message, whose errors point into expanded code. `fixture.Hostile` is named to collide.
    #[test]
    fn variant_fields_are_bound_out_of_the_way() {
        let index = fixture_index();
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
        let plan = match oneof_plan(&input, &index) {
            Ok(plan) => plan,
            Err(errors) => panic!("the fixture resolves: {}", errors.into_syn_error()),
        };
        let emitted = oneof(&plan).to_string();

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

    /// An embedded oneof records which oneof it stands for; a whole-message enum does not.
    ///
    /// Not a `trybuild` case: this fires at const-eval against the real `codec`, which the
    /// compile-fail suite deliberately does not host (see `tests/ui.rs`). Pinned at the token level
    /// instead, which is what the suite's excluded classes get.
    #[test]
    fn an_embedded_oneof_records_the_oneof_it_stands_for() {
        let index = fixture_index();

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
        let plan = match oneof_plan(&embedded, &index) {
            Ok(plan) => plan,
            Err(errors) => panic!("the fixture resolves: {}", errors.into_syn_error()),
        };
        let emitted = oneof(&plan).to_string();
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
        let plan = match oneof_plan(&whole, &index) {
            Ok(plan) => plan,
            Err(errors) => panic!("the fixture resolves: {}", errors.into_syn_error()),
        };
        let emitted = oneof(&plan).to_string();
        assert!(
            !emitted.contains("crate :: codec :: Oneof for"),
            "a whole-message enum gets no oneof marker: {emitted}",
        );
    }
}
