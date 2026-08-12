//! Doc harvesting for the message types: the `#[armonik_macros::message]` /
//! `#[armonik_macros::enumeration]` attribute macros re-emit the annotated item with `#[doc]`
//! attributes extracted from the protos' comments (type, fields, oneof variants, enum values), then
//! append the same expansion the old derives produced.
//!
//! Only an attribute macro may rewrite the item, which is the whole reason these are attributes:
//! the proto prose becomes uncopyable, as it already is for the services. Injected docs come first,
//! hand-written ones after, for Rust-specific notes. The `#[armonik(...)]` attributes are consumed
//! here and stripped from the re-emitted item.

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::attrs::{self, AttrItem};
use crate::descriptor::EnumMeta;

pub(crate) enum Mode {
    Message,
    Enumeration,
}

pub(crate) fn expand(mut input: DeriveInput, mode: Mode) -> syn::Result<TokenStream> {
    // The expansion first, over the pristine input (it reads the `#[armonik(...)]` attributes).
    let expanded = match mode {
        Mode::Message => crate::expand_message(input.clone()).map(|expansion| (expansion, None)),
        Mode::Enumeration => crate::expand_enumeration(input.clone())
            .map(|(expansion, tags)| (expansion, Some(tags))),
    };

    let (expansion, tags) = match expanded {
        Ok(expanded) => expanded,
        Err(error) => return Ok(salvage(input, &mode, error)),
    };

    // Doc injection reads the descriptor, which the expansion has already validated; a failure here
    // is a bug rather than a user error, so it still propagates.
    inject(&mut input, &mode)?;
    strip(&mut input);
    if let Some(tags) = tags {
        tag_variants(&mut input, &tags);
    }
    input.attrs.push(serde_derive());

    Ok(quote! {
        #input
        #expansion
    })
}

/// The `serde` line every one of these types carries, which was byte-identical at 198 sites.
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
fn salvage(mut input: DeriveInput, mode: &Mode, error: syn::Error) -> TokenStream {
    // Before `strip`, which is what the stubs read their shape and proto names from.
    let stubs = stubs(&input, mode);
    // Best-effort docs: the descriptor lookup may well be what failed.
    let _ = inject(&mut input, mode);
    strip(&mut input);
    // The same line the happy path adds. Without it, one mistake reads as one error under the
    // default features and as five, with `and 365 others`, under `--all-features`, which is what
    // the crate's own tests and CI run: every `serde` bound on the re-emitted type goes unmet.
    input.attrs.push(serde_derive());
    let error = error.into_compile_error();
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
fn stubs(input: &DeriveInput, mode: &Mode) -> TokenStream {
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
        matches!(mode, Mode::Enumeration) && !has(|item| matches!(item, AttrItem::Transparent));

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
            // through `prost::Message` and projects it through `Normalize`. Unlike a whole message
            // it gets no `Msg`: it stands for a fragment of one, not for a message.
            let normalize =
                crate::emit::normalize_impl(&generics, ident, std::slice::from_ref(&unimplemented));
            quote! {
                #message
                #normalize
            }
        } else {
            // `Msg: prost::Message + Default`, and a type whose expansion failed has no emitted
            // `Default` of its own. An enumeration gets one below; anything else has to derive it,
            // and where it does not the `Msg` stub is skipped rather than bounded: `where Self:
            // Default` on a concrete type is rejected outright (`trivial_bounds`), which would add
            // the second error this whole path exists to avoid.
            let msg = (matches!(mode, Mode::Enumeration) || derives_default(input))
                .then(|| crate::emit::msg_impl(&generics, ident, &proto_names));
            quote! {
                #message
                #msg
            }
        }
    };

    if matches!(mode, Mode::Enumeration) {
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
            // No serde derive, unlike the real one: a stub next to a `compile_error!` is never
            // serialized, and the `cfg` would be one the crate under test need not know about.
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

/// Carry each variant's proto value as its discriminant, so that the ordering the type derives
/// (`PartialOrd`/`Ord` compare discriminants) is the ordering of the proto values. The catch-all
/// stands for the zero value and for every value unknown to this crate version, which share no
/// single number: it takes `i32::MIN`, so they sort before every named value and among themselves
/// by the raw value their payload holds.
fn tag_variants(input: &mut DeriveInput, tags: &crate::EnumTags) {
    // Explicit discriminants on an enum that has a dataful variant need a primitive representation.
    input.attrs.push(syn::parse_quote!(#[repr(i32)]));
    let syn::Data::Enum(data) = &mut input.data else {
        return;
    };
    for variant in &mut data.variants {
        let value: syn::Expr = if variant.ident == tags.unknown {
            syn::parse_quote!(i32::MIN)
        } else {
            match tags.named.iter().find(|(name, _)| *name == variant.ident) {
                Some((_, value)) => syn::parse_quote!(#value),
                None => continue,
            }
        };
        variant.discriminant = Some((syn::token::Eq::default(), value));
    }
}

/// Inject the harvested `#[doc]`s: on the type, its named fields, its oneof variants (matched to
/// members like the resolver does, by snake_cased name or `rename`), its struct-variant fields, and
/// its enum values.
fn inject(input: &mut DeriveInput, mode: &Mode) -> syn::Result<()> {
    // The proto the type stands for: the first `message =` / `enum =` name (unified types agree on
    // their shape, the first one documents it). `generic` types name no proto and get nothing.
    let entries = attrs::parse(&input.attrs)?;
    let proto = entries.iter().find_map(|entry| match &entry.item {
        AttrItem::Message(lit) | AttrItem::Enum(lit) => Some(lit.value()),
        _ => None,
    });
    let Some(proto) = proto else {
        return Ok(());
    };
    let index = crate::load_index(input)?;

    if let (Mode::Enumeration, Some(meta)) = (&mode, index.enums.get(&proto)) {
        return inject_enumeration(input, &proto, meta);
    }
    let Some(meta) = index.messages.get(&proto) else {
        // Transparent enums name wrapper *messages*; type docs only.
        if let Some(meta) = index.enums.get(&proto) {
            prepend(&mut input.attrs, &meta.docs);
        }
        return Ok(());
    };

    prepend(&mut input.attrs, &meta.docs);

    let field_docs = |attrs: &[syn::Attribute], ident: Option<&syn::Ident>| -> Vec<String> {
        let name = renamed(attrs).or_else(|| ident.map(ToString::to_string));
        name.and_then(|name| {
            meta.fields
                .iter()
                .find(|field| field.name == name)
                .map(|field| field.docs.clone())
        })
        .unwrap_or_default()
    };

    match &mut input.data {
        syn::Data::Struct(data) => {
            for field in &mut data.fields {
                let docs = field_docs(&field.attrs, field.ident.as_ref());
                prepend(&mut field.attrs, &docs);
            }
        }
        syn::Data::Enum(data) => {
            // Whole-message and embedded-oneof enums: variants are oneof members; struct-variant
            // fields are sibling or inlined fields.
            for variant in &mut data.variants {
                let name = renamed(&variant.attrs)
                    .unwrap_or_else(|| crate::names::snake(&variant.ident.to_string()));
                let docs = meta
                    .fields
                    .iter()
                    .find(|field| field.name == name)
                    .map(|field| field.docs.clone())
                    .unwrap_or_default();
                prepend(&mut variant.attrs, &docs);
                if let syn::Fields::Named(fields) = &mut variant.fields {
                    for field in &mut fields.named {
                        let docs = field_docs(&field.attrs, field.ident.as_ref());
                        prepend(&mut field.attrs, &docs);
                    }
                }
            }
        }
        syn::Data::Union(_) => {}
    }
    Ok(())
}

fn inject_enumeration(input: &mut DeriveInput, proto: &str, meta: &EnumMeta) -> syn::Result<()> {
    prepend(&mut input.attrs, &meta.docs);

    let syn::Data::Enum(data) = &mut input.data else {
        return Ok(());
    };
    // Matched exactly as the resolver matches: through `names::variant_name`, off the *proto* enum's
    // simple name. Harvesting used to strip a SCREAMING_SNAKE spelling of the Rust type's name
    // instead, which silently harvested nothing whenever the two names differed.
    let simple = proto.rsplit('.').next().unwrap_or(proto);
    for variant in &mut data.variants {
        let docs = meta
            .values
            .iter()
            .zip(&meta.value_docs)
            .find(|((name, _), _)| match renamed(&variant.attrs) {
                Some(rename) => name == &rename,
                None => variant.ident == crate::names::variant_name(simple, name),
            })
            .map(|(_, docs)| docs.clone())
            .unwrap_or_default();
        prepend(&mut variant.attrs, &docs);
    }
    Ok(())
}

/// The `#[armonik(rename = "...")]` value among `attrs`, if any.
fn renamed(attrs: &[syn::Attribute]) -> Option<String> {
    attrs::parse(attrs).ok().and_then(|entries| {
        entries.iter().find_map(|entry| match &entry.item {
            AttrItem::Rename(lit) => Some(lit.value()),
            _ => None,
        })
    })
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
