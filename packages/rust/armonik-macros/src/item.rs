//! The re-emitted item: the `#[armonik_macros::message]` / `#[armonik_macros::enumeration]`
//! attribute macros hand the annotated type back with `#[doc]` attributes extracted from the
//! protos' comments (type, fields, oneof variants, enum values), the `#[armonik(...)]` attributes
//! stripped, and the hover anchors that make those attributes documented.
//!
//! Only an attribute macro may rewrite the item, which is the whole reason these are attributes:
//! the proto prose becomes uncopyable, as it already is for the services. Injected docs come first,
//! hand-written ones after, for Rust-specific notes.
//!
//! Also the salvage path, which is what the item looks like when resolution failed.

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::attrs::{self, AttrItem, Errors};
use crate::plan::{EnumPlan, Ir, Slot, SlotCodec};

/// Which macro is expanding. Not a shuttle between layers: the salvage path has no plan to read the
/// shape off, and the doc harvest matches enum values differently from message fields.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Message,
    Enumeration,
}

impl Kind {
    /// The macro's own name, for the hover anchors.
    fn derive(self) -> &'static str {
        match self {
            Kind::Message => "message",
            Kind::Enumeration => "enumeration",
        }
    }
}

/// Re-emit the item of a `#[armonik_macros::message]` type: the plan's docs injected,
/// `#[armonik(...)]` stripped, the serde line added.
///
/// Infallible: everything it needs is already in the plan.
pub(crate) fn rewrite(input: &mut DeriveInput, ir: &Ir) {
    inject(input, ir);
    strip(input);
    input.attrs.push(serde_derive());
}

/// The same, minus the serde line: an enumeration's `Serialize`/`Deserialize` are emitted by hand
/// (`shape::enumeration::serde`), because the derived pair spells the catch-all as an object and
/// builds it without normalizing. The comparison traits are emitted for the same reason, so nothing
/// here has to reach a proto value either.
pub(crate) fn rewrite_enum(input: &mut DeriveInput, plan: &EnumPlan) {
    inject_enum(input, plan);
    strip(input);
}

/// Hover-documentation anchors: every `#[armonik(...)]` key token of the input, re-emitted as an
/// anonymous import of the deriving macro respanned onto the key. IDE hover on the otherwise-inert
/// keys then resolves to this crate's macro, the single home of the grammar documentation. The
/// anonymous `const` compiles to nothing.
///
/// Reads the pristine item, so it has to run before [`rewrite`] strips the attributes it points at.
pub(crate) fn anchors(input: &DeriveInput, kind: Kind) -> TokenStream {
    let mut spans = Vec::new();
    attrs::for_each_site(input, |attrs| spans.extend(attrs::key_spans(attrs)));
    if spans.is_empty() {
        return TokenStream::new();
    }
    let uses = spans.iter().map(|span| {
        let derive = syn::Ident::new(kind.derive(), *span);
        quote! {
            {
                #[allow(unused_imports)]
                use ::armonik_macros::#derive as _;
            }
        }
    });
    quote! {
        const _: () = {
            #(#uses)*
        };
    }
}

/// The `serde` line every one of these types carries.
///
/// The `#[derive(...)]` above it is deliberately *not* emitted: it varies across ten shapes, and
/// hiding it would take the derive set off the type.
fn serde_derive() -> syn::Attribute {
    syn::parse_quote!(
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    )
}

/// Emit the annotated type anyway, next to the error.
///
/// Returning only `compile_error!` deletes the type, so every `use` of it downstream becomes
/// `E0432` carrying a *wrong* suggestion (the same name in an unrelated module), and every field
/// typed by it becomes `E0277`. One mistake then reads as a page of unrelated ones, none of which
/// points at the mistake. So on the error path the item is re-emitted, together with the parts of
/// the expansion its users need to type-check: the type resolves, its uses resolve, and the one
/// real error is the only one reported.
///
/// The stubs are unreachable by construction: the `compile_error!` next to them guarantees the
/// crate never builds.
///
/// No hover anchors here, deliberately: they would point at the `#[armonik(...)]` keys of an item
/// that did not resolve, in a build that is failing anyway.
pub(crate) fn salvage(mut input: DeriveInput, kind: Kind, error: Errors) -> TokenStream {
    // Before `strip`, which is what the stubs read their shape and proto names from.
    let stubs = stubs(&input, kind);
    // Type-level docs only, and best-effort: the descriptor lookup may well be what failed.
    inject_type_docs(&mut input);
    strip(&mut input);
    // The same line the happy path adds. Without it, one mistake reads as one error under the
    // default features and as five, with `and 365 others`, under `--all-features`, which is what
    // the crate's own tests and CI run: every `serde` bound on the re-emitted type goes unmet.
    input.attrs.push(serde_derive());
    let error = error.into_syn_error().into_compile_error();
    quote! {
        #input
        #stubs
        #error
    }
}

/// The parts of a type's expansion that its *users* need, one shape at a time.
///
/// The trait a field site reaches for depends on the shape: a struct or whole-message enum is
/// reached through `codec::Msg` (which blankets `ProtoField`), an embedded oneof through
/// `prost::Message` alone, a plain enumeration through `codec::ProtoField` directly. The `SHAPE` of a
/// stub names no proto message, which the const-asserts read as "unchecked", so the field sites
/// that merely mention the type stay quiet.
fn stubs(input: &DeriveInput, kind: Kind) -> TokenStream {
    let ident = &input.ident;
    let entries = attrs::parse(&input.attrs).unwrap_or_default();
    let has = |want: fn(&AttrItem) -> bool| entries.iter().any(|entry| want(&entry.item));
    // The proto names the type claims. Taken from the attributes rather than left empty, so that
    // the `service!`-emitted asserts checking a request or response type against its RPC still
    // hold: an empty `NAMES` would fail every one of them.
    let proto_names: Vec<String> = entries
        .iter()
        .filter_map(|entry| match &entry.item {
            AttrItem::Message(lit) => Some(lit.value()),
            _ => None,
        })
        .collect();

    // The same bounds the real emission puts on, from the same place, so a stub impl applies
    // exactly where the real one would.
    let generics = crate::emit::bound_generics(&input.generics);

    // A plain enumeration is the one shape that is not message-shaped: it implements `ProtoField`
    // itself, and an enum-typed field's const-assert wants the enum kind, not the message one.
    let plain_enumeration =
        matches!(kind, Kind::Enumeration) && !has(|item| matches!(item, AttrItem::Transparent));

    let mut out = if plain_enumeration {
        quote! {
            impl crate::codec::ProtoField for #ident {
                const SHAPE: crate::codec::Shape = crate::codec::Shape::enumeration(&[]);

                fn encode_field(_tag: u32, _value: &Self, _buf: &mut impl ::prost::bytes::BufMut) {
                    ::core::unimplemented!()
                }

                fn merge_field(
                    _wire_type: ::prost::encoding::WireType,
                    _value: &mut Self,
                    _buf: &mut impl ::prost::bytes::Buf,
                    _ctx: ::prost::encoding::DecodeContext,
                ) -> ::core::result::Result<(), ::prost::DecodeError> {
                    ::core::unimplemented!()
                }

                fn encoded_len_field(_tag: u32, _value: &Self) -> usize {
                    ::core::unimplemented!()
                }
            }
        }
    } else {
        // Both remaining shapes are message-shaped, so both get the same `prost::Message`, from the
        // same place the real one comes from. Only `clear` differs from a real emission: the
        // whole-value reset needs a `Default` this type may not have.
        let unimplemented = quote!(::core::unimplemented!());
        let message = crate::emit::message_impl(
            &generics,
            ident,
            quote! { let _ = buf; #unimplemented },
            quote! { let _ = (tag, wire_type, buf, ctx); #unimplemented },
            unimplemented.clone(),
            Some(unimplemented.clone()),
        );
        if has(|item| matches!(item, AttrItem::Oneof(_))) {
            // An embedded oneof is carried by a field of the struct that owns it, which encodes it
            // through `prost::Message`, projects it through `Normalize`, and const-asserts its
            // identity through `Oneof`. Unlike a whole message it gets no `Msg`: it stands for a
            // fragment of one, not for a message.
            //
            // The empty `ONEOF` is the "unchecked" case, and it is load-bearing rather than
            // defensive: without it a typo'd `oneof = "..."` reports the real error plus an
            // `E0277: Condition: Oneof` at every field carrying the type, which is the cascade this
            // whole path exists to prevent.
            let marker = quote! {
                impl #generics crate::codec::Oneof for #ident #generics {
                    const ONEOF: &'static [&'static str] = &[];
                }
            };
            let normalize =
                crate::emit::normalize_impl(&generics, ident, std::slice::from_ref(&unimplemented));
            quote! {
                #marker
                #message
                #normalize
            }
        } else {
            // `Msg: prost::Message + Default`, and a type whose expansion failed has no emitted
            // `Default` of its own. An enumeration gets one below; anything else has to derive it,
            // and where it does not the `Msg` stub is skipped rather than bounded: `where Self:
            // Default` on a concrete type is rejected outright (`trivial_bounds`), which would add
            // the second error this whole path exists to avoid.
            let msg = (matches!(kind, Kind::Enumeration) || derives_default(input))
                .then(|| crate::emit::msg_impl(&generics, ident, &proto_names));
            quote! {
                #message
                #msg
            }
        }
    };

    if matches!(kind, Kind::Enumeration) {
        out.extend(enumeration_stubs(input, ident));
    }
    out
}

/// Whether the item carries `Default` in a `#[derive(...)]`.
fn derives_default(input: &DeriveInput) -> bool {
    input.attrs.iter().any(|attr| {
        attr.path().is_ident("derive")
            && attr
                .parse_args_with(
                    syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
                )
                .is_ok_and(|paths| paths.iter().any(|path| path.is_ident("Default")))
    })
}

/// The enumeration expansion owns more than the trait impls: the catch-all payload struct, the two
/// `i32` conversions and `Default`. Without the payload struct the re-emitted enum does not even
/// name a type that exists.
fn enumeration_stubs(input: &DeriveInput, ident: &syn::Ident) -> TokenStream {
    let syn::Data::Enum(data) = &input.data else {
        return TokenStream::new();
    };

    // The catch-all is `Variant(Payload)`; the resolver rejects anything else, and a type that got
    // that far may still have failed on a value mismatch.
    let payloads: Vec<&syn::Ident> = data
        .variants
        .iter()
        .filter_map(|variant| match &variant.fields {
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                match &fields.unnamed[0].ty {
                    syn::Type::Path(path) => path.path.get_ident(),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();

    // `Default` comes from the expansion unless a variant carries the std `#[default]`, in which
    // case the type derives it and a second impl would collide.
    let derives_default = data.variants.iter().any(|variant| {
        variant
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("default"))
    });
    let default = (!derives_default).then(|| {
        quote! {
            impl ::core::default::Default for #ident {
                fn default() -> Self {
                    ::core::unimplemented!()
                }
            }
        }
    });

    quote! {
        #(
            // The serde line, like the real one and for the same reason `salvage` puts it on the
            // re-emitted enum: that enum's catch-all holds this struct, so a `derive(Deserialize)`
            // above needs one here too. Without it, under `--all-features`, one mistake in an
            // enumeration reports as three errors, which is the cascade this whole path prevents.
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
            pub struct #payloads(i32);

            impl #payloads {
                /// The raw protobuf value.
                pub const fn value(self) -> i32 {
                    self.0
                }
            }
        )*

        impl ::core::convert::From<i32> for #ident {
            fn from(_value: i32) -> Self {
                ::core::unimplemented!()
            }
        }

        impl ::core::convert::From<#ident> for i32 {
            fn from(_value: #ident) -> Self {
                ::core::unimplemented!()
            }
        }

        #default
    }
}

/// Apply the docs the plan harvested: on the type, its fields, its oneof variants and their
/// sibling or inlined fields.
///
/// A pure applier. It matches nothing: every `#[doc]` below was decided by resolution, which is
/// what makes the inlined-variant and transparent-enum cases work at all. Slots are found through
/// [`Slot::reaches`], the access path resolution already recorded.
fn inject(input: &mut DeriveInput, ir: &Ir) {
    prepend(&mut input.attrs, &ir.docs);
    match (&mut input.data, &ir.discr) {
        (syn::Data::Struct(data), None) => {
            apply(
                data.fields.iter_mut(),
                &ir.shared.iter().collect::<Vec<_>>(),
            );
        }
        (syn::Data::Enum(data), Some(discr)) => {
            for variant in &mut data.variants {
                let arm = discr
                    .arms
                    .iter()
                    .find(|candidate| candidate.ident == variant.ident);
                if let Some(arm) = arm {
                    prepend(&mut variant.attrs, &arm.own.docs);
                }
                // Every variant carries the shared fields; the "no member set" one carries only
                // those. A member is reached either through its own field, or, under `inline`,
                // through one field per part.
                let mut slots: Vec<&Slot> = ir.shared.iter().collect();
                if let Some(arm) = arm {
                    match &arm.own.codec {
                        SlotCodec::Group { parts } => slots.extend(parts),
                        _ => slots.push(&arm.own),
                    }
                }
                apply(variant.fields.iter_mut(), &slots);
            }
        }
        _ => {}
    }
}

/// The same for an enumeration: the type, then one variant per proto value.
fn inject_enum(input: &mut DeriveInput, plan: &EnumPlan) {
    prepend(&mut input.attrs, &plan.docs);
    let syn::Data::Enum(data) = &mut input.data else {
        return;
    };
    for variant in &mut data.variants {
        if let Some(value) = plan.named.iter().find(|value| value.ident == variant.ident) {
            prepend(&mut variant.attrs, &value.docs);
        }
    }
}

fn apply<'a>(fields: impl Iterator<Item = &'a mut syn::Field>, slots: &[&Slot]) {
    for (index, field) in fields.enumerate() {
        if let Some(slot) = slots.iter().find(|slot| slot.reaches(field, index)) {
            let docs = slot.docs.clone();
            prepend(&mut field.attrs, &docs);
        }
    }
}

/// Type-level docs for the salvage path, where there is no plan to read them off.
///
/// Fields and variants get none: matching them to proto names would be a second copy of the
/// resolvers' rules, for a build that is failing anyway.
fn inject_type_docs(input: &mut DeriveInput) {
    let Ok(entries) = attrs::parse(&input.attrs) else {
        return;
    };
    let Some(proto) = entries.iter().find_map(|entry| match &entry.item {
        AttrItem::Message(lit) | AttrItem::Enum(lit) => Some(lit.value()),
        _ => None,
    }) else {
        return;
    };
    let Ok(index) = crate::descriptor::index() else {
        return;
    };
    // Both tables, whatever the macro: one descriptor set never names a message and an enum the
    // same, and a transparent enumeration names wrapper *messages*.
    let docs = index
        .enums
        .get(&proto)
        .map(|meta| meta.docs.clone())
        .or_else(|| index.messages.get(&proto).map(|meta| meta.docs.clone()));
    if let Some(docs) = docs {
        prepend(&mut input.attrs, &docs);
    }
}

/// Put the harvested docs *before* the existing attributes, so hand-written doc comments read as
/// additional notes after the proto prose.
fn prepend(attrs: &mut Vec<syn::Attribute>, docs: &[String]) {
    for line in docs.iter().rev() {
        attrs.insert(0, syn::parse_quote!(#[doc = #line]));
    }
}

/// Visit every field of the item, wherever it sits: a struct's own, or one of a variant's.
fn for_each_field(input: &mut DeriveInput, visit: impl FnMut(&mut syn::Field)) {
    match &mut input.data {
        syn::Data::Struct(data) => data.fields.iter_mut().for_each(visit),
        syn::Data::Enum(data) => data
            .variants
            .iter_mut()
            .flat_map(|variant| variant.fields.iter_mut())
            .for_each(visit),
        syn::Data::Union(_) => {}
    }
}

/// Remove every `#[armonik(...)]` attribute: they were consumed by the expansion and are not
/// registered anywhere once the item is re-emitted.
fn strip(input: &mut DeriveInput) {
    fn retain(attrs: &mut Vec<syn::Attribute>) {
        attrs.retain(|attr| !attr.path().is_ident("armonik"));
    }
    retain(&mut input.attrs);
    if let syn::Data::Enum(data) = &mut input.data {
        for variant in &mut data.variants {
            retain(&mut variant.attrs);
        }
    }
    for_each_field(input, |field| retain(&mut field.attrs));
}
