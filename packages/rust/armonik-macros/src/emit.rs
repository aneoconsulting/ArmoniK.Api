//! The one emitter every message-shaped type goes through, and the pieces it is built from.
//!
//! [`message`] reads an [`Ir`] and nothing else. A struct is the degenerate enum with no
//! discriminant, so there are exactly two body builders: [`struct_bodies`] writes the shared slots
//! from `self`, [`enum_bodies`] writes each arm's slots from the bindings its pattern introduces.
//! One field is written the same way wherever it sits -- a struct field, a shared sibling, an
//! active member and one part of an inlined member all reduce to [`slot_write`]. The rest is the
//! impl skeletons (`prost::Message`, `Msg`, `Normalize`), the registry call, the descriptor
//! fingerprint tripwire, and the per-field shape assert.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

use crate::descriptor::{Cardinality, FieldKind};
use crate::generator::Generator;
use crate::plan::{Arm, Discr, Expectation, FieldAccess, Ir, Slot, SlotCodec};

/// A field's key in braced syntax: its name, or its position for a tuple variant, whose fields are
/// named `0`, `1`, ... So one pattern and one constructor serve every variant shape.
impl quote::ToTokens for FieldAccess {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            FieldAccess::Named(ident) => quote::ToTokens::to_tokens(ident, tokens),
            FieldAccess::Indexed(index) => quote::ToTokens::to_tokens(index, tokens),
        }
    }
}

/// Everything one [`Ir`] expands to, besides the re-emitted item: the guard block (fingerprint
/// tripwire and shape asserts), the registry entries, the trait trio, and the mode extras (the
/// `GenericFields` table of a generic type, the `Oneof` identity marker of an embedded oneof).
///
/// Total: a poisoned plan is emitted too, with real signatures and placeholder bodies at the scope
/// the failure poisons (the whole wire impl of a struct whose field failed, one `unimplemented!()`
/// arm per failed variant of an enum), so the type and its impls resolve everywhere they are used
/// and the recorded errors are all the build fails with.
pub(crate) fn message(ir: &Ir, generator: &mut Generator) {
    let ident = &ir.ident;
    let generics = bound_generics(&ir.generics);

    let mut asserts = TokenStream::new();
    for slot in ir.shared.iter().chain(arm_slots(&ir.discr)) {
        asserts.extend(slot_asserts(slot, ident, &ir.names));
    }

    let bodies = match &ir.discr {
        // A struct's wire form has no correct partial spelling: an instance carries every field,
        // so one poisoned field (or a fully poisoned plan) poisons the whole body.
        None if ir.shared.iter().any(Slot::is_poisoned) => placeholder_bodies(),
        None => struct_bodies(&ir.shared),
        Some(discr) => enum_bodies(&ir.shared, discr),
    };

    // A generic type carries its fields' tags and instantiated shapes to wherever it is
    // instantiated, because it cannot be checked where it is declared: it names no proto message.
    // Every `#[armonik_macros::alias]` over it then asserts them against the message it registers
    // under. The `SHAPE`s are written against the type parameters, so each instantiation reports
    // its own.
    let generic_fields = ir.generic.then(|| {
        let (_, ty_generics, _) = ir.generics.split_for_impl();
        let entries = ir
            .shared
            .iter()
            .filter(|slot| !slot.is_poisoned())
            .map(|slot| {
                let tag = slot.tag;
                let dispatch = slot_dispatch(slot);
                quote! { (#tag, #dispatch::SHAPE) }
            });
        quote! {
            impl #generics crate::codec::GenericFields for #ident #ty_generics {
                const FIELDS: &'static [(u32, crate::codec::Shape)] = &[#(#entries),*];
            }
        }
    });

    // The `Oneof` marker goes on the embedded shape only, and says which oneof this stands for: a
    // whole-message enum is a message and says so through `Msg::NAMES` already. Without it,
    // substituting one filter family's `Condition` for another's compiles clean and passes the
    // whole harness, because the two are tag-compatible and the substitution is a byte-level
    // bijection.
    let marker = ir.fragment_of.as_ref().map(|path| {
        // The empty path is a poisoned fragment: an empty `ONEOF` is the unchecked case, which is
        // what keeps the carrying struct's identity assert from adding a second error.
        let paths = (!path.is_empty()).then_some(path).into_iter();
        quote! {
            impl crate::codec::Oneof for #ident {
                const ONEOF: &'static [&'static str] = &[#(#paths),*];
            }
        }
    });

    let mut expansion = message_shaped(
        ident,
        &generics,
        ir.fingerprint,
        &ir.names,
        ir.fragment_of.is_none(),
        generator.poisoned(),
        asserts,
        bodies,
    );
    expansion.extend(generic_fields);
    expansion.extend(marker);
    generator.emit(expansion);
}

/// The wire bodies of a type that has no correct ones: real signatures, so everything the type is
/// used by still resolves, and placeholder bodies, so even a build that somehow missed the
/// recorded error cannot encode wrong bytes silently.
pub(crate) fn placeholder_bodies() -> MessageBodies {
    let unimplemented = quote!(::core::unimplemented!());
    MessageBodies {
        encode_raw: quote! { let _ = buf; #unimplemented },
        merge_field: quote! { let _ = (tag, wire_type, buf, ctx); #unimplemented },
        encoded_len: unimplemented,
        normalize: Vec::new(),
    }
}

/// The slots the discriminant's arms own, for the walks that visit every slot of the plan.
fn arm_slots(discr: &Option<Discr>) -> impl Iterator<Item = &Slot> {
    discr
        .iter()
        .flat_map(|discr| discr.arms.iter().map(|arm| &arm.own))
}

/// The four bodies of a discriminant-less message: every slot is shared, written from `self` and
/// merged in place. A whole-message delegate (a `transparent` newtype's single field) supplies the
/// merge fallback instead of a tag arm, since every tag is its.
fn struct_bodies(slots: &[Slot]) -> MessageBodies {
    let mut encode_fragments = Vec::new();
    let mut merge_arms = Vec::new();
    let mut fallback = None;
    let mut len_fragments = Vec::new();
    let mut normalize = Vec::new();

    for slot in slots {
        let access = slot.access.as_ref().expect("a struct field is reachable");
        let written = slot_write(slot, &quote!(&self.#access), Presence::Implicit);
        encode_fragments.push(written.encode);
        let len = written.len;
        len_fragments.push(quote! { len += #len; });
        normalize.extend(written.normalize);

        let merge = slot_merge_in_place(slot, &quote!(&mut self.#access));
        match &slot.codec {
            SlotCodec::Delegate { tags: None, .. } => fallback = Some(merge),
            // A delegate answers to every one of its routed tags.
            SlotCodec::Delegate {
                tags: Some(tags), ..
            } => merge_arms.push(quote! { #(#tags)|* => #merge }),
            _ => {
                let tag = slot.tag;
                merge_arms.push(quote! { #tag => #merge });
            }
        }
    }
    let fallback =
        fallback.unwrap_or_else(|| quote!(::prost::encoding::skip_field(wire_type, tag, buf, ctx)));

    MessageBodies {
        encode_raw: quote! { #(#encode_fragments)* },
        merge_field: quote! {
            match tag {
                #(#merge_arms,)*
                _ => #fallback,
            }
        },
        encoded_len: quote! {
            #[allow(unused_mut)]
            let mut len = 0;
            #(#len_fragments)*
            len
        },
        normalize,
    }
}

/// The four bodies of an enum-shaped message. With shared slots (non-oneof fields of a
/// whole-message enum), every variant carries all of them, the "no member set" default included,
/// which keeps the per-field merge stateless and order-independent: a shared occurrence merges into
/// the current variant's slot, a member occurrence switches variants while carrying the shared
/// values over. A shared-free enum is the degenerate case with an empty shared list.
fn enum_bodies(shared: &[Slot], discr: &Discr) -> MessageBodies {
    // Poisoned variants contribute one `unimplemented!()` arm to every match over the value, so
    // the matches stay exhaustive over what the user wrote while binding nothing of it; the
    // brace-with-rest pattern matches a unit, tuple or struct variant alike. They answer to no
    // tag, so the tag matches need nothing.
    let (arms, poisoned): (Vec<&Arm>, Vec<&Arm>) =
        discr.arms.iter().partition(|arm| !arm.own.is_poisoned());
    let poison_arms: Vec<TokenStream> = poisoned
        .iter()
        .map(|arm| {
            let ident = &arm.ident;
            quote! { Self::#ident { .. } => ::core::unimplemented!(), }
        })
        .collect();

    // Shared machinery (empty and inert without shared slots): all-variant patterns binding a
    // subset of them, plus their fragments. Every variant carries every shared slot, so the
    // fragments are emitted once *around* the member match rather than inside each of its arms, and
    // the arms only have to deal with the member.
    let shared_idents: Vec<&syn::Ident> = shared
        .iter()
        .map(|slot| match slot.access.as_ref() {
            Some(FieldAccess::Named(ident)) => ident,
            _ => unreachable!("a shared slot is a named field of every variant"),
        })
        .collect();
    // Bound under `__f<tag>`, never under the user's field name: these locals sit in the same scope
    // as the emitter's own `buf`, `len`, `value`, `tag`, `wire_type` and `ctx`, and a proto field
    // named like any of those would otherwise shadow one.
    let shared_locals: Vec<syn::Ident> = shared.iter().map(slot_local).collect();
    let variant_idents: Vec<&syn::Ident> = arms
        .iter()
        .map(|arm| &arm.ident)
        .chain(discr.default_arm.iter())
        .collect();
    // `bound` selects which shared slots the pattern binds, by index.
    let pats = |bound: &[usize]| -> Vec<TokenStream> {
        let fields = bound.iter().map(|&i| shared_idents[i]);
        let locals = bound.iter().map(|&i| &shared_locals[i]);
        let binds: Vec<TokenStream> = fields
            .zip(locals)
            .map(|(field, local)| quote!(#field: #local))
            .collect();
        variant_idents
            .iter()
            .map(|variant| quote!(Self::#variant { #(#binds,)* .. }))
            .collect()
    };
    let all_shared: Vec<usize> = (0..shared.len()).collect();
    let all_pats = pats(&all_shared);
    // Arm-invariant: every variant carries every shared field, so the merge takes them out of
    // whatever is there before rebuilding the variant around the merged member.
    let take = (!shared_locals.is_empty()).then(|| {
        quote! {
            #[allow(unused_parens)]
            let (#(#shared_locals),*) = match value {
                #(#all_pats)|* => (#(::std::mem::take(#shared_locals)),*),
                #(#poison_arms)*
            };
        }
    });
    let ctx = EmitCtx {
        shared,
        shared_idents: &shared_idents,
        shared_locals: &shared_locals,
        take,
    };
    let mut encode_arms = Vec::new();
    let mut len_arms = Vec::new();
    let mut merge_arms = Vec::new();
    let mut normalize = Vec::new();

    for arm in &arms {
        let tokens = emit_arm(&ctx, &arm.ident, Some(&arm.own));
        encode_arms.push(tokens.encode);
        len_arms.push(tokens.len);
        merge_arms.extend(tokens.merge);
        normalize.extend(tokens.normalize);
    }

    // The "no member set" variant, through the same emitter: it owns no slot, so it writes only the
    // shared fields and is selected by no tag.
    if let Some(var) = discr.default_arm.as_ref() {
        let tokens = emit_arm(&ctx, var, None);
        encode_arms.push(tokens.encode);
        len_arms.push(tokens.len);
    }

    // A shared field's projection is a property of the field, not of the variant carrying it, so it
    // is collected once rather than per arm.
    for (slot, local) in shared.iter().zip(shared_locals.iter()) {
        normalize.extend(slot_write(slot, &quote!(#local), Presence::Implicit).normalize);
    }

    // A shared occurrence merges in place, whatever the current variant.
    for (position, slot) in shared.iter().enumerate() {
        let local = &shared_locals[position];
        let stag = slot.tag;
        // Only this slot is bound, so the others raise no unused binding; and through the shared
        // helper, so a shared field carrying a `with` adapter is merged through the adapter rather
        // than through its Rust type's own codec.
        let self_pats = pats(&[position]);
        let merge = slot_merge_in_place(slot, &quote!(#local));
        // The bound arm only exists where a variant resolved; with every variant poisoned, the
        // match is the poison arms alone.
        let bound = (!variant_idents.is_empty()).then(|| quote!(#(#self_pats)|* => { #merge }));
        merge_arms.push(quote! {
            #stag => {
                match value {
                    #bound
                    #(#poison_arms)*
                }
            }
        });
    }

    // `let value = self;` is the whole cost of sharing the bodies with the struct path's vocabulary:
    // they are written against a `value` binding, and `prost::Message` takes a receiver.
    MessageBodies {
        encode_raw: quote! {
            let value = self;
            match value {
                #(#encode_arms)*
                #(#poison_arms)*
            }
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
            match value {
                #(#len_arms)*
                #(#poison_arms)*
            }
        },
        normalize,
    }
}

/// The arms one variant contributes: one to each of the encode, length and merge walks, plus the
/// `Normalize` projection its representation implies.
struct ArmTokens {
    encode: TokenStream,
    len: TokenStream,
    /// `None` for the "no member set" variant, which no tag selects.
    merge: Option<TokenStream>,
    normalize: Option<TokenStream>,
}

/// What the arm emitter needs about the enum around it: the shared slots every arm writes
/// alongside its own, and the block that takes them out of whatever variant is there.
///
/// Enum-wide, so it is built once: an arm differs from its siblings only in the variant it names
/// and the slot it owns, which is what [`emit_arm`] takes as arguments.
struct EmitCtx<'a> {
    shared: &'a [Slot],
    shared_idents: &'a [&'a syn::Ident],
    shared_locals: &'a [syn::Ident],
    /// Taking every shared field out of the value before the member is merged, for a merge that
    /// rebuilds the variant. `None` without shared fields, where there is nothing to take.
    take: Option<TokenStream>,
}

/// One variant's arms, whatever it carries.
///
/// A variant binds its own slot's values -- none for a `present` marker, one for a member carried
/// whole, several for an inlined member's parts -- and then the message's shared fields. Everything
/// below is written against that one list, in braced form, so the shapes differ only in what they
/// put in it and in how [`slot_merge_in_place`] reads the member back. An empty list is not a
/// special case: it interpolates to nothing.
fn emit_arm(ctx: &EmitCtx<'_>, var: &syn::Ident, own: Option<&Slot>) -> ArmTokens {
    let EmitCtx {
        shared,
        shared_idents,
        shared_locals,
        take,
    } = ctx;
    // The variant's own bindings, and how the walks name the member's value where it has one: the
    // pattern binding for a bound slot, the unit for a presence marker (its adapters take `&()`
    // and bind nothing), nothing for a group, whose parts name themselves.
    type Own<'s> = (Vec<&'s Slot>, TokenStream, TokenStream);
    let (own_slots, value, place): Own<'_> = match own {
        None => Default::default(),
        Some(slot) => match (&slot.codec, &slot.access) {
            (SlotCodec::Group { parts }, _) => (
                parts.iter().collect(),
                TokenStream::new(),
                TokenStream::new(),
            ),
            (SlotCodec::Field { .. }, None) => (Vec::new(), quote!(&()), quote!(&mut ())),
            (SlotCodec::Field { .. }, Some(_)) => {
                let local = slot_local(slot);
                (vec![slot], quote!(#local), quote!(&mut #local))
            }
            // A variant carries one member, never a whole oneof or message: an enum standing for a
            // oneof *is* the flattened form, so there is nothing left to delegate to. Poisoned
            // arms are partitioned out before this is reached.
            (SlotCodec::Delegate { .. } | SlotCodec::Poisoned, _) => {
                unreachable!("a oneof variant carries a member, not a delegate")
            }
        },
    };
    let own_keys: Vec<&FieldAccess> = own_slots
        .iter()
        .map(|slot| {
            slot.access
                .as_ref()
                .expect("a bound slot is reached by a field")
        })
        .collect();
    let own_locals: Vec<syn::Ident> = own_slots.iter().copied().map(slot_local).collect();

    // Binding every field for the walks that write them, and only the member's for the merge, which
    // takes the shared fields out separately.
    let bind_all =
        quote!(Self::#var { #(#own_keys: #own_locals,)* #(#shared_idents: #shared_locals,)* });
    let bind_own = quote!(Self::#var { #(#own_keys: #own_locals,)* .. });
    let construct =
        quote!(Self::#var { #(#own_keys: #own_locals,)* #(#shared_idents: #shared_locals),* });

    // Every field this variant carries, in ascending tag order: the shared ones and, where it has
    // one, its member. Ordering here rather than around the match is what lets a shared field sit
    // between two members. The member carries the oneof's presence, so it writes whatever it holds;
    // a shared field is an ordinary field of the message and skips its zero.
    let mut writes: Vec<(u32, TokenStream, TokenStream)> = shared
        .iter()
        .zip(shared_locals.iter())
        .map(|(slot, local)| {
            let written = slot_write(slot, &quote!(#local), Presence::Implicit);
            (slot.tag, written.encode, written.len)
        })
        .collect();
    let mut normalize = None;
    if let Some(slot) = own {
        let written = slot_write(slot, &value, Presence::Explicit);
        writes.push((slot.tag, written.encode, written.len));
        normalize = written.normalize;
    }
    writes.sort_by_key(|(tag, _, _)| *tag);
    let encodes = writes.iter().map(|(_, encode, _)| encode);
    let lens = writes.iter().map(|(_, _, len)| len);

    // Merging the member rebuilds the variant, so it needs the member's own values seeded from
    // whatever is there, and the shared fields taken along.
    let seeds = own_slots.iter().map(|slot| {
        let ty = slot.ty().expect("a bound slot carries a value");
        quote!(<#ty as ::core::default::Default>::default())
    });
    let seed = (!own_locals.is_empty()).then(|| {
        quote! {
            #[allow(unused_parens)]
            let (#(mut #own_locals),*) = if let #bind_own = value {
                (#(::std::mem::take(#own_locals)),*)
            } else {
                (#(#seeds),*)
            };
        }
    });
    // The "no member set" variant is reached by no tag, so it contributes no merge arm.
    let merge = own.map(|slot| {
        let tag = slot.tag;
        let merge = slot_merge_in_place(slot, &place);
        quote! {
            #tag => {
                #take
                #seed
                #merge?;
                *value = #construct;
                ::core::result::Result::Ok(())
            }
        }
    });

    ArmTokens {
        encode: quote! { #bind_all => { #(#encodes)* } },
        len: quote! { #bind_all => 0 #(+ #lens)*, },
        merge,
        normalize,
    }
}

// ---- The pieces the emitter is built from ----

/// Runtime path of a descriptor kind, for const-assert patterns. `None` for the sint/fixed wire
/// kinds the codec does not implement (no ArmoniK field uses them); the caller turns that into an
/// "unsupported wire kind" error rather than naming a `codec::FieldKind` variant that does not
/// exist.
fn kind_pattern(kind: &FieldKind) -> Option<TokenStream> {
    let variant = match kind {
        FieldKind::Double => quote!(Double),
        FieldKind::Float => quote!(Float),
        FieldKind::Int32 => quote!(Int32),
        FieldKind::Int64 => quote!(Int64),
        FieldKind::UInt32 => quote!(UInt32),
        FieldKind::UInt64 => quote!(UInt64),
        FieldKind::Bool => quote!(Bool),
        FieldKind::String => quote!(String),
        FieldKind::Bytes => quote!(Bytes),
        FieldKind::Message(_) => quote!(Message),
        FieldKind::Enum(_) => quote!(Enum),
        FieldKind::Unsupported(_) => return None,
    };
    Some(quote!(crate::codec::FieldKind::#variant))
}

fn kind_description(kind: &FieldKind) -> String {
    match kind {
        FieldKind::Message(name) => format!("message {name}"),
        FieldKind::Enum(name) => format!("enum {name}"),
        FieldKind::Unsupported(name) => (*name).to_owned(),
        other => format!("{other:?}").to_lowercase(),
    }
}

/// The cardinalities a Rust type may use for a descriptor cardinality.
///
/// One rule that is not a restatement: a singular *message* field may be either plain in Rust
/// ("absent reads as default") or `Option` (presence is significant), so both are accepted.
fn cardinalities(expect: &Expectation) -> Vec<(TokenStream, &'static str)> {
    let one = |token, description| vec![(token, description)];
    match &expect.cardinality {
        Cardinality::Map { .. } => one(quote!(crate::codec::Cardinality::Map), "map"),
        Cardinality::Repeated => one(quote!(crate::codec::Cardinality::Repeated), "repeated"),
        Cardinality::Optional => one(
            quote!(crate::codec::Cardinality::Optional),
            "optional (explicit presence)",
        ),
        Cardinality::Singular if matches!(expect.kind, FieldKind::Message(_)) => vec![
            (quote!(crate::codec::Cardinality::Singular), "singular"),
            (
                quote!(crate::codec::Cardinality::Optional),
                "optional (explicit presence)",
            ),
        ],
        Cardinality::Singular => one(quote!(crate::codec::Cardinality::Singular), "singular"),
    }
}

/// The proto type a message- or enum-kind field names, which the assert checks the Rust type
/// against. Scalars name nothing and go unchecked.
fn type_name(kind: &FieldKind) -> Option<&str> {
    match kind {
        FieldKind::Message(name) | FieldKind::Enum(name) => Some(name),
        _ => None,
    }
}

/// Human form of the expected shape, for the assert message.
fn describe(expect: &Expectation) -> String {
    if let Cardinality::Map { key, value } = &expect.cardinality {
        return format!(
            "a map<{}, {}>",
            kind_description(key),
            kind_description(value)
        );
    }
    let cards = cardinalities(expect)
        .iter()
        .map(|(_, description)| *description)
        .collect::<Vec<_>>()
        .join(" or ");
    format!("{cards} {}", kind_description(&expect.kind))
}

/// The `crate::codec::Expect` literal for one descriptor field.
///
/// `Err` is the spanned compile error for a wire kind the codec does not implement, which is the
/// one thing that can go wrong building it. One home for this literal, because there are two
/// readers: a field of a validated message, and a field of a generic type checked at each of its
/// `#[armonik_macros::alias]` instantiations.
pub(crate) fn expect_literal(
    expect: &Expectation,
    proto_path: &str,
    span: proc_macro2::Span,
) -> Result<TokenStream, TokenStream> {
    // A map's own kind is not checked: what it is made of is, through `map`.
    let is_map = matches!(expect.cardinality, Cardinality::Map { .. });
    let kind_expr = if is_map {
        quote!(::core::option::Option::None)
    } else {
        match kind_pattern(&expect.kind) {
            Some(token) => quote!(::core::option::Option::Some(#token)),
            None => return Err(unsupported_kind_error(&expect.kind, proto_path, span)),
        }
    };
    let map_expr = match &expect.cardinality {
        Cardinality::Map { key, value } => match (kind_pattern(key), kind_pattern(value)) {
            (Some(key_token), Some(value_token)) => {
                quote!(::core::option::Option::Some((#key_token, #value_token)))
            }
            (key_token, _) => {
                let unsupported = if key_token.is_none() { key } else { value };
                return Err(unsupported_kind_error(unsupported, proto_path, span));
            }
        },
        _ => quote!(::core::option::Option::None),
    };
    // A map names its *value* type, everything else its own.
    let named = match &expect.cardinality {
        Cardinality::Map { value, .. } => type_name(value),
        _ => type_name(&expect.kind),
    };
    let name_expr = match named {
        Some(name) => quote!(::core::option::Option::Some(#name)),
        None => quote!(::core::option::Option::None),
    };
    let cards = cardinalities(expect).into_iter().map(|(token, _)| token);
    Ok(quote! {
        crate::codec::Expect {
            kind: #kind_expr,
            cardinalities: &[#(#cards),*],
            name: #name_expr,
            map: #map_expr,
        }
    })
}

/// One spanned shape assert per checked field: the field type's `SHAPE` against the descriptor's
/// `Expect`.
fn field_asserts_for(
    ty: &syn::Type,
    span: proc_macro2::Span,
    proto_path: &str,
    checks: &Option<Expectation>,
    type_ident: &syn::Ident,
) -> TokenStream {
    let Some(expect) = checks else {
        return TokenStream::new();
    };
    let literal = match expect_literal(expect, proto_path, span) {
        Ok(literal) => literal,
        Err(error) => return error,
    };
    let message = format!(
        "armonik: the Rust type of the field of `{type_ident}` mapping to proto field \
         `{proto_path}` does not have the expected shape ({})",
        describe(expect),
    );
    quote_spanned! {span=>
        assert!(
            crate::codec::shape_matches(
                &<#ty as crate::codec::ProtoField>::SHAPE,
                &#literal,
            ),
            #message
        );
    }
}

/// A spanned compile error for a proto field whose wire kind the codec does not implement (the
/// sint/fixed kinds; no ArmoniK field uses them, so `codec::FieldKind` omits them).
fn unsupported_kind_error(
    kind: &FieldKind,
    proto_path: &str,
    span: proc_macro2::Span,
) -> TokenStream {
    let message = format!(
        "armonik: proto field `{proto_path}` uses wire kind {}, which the codec does not implement",
        kind_description(kind),
    );
    quote_spanned! {span=> ::core::compile_error!(#message); }
}

/// Which of a slot's two entry points to write it through.
#[derive(Clone, Copy)]
enum Presence {
    /// An ordinary proto3 field: absent and zero are one value, so a zero is left out.
    Implicit,
    /// The field being there at all is what carries the information (a oneof member selects its
    /// variant), so it is written whatever it holds.
    Explicit,
}

/// Register the type's proto names via `armonik`'s `register!` macro, the single home of the
/// registry's layout (the `linkme` slice, the `cfg(test)` gate, the round-trip and
/// `Normalize` hooks). Empty `names` (generic types, covered through their aliases) register
/// nothing.
pub(crate) fn registrations(ident: &syn::Ident, names: &[String]) -> TokenStream {
    if names.is_empty() {
        return TokenStream::new();
    }
    quote! {
        crate::register!(message: #ident, #(#names),*);
    }
}

/// Register proto messages an absorbing construct swallows into its parent (an `inlined` field's
/// wrapper or pair layer, a transparent chain's middle wrappers, an inlined variant's message), so they
/// have no Rust type of their own, and the differential harness counts them as covered through
/// their parent.
pub(crate) fn absorbed_registrations(names: &[String]) -> TokenStream {
    if names.is_empty() {
        return TokenStream::new();
    }
    quote! {
        crate::register!(absorbed: #(#names),*);
    }
}

/// Test-only `Normalize` impl: the type's value-level projection for the differential harness,
/// stitched from the same constructs that shape the codec (adapters, presence markers, wrapper
/// chains, delegation).
fn normalize_impl(
    generics: &syn::Generics,
    ident: &syn::Ident,
    fragments: &[TokenStream],
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        #[cfg(test)]
        impl #impl_generics crate::differential::Normalize for #ident #ty_generics #where_clause {
            fn normalize(
                message: &mut ::prost_reflect::DynamicMessage,
            ) {
                let _ = &message;
                #(#fragments)*
            }
        }
    }
}

/// The compile-time tripwire: a const-assert that the descriptor fingerprint the derive was
/// expanded against still matches the schema baked into the crate. Emitted (in a `const _: () = {
/// ... };` block) by every derive.
pub(crate) fn tripwire(fingerprint: u64) -> TokenStream {
    let fingerprint = proc_macro2::Literal::u64_suffixed(fingerprint);
    quote! {
        assert!(
            crate::__schema::DESCRIPTOR_FINGERPRINT == #fingerprint,
            "armonik: a derive was expanded against a stale protobuf descriptor; \
             rebuild the crate"
        );
    }
}

/// The bounds every emitted impl puts on a generic type's parameters.
///
/// `ProtoField` because a field is encoded through it, `Send`/`Sync` because `prost::Message`
/// requires them, and nothing else, since no emitted code needs more. The stub emission
/// (`item::stubs`) reads the same list, so that a stub impl applies exactly where the real one
/// would.
fn bound_generics(generics: &syn::Generics) -> syn::Generics {
    let mut generics = generics.clone();
    for param in generics.type_params_mut() {
        param
            .bounds
            .push(syn::parse_quote!(crate::codec::ProtoField));
        param.bounds.push(syn::parse_quote!(::core::marker::Send));
        param.bounds.push(syn::parse_quote!(::core::marker::Sync));
    }
    generics
}

/// The `prost::Message` impl skeleton, filled in by [`message_shaped`]. `clear` is `None` for the
/// whole-value reset a resolved type wants -- the zero-default invariant makes its `Default` the
/// proto zero -- and `Some` for a poisoned one, whose reset would restore a value the expansion
/// could not describe. That placeholder is about the body only: `Msg` requires `Default` of a
/// poisoned type exactly as of a resolved one.
fn message_impl(
    generics: &syn::Generics,
    ident: &syn::Ident,
    encode_raw: TokenStream,
    merge_field: TokenStream,
    encoded_len: TokenStream,
    clear: Option<TokenStream>,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let clear = clear.unwrap_or_else(|| quote!(*self = ::core::default::Default::default();));
    quote! {
        impl #impl_generics ::prost::Message for #ident #ty_generics #where_clause {
            fn encode_raw(&self, buf: &mut impl ::prost::bytes::BufMut) {
                #encode_raw
            }

            fn merge_field(
                &mut self,
                tag: u32,
                wire_type: ::prost::encoding::WireType,
                buf: &mut impl ::prost::bytes::Buf,
                ctx: ::prost::encoding::DecodeContext,
            ) -> ::core::result::Result<(), ::prost::DecodeError> {
                #merge_field
            }

            fn encoded_len(&self) -> usize {
                #encoded_len
            }

            fn clear(&mut self) {
                #clear
            }
        }
    }
}

/// The four method bodies a message-shaped type supplies. Everything else about its emission is the
/// same whatever shape produced it, which is what [`message_shaped`] exists to say once.
pub(crate) struct MessageBodies {
    pub(crate) encode_raw: TokenStream,
    pub(crate) merge_field: TokenStream,
    pub(crate) encoded_len: TokenStream,
    /// The `Normalize` projection the representation implies, one fragment per construct that
    /// defines its own equivalence classes.
    pub(crate) normalize: Vec<TokenStream>,
}

/// Everything a message-shaped type emits: the guard block, its registry entries, and the trait
/// trio. The single call site of [`message_impl`], [`normalize_impl`] and [`msg_impl`].
///
/// `is_message` is false for an embedded oneof, which is a fragment of a message rather than one: it
/// gets no `Msg` and registers nothing. A generic type is a message with no names, which
/// [`registrations`] renders as nothing while `Msg` still carries an empty `NAMES`.
///
/// A `poisoned` expansion registers nothing either, whatever it is: a half-resolved type in the
/// differential registry only makes the harness's failures confusing. Its `clear` is a placeholder
/// like its other bodies, since resetting a value whose wire form did not resolve is not something
/// to run. It is not an exemption from `Default`: every message needs one (`Msg` requires it, and
/// decoding seeds a field from it), so an item that provides none reads as a second error, because
/// it is a second thing to fix.
#[allow(clippy::too_many_arguments)]
pub(crate) fn message_shaped(
    ident: &syn::Ident,
    generics: &syn::Generics,
    fingerprint: u64,
    names: &[String],
    is_message: bool,
    poisoned: bool,
    asserts: TokenStream,
    bodies: MessageBodies,
) -> TokenStream {
    let tripwire = tripwire(fingerprint);
    let normalize = normalize_impl(generics, ident, &bodies.normalize);
    let message = message_impl(
        generics,
        ident,
        bodies.encode_raw,
        bodies.merge_field,
        bodies.encoded_len,
        poisoned.then(|| quote!(::core::unimplemented!())),
    );
    let registrations = (is_message && !poisoned).then(|| registrations(ident, names));
    let msg = is_message.then(|| msg_impl(generics, ident, names));
    quote! {
        const _: () = {
            #tripwire
            #asserts
        };

        #registrations

        #normalize

        #message

        #msg
    }
}

/// The one-line `Msg` implementation for a message-shaped type: the blanket `ProtoField` impl in
/// `codec` picks it up, so the type composes as a field of other derived messages.
fn msg_impl(generics: &syn::Generics, ident: &syn::Ident, proto_names: &[String]) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics crate::codec::Msg for #ident #ty_generics #where_clause {
            const NAMES: &'static [&'static str] = &[#(#proto_names),*];
        }
    }
}

/// The shape assert for one slot, and for the parts of an inlined member.
///
/// Every slot that names a Rust type gets one; a `present` marker names none, and an inlined member
/// delegates to its parts. A delegate has no `Expectation` to check -- it stands for a declaration
/// rather than a field, so there is no kind or cardinality to compare -- but it does have an
/// identity, asserted so that substituting one wire-compatible type for another is caught: the
/// oneof it narrows, or for a whole-message delegate the message(s) in `names` this type claims to
/// be wire-identical to.
fn slot_asserts(slot: &Slot, type_ident: &syn::Ident, names: &[String]) -> TokenStream {
    match (&slot.codec, slot.ty()) {
        (SlotCodec::Group { parts }, _) => parts
            .iter()
            .map(|part| slot_asserts(part, type_ident, names))
            .collect(),
        // What is checked is whatever `checks` carries: the member itself for a plain field, the
        // wrapper's inner field for an inlined one, nothing for a `with` adapter, which exists
        // because the Rust representation is deliberately not the proto's.
        (SlotCodec::Field { .. }, Some(ty)) => {
            field_asserts_for(ty, slot.span, &slot.proto_path, &slot.checks, type_ident)
        }
        (SlotCodec::Delegate { ty, tags }, _) => match tags {
            Some(_) => {
                let path = &slot.proto_path;
                quote_spanned! { slot.span =>
                    const _: () = crate::codec::assert_oneof::<#ty>(#path);
                }
            }
            None => names
                .iter()
                .map(|name| {
                    quote_spanned! { slot.span =>
                        const _: () = crate::codec::assert_transparent_message::<#ty>(#name);
                    }
                })
                .collect(),
        },
        _ => TokenStream::new(),
    }
}

/// How a slot's value is encoded: through the field type's `ProtoField`, the `ProtoAdapter` a
/// `with` names, or the value's own `prost::Message` impl for a delegate.
fn slot_dispatch(slot: &Slot) -> TokenStream {
    match &slot.codec {
        // `ProtoField` and `ProtoAdapter` share their method names, so a fragment is written once
        // and prefixed with whichever the field encodes through.
        SlotCodec::Field { ty, adapter } => match adapter {
            Some(adapter) => quote!(<#adapter as crate::codec::ProtoAdapter<#ty>>),
            None => quote!(<#ty as crate::codec::ProtoField>),
        },
        SlotCodec::Delegate { ty, .. } => quote!(<#ty as ::prost::Message>),
        SlotCodec::Group { .. } | SlotCodec::Poisoned => {
            unreachable!("an inlined member frames itself; a poisoned slot is never dispatched")
        }
    }
}

/// What one slot contributes to the three walks over a message: the encode statement, the length
/// expression, and the `Normalize` projection its representation implies.
struct SlotWrite {
    pub(crate) encode: TokenStream,
    pub(crate) len: TokenStream,
    pub(crate) normalize: Option<TokenStream>,
}

/// The write side of one slot, whatever it sits in.
///
/// `value` names the value, already by reference: `&self.field` for a struct's field, the binding a
/// pattern introduced for a shared slot or member. That one parameter is the whole difference
/// between a struct's field and a variant's member on this side, which is why both body builders
/// share this. A `Group` slot ignores it: its parts name themselves, through the bindings the
/// caller's pattern introduces.
///
/// The read side does not factor the same way, and deliberately is not forced to: a shared slot
/// merges in place, while a variant's own slot has to take the shared ones out, merge, and rebuild
/// the variant around them. Those are two templates about the *enum*, not about the slot.
fn slot_write(slot: &Slot, value: &TokenStream, presence: Presence) -> SlotWrite {
    let tag = slot.tag;
    match &slot.codec {
        SlotCodec::Field { adapter, .. } => {
            let d = slot_dispatch(slot);
            // An adapter owns its wire form, deciding for itself what "nothing" looks like, so it is
            // never asked to leave a zero out: `PairMap` writes an empty map as no entries, and
            // `ErrorAdapter`'s empty string is what its `Success` *is*.
            let presence = match adapter {
                Some(_) => Presence::Explicit,
                None => presence,
            };
            // Which pair of the two method names to reach for; the skip itself is theirs.
            let (encode, len) = match presence {
                Presence::Implicit => (quote!(encode_implicit), quote!(encoded_len_implicit)),
                Presence::Explicit => (quote!(encode_field), quote!(encoded_len_field)),
            };
            SlotWrite {
                encode: quote! { #d::#encode(#tag, #value, buf); },
                len: quote! { #d::#len(#tag, #value) },
                // A `with` adapter defines its own equivalence classes; it declares them itself.
                normalize: adapter
                    .is_some()
                    .then(|| quote! { #d::normalize_dynamic(message, #tag); }),
            }
        }
        SlotCodec::Delegate { ty, .. } => {
            let d = slot_dispatch(slot);
            SlotWrite {
                encode: quote! { #d::encode_raw(#value, buf); },
                len: quote! { #d::encoded_len(#value) },
                normalize: Some(quote! {
                    <#ty as crate::differential::Normalize>::normalize(message);
                }),
            }
        }
        // A poisoned slot never reaches the walks: a struct's whole body is a placeholder, an
        // enum's poisoned arms are partitioned out.
        SlotCodec::Poisoned => unreachable!("a poisoned slot is never written"),
        // The member message is absorbed, so its framing is hand-rolled here; its parts are
        // ordinary fields, named by the bindings the caller's pattern introduced.
        SlotCodec::Group { parts } => {
            let (encodes, lens): (Vec<_>, Vec<_>) = parts
                .iter()
                .map(|part| {
                    let local = slot_local(part);
                    // Ordinary fields of the absorbed message; the framing below is what carries
                    // the member's presence, and it is written unconditionally.
                    let written = slot_write(part, &quote!(#local), Presence::Implicit);
                    (written.encode, written.len)
                })
                .unzip();
            let body_len = quote! { let body_len = 0 #(+ #lens)*; };
            SlotWrite {
                encode: quote! {
                    #body_len
                    ::prost::encoding::encode_key(
                        #tag,
                        ::prost::encoding::WireType::LengthDelimited,
                        buf,
                    );
                    ::prost::encoding::encode_varint(body_len as u64, buf);
                    #(#encodes)*
                },
                len: quote! {
                    {
                        #body_len
                        ::prost::encoding::key_len(#tag)
                            + ::prost::encoding::encoded_len_varint(body_len as u64)
                            + body_len
                    }
                },
                normalize: None,
            }
        }
    }
}

/// The local a pattern binds a slot to, whenever the emitter reaches it through a pattern rather
/// than through `self`: a whole-message enum's shared field, or one part of an inlined member.
///
/// Named `__f<tag>` and never after the user's field, because these sit in the same scope as the
/// emitter's own `buf`, `len`, `value`, `tag`, `wire_type`, `ctx`, `payload`, `parts` and
/// `body_len`. A proto field named like any of those would otherwise shadow one, which is not a
/// wrong encoding but an unimplementable message: the errors point into expanded code.
fn slot_local(slot: &Slot) -> syn::Ident {
    quote::format_ident!("__f{}", slot.tag)
}

/// Merge one slot into the place `value` names, for the slots a message reaches directly: a
/// struct's field, or a whole-message enum's shared field.
fn slot_merge_in_place(slot: &Slot, value: &TokenStream) -> TokenStream {
    match &slot.codec {
        SlotCodec::Field { .. } => {
            let d = slot_dispatch(slot);
            quote! { #d::merge_field(wire_type, #value, buf, ctx) }
        }
        SlotCodec::Delegate { .. } => {
            let d = slot_dispatch(slot);
            quote! { #d::merge_field(#value, tag, wire_type, buf, ctx) }
        }
        SlotCodec::Poisoned => unreachable!("a poisoned slot is never merged"),
        // The parts are ordinary fields, merged into the locals the caller's pattern bound, under
        // prost's own framing: the recursion and length limits `ctx` carries, and the rejection of a
        // body that runs past its declared end.
        SlotCodec::Group { parts } => {
            let locals: Vec<syn::Ident> = parts.iter().map(slot_local).collect();
            let tags = parts.iter().map(|part| part.tag);
            let tys = parts
                .iter()
                .map(|part| part.ty().expect("an inlined part carries a value"));
            let at = (0..parts.len()).map(syn::Index::from);
            quote! {{
                ::prost::encoding::check_wire_type(
                    ::prost::encoding::WireType::LengthDelimited,
                    wire_type,
                )?;
                let mut __parts = (#(&mut #locals,)*);
                ::prost::encoding::merge_loop(&mut __parts, buf, ctx, |__parts, buf, ctx| {
                    let (tag, wire_type) = ::prost::encoding::decode_key(buf)?;
                    match tag {
                        #(
                            #tags => <#tys as crate::codec::ProtoField>::merge_field(
                                wire_type, &mut *__parts.#at, buf, ctx,
                            ),
                        )*
                        _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
                    }
                })
            }}
        }
    }
}
