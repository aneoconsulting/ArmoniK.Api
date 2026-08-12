//! The emission pieces every shape is built from.
//!
//! One field is written the same way wherever it sits: a struct field, a oneof sibling, an active
//! oneof member and one part of an inlined member all reduce to [`field_fragments`]. The rest is
//! the impl skeletons (`prost::Message`, `Msg`, `Normalize`), the registry call, the descriptor
//! fingerprint tripwire, and the per-field shape assert.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

use crate::descriptor::{Cardinality, FieldKind};
use crate::plan::{Expectation, FieldAccess, Slot, SlotCodec};

impl quote::ToTokens for FieldAccess {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            FieldAccess::Named(ident) => quote::ToTokens::to_tokens(ident, tokens),
            FieldAccess::Indexed(index) => quote::ToTokens::to_tokens(index, tokens),
        }
    }
}

/// Runtime path of a descriptor kind, for const-assert patterns. `None` for the sint/fixed wire
/// kinds the codec does not implement (no ArmoniK field uses them); the caller turns that into a
/// clear "unsupported wire kind" compile error rather than referencing a `codec::FieldKind` variant
/// that no longer exists.
pub(crate) fn kind_pattern(kind: &FieldKind) -> Option<TokenStream> {
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

pub(crate) fn kind_description(kind: &FieldKind) -> String {
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
pub(crate) fn cardinalities(expect: &Expectation) -> Vec<(TokenStream, &'static str)> {
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
pub(crate) fn type_name(kind: &FieldKind) -> Option<&str> {
    match kind {
        FieldKind::Message(name) | FieldKind::Enum(name) => Some(name),
        _ => None,
    }
}

/// Human form of the expected shape, for the assert message.
pub(crate) fn describe(expect: &Expectation) -> String {
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

/// One spanned shape assert per checked field: the field type's `SHAPE` against the descriptor's
/// `Expect`.
pub(crate) fn field_asserts_for(
    ty: &syn::Type,
    span: proc_macro2::Span,
    proto_path: &str,
    checks: &Option<Expectation>,
    type_ident: &syn::Ident,
) -> TokenStream {
    let Some(expect) = checks else {
        return TokenStream::new();
    };

    // A map's own kind is not checked: what it is made of is, through `map`.
    let is_map = matches!(expect.cardinality, Cardinality::Map { .. });
    let kind_expr = if is_map {
        quote!(::core::option::Option::None)
    } else {
        match kind_pattern(&expect.kind) {
            Some(token) => quote!(::core::option::Option::Some(#token)),
            None => return unsupported_kind_error(&expect.kind, proto_path, span),
        }
    };
    let map_expr = match &expect.cardinality {
        Cardinality::Map { key, value } => match (kind_pattern(key), kind_pattern(value)) {
            (Some(key_token), Some(value_token)) => {
                quote!(::core::option::Option::Some((#key_token, #value_token)))
            }
            (key_token, _) => {
                let unsupported = if key_token.is_none() { key } else { value };
                return unsupported_kind_error(unsupported, proto_path, span);
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
    let message = format!(
        "armonik: the Rust type of the field of `{type_ident}` mapping to proto field \
         `{proto_path}` does not have the expected shape ({})",
        describe(expect),
    );
    quote_spanned! {span=>
        assert!(
            crate::codec::shape_matches(
                &<#ty as crate::codec::ProtoField>::SHAPE,
                &crate::codec::Expect {
                    kind: #kind_expr,
                    cardinalities: &[#(#cards),*],
                    name: #name_expr,
                    map: #map_expr,
                },
            ),
            #message
        );
    }
}

/// A spanned compile error for a proto field whose wire kind the codec does not implement (the
/// sint/fixed kinds; no ArmoniK field uses them, so `codec::FieldKind` omits them).
pub(crate) fn unsupported_kind_error(
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

/// Qualified dispatch prefix for a field's codec: `ProtoField` and `ProtoAdapter` share their
/// method names, so every fragment is written once and prefixed with whichever the field encodes
/// through.
pub(crate) fn dispatch(ty: &syn::Type, adapter: Option<&syn::Type>) -> TokenStream {
    match adapter {
        Some(adapter) => quote!(<#adapter as crate::codec::ProtoAdapter<#ty>>),
        None => quote!(<#ty as crate::codec::ProtoField>),
    }
}

/// The `(tag, encode statement, length expression)` fragments for one field, where `value` is the
/// expression holding it. Every field is written the same way, nothing being conditional on what it
/// holds, so a plain struct field, a oneof sibling, an active oneof member and one part of an
/// inline variant all reduce to this. The encode statement writes into a `buf` in scope; the length
/// is an expression, so a caller can sum it into an accumulator or return it straight out of a
/// match arm.
pub(crate) fn field_fragments(
    dispatch: &TokenStream,
    tag: u32,
    value: TokenStream,
) -> (u32, TokenStream, TokenStream) {
    (
        tag,
        quote! { #dispatch::encode_field(#tag, #value, buf); },
        quote! { #dispatch::encoded_len_field(#tag, #value) },
    )
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

/// Register proto messages a flattening construct swallows into its parent (a `with` adapter's
/// `absorbs`, a transparent chain's middle wrappers, an inline struct variant's message), so they
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
/// chains, oneof delegation).
pub(crate) fn normalize_impl(
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
/// requires them. Nothing else: `PartialEq` and `Debug` were here for the deleted `is_default`
/// family and no emitted code needs them. The stub emission (`item::stubs`) reads the same list, so
/// that a stub impl applies exactly where the real one would.
pub(crate) fn bound_generics(generics: &syn::Generics) -> syn::Generics {
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

/// The `prost::Message` impl skeleton shared by every emission site (plain struct, transparent
/// struct, transparent enum, whole-message oneof, and the `item::stubs` error path). `clear` is
/// `None` for the whole-value reset every real site wants -- every derived type is `Default`, and
/// the zero-default invariant makes that the proto zero -- and `Some` only for a stub, whose type
/// may have no `Default` at all.
pub(crate) fn message_impl(
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
/// Six shapes reach here (plain, transparent and generic structs, whole-message and embedded oneofs,
/// and a transparent enumeration), and they differ only in the four bodies and in whether the type
/// stands for a message at all. The four emitters covering them used to restate the assembly
/// separately, so "a message-shaped type gets exactly these impls, with these bounds, under these
/// names" was a fact spelled four times and true by coincidence.
///
/// `is_message` is false for an embedded oneof, which is a fragment of a message rather than one: it
/// gets no `Msg` and registers nothing. A generic type is a message with no names, which
/// [`registrations`] renders as nothing while `Msg` still carries an empty `NAMES`.
pub(crate) fn message_shaped(
    ident: &syn::Ident,
    generics: &syn::Generics,
    fingerprint: u64,
    names: &[String],
    is_message: bool,
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
        None,
    );
    let registrations = is_message.then(|| registrations(ident, names));
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
pub(crate) fn msg_impl(
    generics: &syn::Generics,
    ident: &syn::Ident,
    proto_names: &[String],
) -> TokenStream {
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
/// delegates to its parts.
pub(crate) fn slot_asserts(slot: &Slot, type_ident: &syn::Ident) -> TokenStream {
    match (&slot.codec, slot.ty()) {
        (SlotCodec::Inline { parts }, _) => parts
            .iter()
            .map(|part| slot_asserts(part, type_ident))
            .collect(),
        // A `with` adapter is checked by nothing: it exists because the Rust representation is
        // deliberately not the proto's.
        (SlotCodec::Field { adapter, .. }, Some(ty)) if adapter.is_none() => {
            field_asserts_for(ty, slot.span, &slot.proto_path, &slot.checks, type_ident)
        }
        _ => TokenStream::new(),
    }
}

/// How a slot's value is encoded: through the field type's `ProtoField`, or through the
/// `ProtoAdapter` a `with` names.
pub(crate) fn slot_dispatch(slot: &Slot) -> TokenStream {
    match &slot.codec {
        SlotCodec::Field { ty, adapter } => dispatch(ty, adapter.as_deref()),
        SlotCodec::Oneof { ty, .. } => quote!(<#ty as ::prost::Message>),
        SlotCodec::Marker { .. } | SlotCodec::Inline { .. } => {
            unreachable!("markers and inlined members frame themselves")
        }
    }
}

/// What one slot contributes to the three walks over a message: the encode statement, the length
/// expression, and the `Normalize` projection its representation implies.
pub(crate) struct SlotWrite {
    pub(crate) encode: TokenStream,
    pub(crate) len: TokenStream,
    pub(crate) normalize: Option<TokenStream>,
}

/// The write side of one slot, whatever it sits in.
///
/// `value` names the value, already by reference: `&self.field` for a struct's field, the binding a
/// pattern introduced for a oneof's sibling or member. That one parameter is the whole difference
/// between a struct's field and a variant's member on this side, which is why both emitters can
/// share this. An `Inline` slot ignores it: its parts name themselves, through the bindings the
/// caller's pattern introduces.
///
/// The read side does not factor the same way, and deliberately is not forced to: a shared slot
/// merges in place, while a variant's own slot has to take the shared ones out, merge, and rebuild
/// the variant around them. Those are two templates about the *enum*, not about the slot.
pub(crate) fn slot_write(slot: &Slot, value: &TokenStream) -> SlotWrite {
    let tag = slot.tag;
    match &slot.codec {
        SlotCodec::Field { adapter, .. } => {
            let d = slot_dispatch(slot);
            let (_, encode, len) = field_fragments(&d, tag, value.clone());
            SlotWrite {
                encode,
                len,
                // A `with` adapter defines its own equivalence classes; it declares them itself.
                normalize: adapter
                    .is_some()
                    .then(|| quote! { #d::normalize_dynamic(message, #tag); }),
            }
        }
        SlotCodec::Oneof { ty, .. } => {
            let d = slot_dispatch(slot);
            SlotWrite {
                encode: quote! { #d::encode_raw(#value, buf); },
                len: quote! { #d::encoded_len(#value) },
                normalize: Some(quote! {
                    <#ty as crate::differential::Normalize>::normalize(message);
                }),
            }
        }
        // The active member carries the oneof's presence, so a marker writes something whatever it
        // holds: `true` for a bool member, an empty body for a message one.
        SlotCodec::Marker {
            empty_message: false,
        } => SlotWrite {
            encode: quote! {
                <bool as crate::codec::ProtoField>::encode_field(#tag, &true, buf);
            },
            len: quote! { <bool as crate::codec::ProtoField>::encoded_len_field(#tag, &true) },
            // Only presence survives: an explicit `false` still selects the variant.
            normalize: Some(quote! { crate::differential::bool_marker(message, #tag); }),
        },
        SlotCodec::Marker {
            empty_message: true,
        } => SlotWrite {
            encode: quote! { crate::codec::empty_body::encode(#tag, buf); },
            len: quote! { crate::codec::empty_body::encoded_len(#tag) },
            normalize: None,
        },
        // The member message is absorbed, so its framing is hand-rolled here; its parts are
        // ordinary fields, named by the bindings the caller's pattern introduced.
        SlotCodec::Inline { parts } => {
            let (encodes, lens): (Vec<_>, Vec<_>) = parts
                .iter()
                .map(|part| {
                    let local = slot_local(part);
                    let written = slot_write(part, &quote!(#local));
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
pub(crate) fn slot_local(slot: &Slot) -> syn::Ident {
    quote::format_ident!("__f{}", slot.tag)
}

/// Merge one slot into the place `value` names, for the slots a message reaches directly: a
/// struct's field, or a whole-message enum's shared field.
pub(crate) fn slot_merge_in_place(slot: &Slot, value: &TokenStream) -> TokenStream {
    let d = slot_dispatch(slot);
    match &slot.codec {
        SlotCodec::Field { .. } => quote! { #d::merge_field(wire_type, #value, buf, ctx) },
        SlotCodec::Oneof { .. } => {
            quote! { #d::merge_field(#value, tag, wire_type, buf, ctx) }
        }
        SlotCodec::Marker { .. } | SlotCodec::Inline { .. } => {
            unreachable!("markers and inlined members are only ever a variant's own slot")
        }
    }
}
