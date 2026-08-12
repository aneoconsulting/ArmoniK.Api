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
    message_impl, msg_impl, normalize_impl, registrations, slot_asserts, slot_local,
    slot_merge_in_place, slot_write, tripwire,
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
    errors: &mut Errors,
    variant_span: Span,
    proto_name: &str,
) -> Option<Vec<Leftover>> {
    let mut failed = false;
    let mut seen = vec![false; sibling_metas.len()];
    let mut leftovers: Vec<Leftover> = Vec::new();

    for field in &named.named {
        let ident = field.ident.clone().expect("named fields have idents");
        let Some((FieldAttrs { rename, with, .. }, _)) = scan_attrs(
            &field.attrs,
            Allowed {
                rename: true,
                with: true,
                absorbs: true,
                ..Allowed::default()
            },
            "this armonik attribute is not valid on a struct variant field",
            errors,
        ) else {
            failed = true;
            continue;
        };

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
            failed = true;
        }
    }

    (!failed).then_some(leftovers)
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
    sibling_metas: &[&FieldMeta],
    sibling_bindings: &mut [Option<(syn::Ident, syn::Type)>],
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
        syn::Fields::Named(named) => {
            if let Some((with_span, _)) = &with {
                errors.at(
                    *with_span,
                    "in a struct variant, put with = ... on the field carrying the member",
                );
                return Err(());
            }
            let Some(leftovers) = split_variant_fields(
                named,
                sibling_metas,
                sibling_bindings,
                errors,
                ctx.span,
                ctx.proto_name,
            ) else {
                return Err(());
            };

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
        });
    }
    matcher.check_complete(ctx.span, errors);
    parts.sort_by_key(|part| part.tag);
    absorbs.push(inner_name.clone());
    Ok((None, SlotCodec::Inline { parts }, None))
}

pub(crate) fn oneof_plan(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
) -> Result<OneofPlan, Errors> {
    let mut errors = Errors::new();

    let entries = attrs::parse(&input.attrs)?;

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
        return Err(errors);
    };

    let Some(meta) = index.messages.get(&proto_name) else {
        errors.push(not_found(message_span, "message", &proto_name));
        return Err(errors);
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
                return Err(errors);
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
                return Err(errors);
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
                return Err(errors);
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
                return Err(errors);
            }
        },
    };
    // Non-oneof fields of a whole-message enum, replicated in every variant.
    let sibling_metas: Vec<&FieldMeta> = if whole_message {
        meta.fields
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
                inline,
                ..
            },
            _,
        )) = scan_attrs(
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
            &mut errors,
        )
        else {
            continue;
        };

        // The attribute-less unit variant is "no member set"; with sibling fields, that case is a
        // struct variant carrying exactly them and is detected below, after member-name matching
        // fails.
        if matches!(variant.fields, syn::Fields::Unit)
            && !present
            && rename.is_none()
            && sibling_metas.is_empty()
        {
            if default_variant.replace(variant.ident.clone()).is_some() {
                errors.at(
                    span,
                    "at most one attribute-less unit variant (the \"no member set\" case) \
                     is allowed",
                );
            }
            continue;
        }

        let member_name = rename
            .clone()
            .unwrap_or_else(|| crate::names::snake_case(&unraw(&variant.ident)));
        let member = oneof
            .fields
            .iter()
            .enumerate()
            .find_map(|(position, &field)| {
                (meta.fields[field].name == member_name).then_some((position, &meta.fields[field]))
            });
        // A struct variant whose fields are *all* siblings names no member: it is the "no member
        // set" case, the struct-variant twin of the attribute-less unit variant above.
        if member.is_none() && !sibling_metas.is_empty() && !present && rename.is_none() {
            if let syn::Fields::Named(named) = &variant.fields {
                match split_variant_fields(
                    named,
                    &sibling_metas,
                    &mut sibling_bindings,
                    &mut errors,
                    span,
                    &proto_name,
                ) {
                    Some(leftovers) if leftovers.is_empty() => {
                        if default_variant.replace(variant.ident.clone()).is_some() {
                            errors.at(
                                span,
                                "at most one attribute-less variant (the \"no member \
                                 set\" case) is allowed",
                            );
                        }
                        continue;
                    }
                    // Something is left over, so the variant does mean to name a member: fall
                    // through to the unknown-member error below.
                    Some(_) => {}
                    None => continue,
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
            inline,
        };
        let (access, codec, checks) = match resolve_variant(
            &ctx,
            &sibling_metas,
            &mut sibling_bindings,
            with,
            &mut absorbs,
            &mut errors,
        ) {
            Ok(resolved) => resolved,
            Err(()) => continue,
        };

        variants.push(OneofVariant {
            ident: variant.ident.clone(),
            own: Slot {
                access,
                span,
                tag: field_meta.tag,
                codec,
                checks,
                proto_path,
            },
        });
    }

    for (position, member_covered) in covered.iter().enumerate() {
        if !member_covered {
            let field = &meta.fields[oneof.fields[position]];
            errors.at(
                input.ident.span(),
                format!(
                    "oneof member `{proto_name}.{}` (tag {}) is not covered by any variant",
                    field.name, field.tag
                ),
            );
        }
    }

    errors.into_result()?;

    let mut siblings = Vec::new();
    for (meta_field, binding) in sibling_metas.iter().zip(&sibling_bindings) {
        // Missing bindings are only possible when every variant errored; those errors were reported
        // above.
        let Some((ident, ty)) = binding else { continue };
        siblings.push(Slot {
            span: ident.span(),
            access: Some(FieldAccess::Named(ident.clone())),
            tag: meta_field.tag,
            codec: SlotCodec::Field {
                ty: Box::new(ty.clone()),
                adapter: None,
            },
            proto_path: format!("{proto_name}.{}", meta_field.name),
            checks: Expectation::of(meta_field),
        });
    }
    siblings.sort_by_key(|sibling| sibling.tag);

    variants.sort_by_key(|variant| variant.own.tag);
    Ok(OneofPlan {
        ident: input.ident.clone(),
        proto_name,
        whole_message,
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
    let fingerprint = proc_macro2::Literal::u64_suffixed(plan.fingerprint);

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
        let var = &variant.ident;
        let own = &variant.own;
        let tag = own.tag;
        asserts.extend(slot_asserts(own, ident));
        match &own.codec {
            SlotCodec::Field { .. } => {
                let merge = slot_merge_in_place(own, &quote!(&mut payload));
                let binding = match own.access.as_ref() {
                    Some(FieldAccess::Named(field)) => Some(field),
                    _ => None,
                };

                // The active member carries the oneof's presence, so it is emitted even with a
                // default payload, like every other field.
                let written = slot_write(own, &quote!(payload));
                let (encode, len) = (written.encode, written.len);
                normalize_fragments.extend(written.normalize);

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
                        Some({
                            let take_pats = pats(&all_siblings);
                            quote! {
                                #[allow(unused_parens)]
                                let (#(#sib_locals),*) = match value {
                                    #(#take_pats)|* => (#(::std::mem::take(#sib_locals)),*),
                                };
                            }
                        }),
                    ),
                };

                encode_arms.push(quote! { #pattern => { #encode } });
                len_arms.push(quote! { #pattern => #len, });
                merge_arms.push(quote! {
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
                });
            }
            SlotCodec::Marker {
                empty_message: false,
            } => {
                // Only the member's presence survives (an explicit `false` reads as set).
                normalize_fragments.push(quote! {
                    crate::differential::bool_marker(message, #tag);
                });
                encode_arms.push(quote! {
                    Self::#var => {
                        <bool as crate::codec::ProtoField>::encode_field(#tag, &true, buf);
                    }
                });
                len_arms.push(quote! {
                    Self::#var => <bool as crate::codec::ProtoField>::encoded_len_field(#tag, &true),
                });
                merge_arms.push(quote! {
                    #tag => {
                        let mut marker = false;
                        <bool as crate::codec::ProtoField>::merge_field(
                            wire_type, &mut marker, buf, ctx,
                        )?;
                        *value = Self::#var;
                        ::core::result::Result::Ok(())
                    }
                });
            }
            SlotCodec::Marker {
                empty_message: true,
            } => {
                encode_arms.push(quote! {
                    Self::#var => {
                        crate::codec::empty_body::encode(#tag, buf);
                    }
                });
                len_arms.push(quote! {
                    Self::#var => crate::codec::empty_body::encoded_len(#tag),
                });
                merge_arms.push(quote! {
                    #tag => {
                        ::prost::encoding::skip_field(wire_type, tag, buf, ctx)?;
                        *value = Self::#var;
                        ::core::result::Result::Ok(())
                    }
                });
            }
            // A variant carries one member, never a whole oneof: an enum standing for a oneof
            // *is* the flattened form, so there is nothing left to flatten.
            SlotCodec::Oneof { .. } => {
                unreachable!("a oneof variant carries a member, not another oneof")
            }
            SlotCodec::Inline { parts } => {
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

                encode_arms.push(quote! {
                    Self::#var { #(#part_idents: #part_locals),* } => { #encode }
                });
                len_arms.push(quote! {
                    Self::#var { #(#part_idents: #part_locals),* } => #len,
                });
                merge_arms.push(quote! {
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
                });
            }
        }
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

    let generics = syn::Generics::default();

    // A whole-message enum is additionally the message itself: it registers and gets the `Msg`
    // marker, which is what makes it usable as an RPC message and as a field of another message.
    // The `prost::Message` impl below is shared with the embedded case; nothing is layered on top
    // of it, because there is nothing left to add. The old forwarding layer wrapped the same match
    // in a second one whose default arm was `skip_field`, which the inner match already ends with.
    let whole_message = plan.whole_message.then(|| {
        let registrations = registrations(ident, std::slice::from_ref(&plan.proto_name));
        let msg = msg_impl(&generics, ident, std::slice::from_ref(proto_name));
        quote! {
            #registrations

            #msg
        }
    });

    // Emitted for embedded oneofs too: the containing message's `Normalize` delegates to it (the
    // members live on the parent's dynamic message).
    let normalize = normalize_impl(&generics, ident, &normalize_fragments);

    // `let value = self;` is the whole cost of the change: the emitted bodies are written against a
    // `value` binding, and `prost::Message` takes a receiver where the deleted `ProtoOneof` took an
    // argument.
    let message = message_impl(
        &generics,
        ident,
        quote! {
            let value = self;
            #bind_siblings
            #(#low_encode)*
            match value {
                #(#encode_arms)*
                #default_encode_arm
            }
            #(#high_encode)*
        },
        quote! {
            let value = self;
            match tag {
                #(#merge_arms)*
                _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        },
        quote! {
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
        None,
    );

    let tripwire = tripwire(&fingerprint);
    quote! {
        const _: () = {
            #tripwire
            #asserts
        };

        #normalize

        #message

        #whole_message
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
    fn fixture_index() -> std::sync::Arc<DescriptorIndex> {
        use prost::Message as _;

        let dir = std::env::temp_dir().join("armonik-macros-oneof-fixture");
        std::fs::create_dir_all(&dir).expect("create the fixture directory");
        let descriptor = protox::compile(["tests/fixture.proto"], ["tests"])
            .expect("compile tests/fixture.proto")
            .encode_to_vec();
        std::fs::write(dir.join("descriptor.bin"), &descriptor).expect("write the descriptor set");
        std::env::set_var("OUT_DIR", &dir);
        crate::descriptor::index().expect("the fixture index loads")
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
}
