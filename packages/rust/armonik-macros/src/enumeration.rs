//! `#[armonik_macros::enumeration]`: a proto enum, plain or flattened through a chain of
//! single-field wrapper messages.

use proc_macro2::{Span, TokenStream};
use quote::quote;

use crate::attrs::{self, scan_attrs, unraw, Allowed, AttrItem, Errors, FieldAttrs};
use crate::descriptor::{DescriptorIndex, FieldKind};
use crate::emit::{message_shaped, tripwire, MessageBodies};
use crate::matcher::{not_found, unknown_name};
use crate::plan::{EnumMode, EnumPlan, EnumValue};

/// Resolve `#[armonik_macros::enumeration]`. A proto enum and a transparent wrapper chain around
/// one are two modes of a single plan rather than two shapes: [`enum_plan`] owns that split, and
/// the entry point reads the mode back off the plan to pick the wire impl.
pub(crate) fn resolve_enumeration(input: &syn::DeriveInput) -> Result<EnumPlan, Errors> {
    let index = crate::resolve::index(input)?;
    enum_plan(input, &index)
}

fn enum_plan(input: &syn::DeriveInput, index: &DescriptorIndex) -> Result<EnumPlan, Errors> {
    let mut errors = Errors::new();
    reject_implemented_derives(input, &mut errors);

    let entries = attrs::parse(&input.attrs)?;

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

    // Resolve the proto enum(s) the variants are matched against, and the wrapper tag in
    // transparent mode.
    let mut proto_enums: Vec<(String, &crate::descriptor::EnumMeta)> = Vec::new();
    // Intermediate wrapper messages walked through in transparent mode: they have no Rust type, so
    // they are registered as absorbed.
    let mut absorbs: Vec<String> = Vec::new();
    let mode = if transparent {
        if message_names.is_empty() {
            errors.at(
                input.ident.span(),
                "#[armonik(transparent)] requires #[armonik(message = \"full.proto.Name\")] \
                 naming the single-field wrapper message",
            );
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
                    errors.at(
                        *span,
                        format!("`{current}` is not a single-field wrapper message"),
                    );
                    break None;
                };
                path.push(field.tag);
                match &field.kind {
                    FieldKind::Enum(inner) => break Some(inner.clone()),
                    FieldKind::Message(inner) => {
                        // A wrapper layer between the root message and the enum: no Rust type
                        // stands for it.
                        absorbs.push(inner.clone());
                        current = inner.clone();
                    }
                    other => {
                        errors.at(
                            *span,
                            format!(
                                "the single field of `{current}` is neither an enum nor a \
                                 wrapper message ({other:?})"
                            ),
                        );
                        break None;
                    }
                }
            };
            let Some(enum_name) = enum_name else {
                continue;
            };
            if let Some(previous) = &wrapper_path {
                if *previous != path {
                    errors.at(
                        *span,
                        "transparent wrapper messages disagree on the wrapper tag path",
                    );
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
            errors.at(
                input.ident.span(),
                "missing #[armonik(enum = \"full.proto.Name\")]",
            );
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
        errors.at(
            input.ident.span(),
            "#[armonik_macros::enumeration] expects an enum",
        );
        return Err(errors);
    };

    // Collect variants: unit variants matched by name, plus exactly one catch-all tuple variant
    // whose payload struct the derive emits.
    let mut named: Vec<(syn::Ident, String)> = Vec::new();
    let mut unknown: Option<(syn::Ident, syn::Ident)> = None;
    let mut has_std_default = false;
    for variant in &data.variants {
        has_std_default |= variant
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("default"));
        let Some(FieldAttrs { rename, .. }) = scan_attrs(
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
                    errors.at(
                        variant.ident.span(),
                        "the catch-all payload must be a bare type name; the derive emits \
                         that struct",
                    );
                    continue;
                };
                if unknown.replace((variant.ident.clone(), payload)).is_some() {
                    errors.at(variant.ident.span(), "#[armonik_macros::enumeration] expects exactly one catch-all tuple variant");
                }
            }
            _ => errors.push(syn::Error::new(
                variant.ident.span(),
                "#[armonik_macros::enumeration] variants must be unit variants or the single \
                 catch-all tuple variant",
            )),
        }
    }
    let Some((unknown_variant, payload)) = unknown else {
        errors.at(
            input.ident.span(),
            "#[armonik_macros::enumeration] requires a catch-all tuple variant, \
             e.g. `Unknown(UnknownTaskStatus)`",
        );
        return Err(errors);
    };

    // Match every named variant against every proto enum; they must agree.
    let mut resolved: Vec<EnumValue> = Vec::new();
    let mut zero_variant = None;
    for (ident, proto_name) in &named {
        let mut number: Option<i32> = None;
        let mut docs: Vec<String> = Vec::new();
        for (enum_name, meta) in &proto_enums {
            let simple = enum_name.rsplit('.').next().unwrap_or(enum_name);
            let matched = meta.values.iter().position(|(value_name, _)| {
                value_name == proto_name
                    || crate::names::variant_name(simple, value_name) == *proto_name
            });
            match matched.map(|at| (at, &meta.values[at].1)) {
                Some((at, value)) => {
                    // Unified enums agree on their values, so the first one documents them.
                    if docs.is_empty() {
                        docs = meta.value_docs[at].clone();
                    }
                    if *number.get_or_insert(*value) != *value {
                        errors.at(
                            ident.span(),
                            format!("unified proto enums disagree on the value of `{proto_name}`"),
                        );
                    }
                }
                None => {
                    let available = meta
                        .values
                        .iter()
                        .map(|(value_name, _)| crate::names::variant_name(simple, value_name))
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
            resolved.push(EnumValue {
                ident: ident.clone(),
                number,
                docs,
            });
        }
    }

    // Completeness: every proto value needs a named variant, except the zero one, which the
    // catch-all covers losslessly and the emitted `UNSPECIFIED` const names.
    for (enum_name, meta) in &proto_enums {
        let simple = enum_name.rsplit('.').next().unwrap_or(enum_name);
        for (value_name, value) in &meta.values {
            let mapped = crate::names::variant_name(simple, value_name);
            let covered = named
                .iter()
                .any(|(_, proto_name)| *proto_name == mapped || proto_name == value_name);
            if !(covered || *value == 0) {
                errors.at(
                    input.ident.span(),
                    format!(
                        "proto enum value `{enum_name}.{value_name}` (= {value}) is not \
                         covered by any Rust variant"
                    ),
                );
            }
        }
    }

    errors.into_result()?;

    let docs = match &mode {
        EnumMode::Plain { .. } => proto_enums[0].1.docs.clone(),
        EnumMode::Transparent { names, .. } => names
            .first()
            .and_then(|name| index.messages.get(name))
            .map(|meta| meta.docs.clone())
            .unwrap_or_default(),
    };

    Ok(EnumPlan {
        ident: input.ident.clone(),
        unknown_variant,
        payload,
        docs,
        named: resolved,
        zero_variant,
        has_std_default,
        mode,
        fingerprint: index.fingerprint,
        absorbs,
    })
}

/// The wire half. A plain proto enum is an `int32` varint, so it implements `ProtoField` directly;
/// a transparent wrapper chain is message-shaped and goes through the same bundle as every struct.
/// These are the crate's two families, and the enumeration is the one type that can be either; the
/// choice is read off [`EnumMode`] here because no other macro has one.
pub(crate) fn wire(plan: &EnumPlan) -> TokenStream {
    match &plan.mode {
        EnumMode::Plain { names } => plain_wire(plan, names),
        EnumMode::Transparent { names, path } => transparent_wire(plan, names, path),
    }
}

fn transparent_wire(plan: &EnumPlan, names: &[String], path: &[u32]) -> TokenStream {
    // The chain as a codec type rather than a runtime walk over the tags: the enum at the bottom,
    // one `Wrapper` per level above it, and the outermost tag written by the message itself.
    let (root, nested) = path.split_first().expect("non-empty wrapper path");
    let codec = nested.iter().rev().fold(
        quote!(crate::codec::adapters::EnumLeaf),
        |inner, tag| quote!(crate::codec::adapters::Wrapper<#inner, #tag>),
    );
    message_shaped(
        &plan.ident,
        &syn::Generics::default(),
        plan.fingerprint,
        names,
        true,
        // The variants were checked against the proto enum by resolution; there is no field here
        // whose Rust type a const-assert could check.
        TokenStream::new(),
        MessageBodies {
            encode_raw: quote! {
                <#codec as crate::codec::ProtoAdapter<Self>>::encode_field(#root, self, buf);
            },
            merge_field: quote! {
                if tag == #root {
                    <#codec as crate::codec::ProtoAdapter<Self>>::merge_field(
                        wire_type, self, buf, ctx,
                    )
                } else {
                    ::prost::encoding::skip_field(wire_type, tag, buf, ctx)
                }
            },
            encoded_len: quote! {
                <#codec as crate::codec::ProtoAdapter<Self>>::encoded_len_field(#root, self)
            },
            // Zero, absent and present-but-empty carry no information at any depth of the chain.
            normalize: vec![quote! { crate::differential::wrapper_chain(message); }],
        },
    )
}

/// A proto enum on the wire: an `int32` varint, reached through `ProtoField` rather than through the
/// `Msg` blanket, because a proto enum is not a message.
fn plain_wire(plan: &EnumPlan, names: &[String]) -> TokenStream {
    let ident = &plan.ident;
    let tripwire = tripwire(plan.fingerprint);
    quote! {
        const _: () = { #tripwire };

        impl crate::codec::ProtoField for #ident {
            const SHAPE: crate::codec::Shape =
                crate::codec::Shape::enumeration(&[#(#names),*]);

            fn encode_field(tag: u32, value: &Self, buf: &mut impl ::prost::bytes::BufMut) {
                crate::codec::enumeration::encode(tag, value, buf);
            }

            fn merge_field(
                wire_type: ::prost::encoding::WireType,
                value: &mut Self,
                buf: &mut impl ::prost::bytes::Buf,
                ctx: ::prost::encoding::DecodeContext,
            ) -> ::core::result::Result<(), ::prost::DecodeError> {
                crate::codec::enumeration::merge(wire_type, value, buf, ctx)
            }

            fn encoded_len_field(tag: u32, value: &Self) -> usize {
                crate::codec::enumeration::encoded_len(tag, value)
            }

            // Whatever the enum calls it, the zero value is the one an implicit-presence field
            // leaves out.
            fn is_zero(value: &Self) -> bool {
                i32::from(*value) == 0
            }

            fn encode_repeated(tag: u32, values: &[Self], buf: &mut impl ::prost::bytes::BufMut) {
                crate::codec::enumeration::encode_repeated(tag, values, buf);
            }

            fn encoded_len_repeated(tag: u32, values: &[Self]) -> usize {
                crate::codec::enumeration::encoded_len_repeated(tag, values)
            }

            fn merge_repeated(
                wire_type: ::prost::encoding::WireType,
                values: &mut ::std::vec::Vec<Self>,
                buf: &mut impl ::prost::bytes::Buf,
                ctx: ::prost::encoding::DecodeContext,
            ) -> ::core::result::Result<(), ::prost::DecodeError> {
                crate::codec::enumeration::merge_repeated(wire_type, values, buf, ctx)
            }
        }
    }
}

/// The value-level half, which has nothing to do with the wire: the catch-all payload struct, the
/// two `i32` conversions, the `UNSPECIFIED` const, `Default`, the comparison traits and serde.
///
/// Emitted for both modes, and only ever by `#[armonik_macros::enumeration]`, which is why the entry
/// point calls it directly: no other macro has an `EnumPlan` to pass, so making it a slot in
/// something shared would give six other shapes a branch they can never take.
pub(crate) fn items(plan: &EnumPlan) -> TokenStream {
    let ident = &plan.ident;
    let unknown = &plan.unknown_variant;
    let payload = &plan.payload;
    let comparison = comparison(plan);
    let serde = serde(plan);

    let payload_doc = format!(
        "Raw value of an `{ident}` not known to this crate version (or the \
         unspecified zero value). Only constructible from a value this crate \
         version does not name, so a known value can never hide inside the \
         catch-all variant.",
    );

    let from_named_arms = plan.named.iter().map(|value| {
        let (variant, number) = (&value.ident, value.number);
        quote!(#number => Self::#variant)
    });
    let into_named_arms = plan.named.iter().map(|value| {
        let (variant, number) = (&value.ident, value.number);
        quote!(#ident::#variant => #number)
    });

    let default_impl = (!plan.has_std_default).then(|| {
        let default_expr = match &plan.zero_variant {
            Some(variant) => quote!(Self::#variant),
            None => quote!(Self::UNSPECIFIED),
        };
        quote! {
            impl ::core::default::Default for #ident {
                fn default() -> Self {
                    #default_expr
                }
            }
        }
    });
    let unspecified_const = plan.zero_variant.is_none().then(|| {
        quote! {
            impl #ident {
                /// The unspecified (zero) value. Compare with `==` rather than matching on it:
                /// the comparison traits are implemented in terms of the proto value, which
                /// makes the type non-structural-match.
                pub const UNSPECIFIED: Self = Self::#unknown(#payload(0));
            }
        }
    });

    quote! {
        #[doc = #payload_doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct #payload(i32);

        impl #payload {
            /// The raw protobuf value.
            pub const fn value(self) -> i32 {
                self.0
            }
        }

        impl ::core::convert::From<i32> for #ident {
            /// Normalizing: known values always map to their named variants.
            fn from(value: i32) -> Self {
                match value {
                    #(#from_named_arms,)*
                    value => Self::#unknown(#payload(value)),
                }
            }
        }

        impl ::core::convert::From<#ident> for i32 {
            fn from(value: #ident) -> Self {
                match value {
                    #(#into_named_arms,)*
                    #ident::#unknown(raw) => raw.0,
                }
            }
        }

        #unspecified_const

        #default_impl

        #comparison

        #serde
    }
}

/// The comparison traits, in terms of the proto value rather than the variant.
///
/// One value has two spellings, the named variant and the catch-all holding its number, and they
/// are one value: `From<i32>` normalizes, so the second only ever arrives from a peer this crate
/// version does not fully know. Deriving these would make the two unequal, unequally hashed, and
/// would order the catch-all by where it happens to sit rather than by what it holds. Emitted, so
/// the sites do not derive them; the resolver rejects it if they try.
fn comparison(plan: &EnumPlan) -> TokenStream {
    let ident = &plan.ident;
    quote! {
        impl ::core::cmp::PartialEq for #ident {
            fn eq(&self, other: &Self) -> bool {
                i32::from(*self) == i32::from(*other)
            }
        }

        impl ::core::cmp::Eq for #ident {}

        impl ::core::cmp::PartialOrd for #ident {
            fn partial_cmp(&self, other: &Self) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::option::Option::Some(::core::cmp::Ord::cmp(self, other))
            }
        }

        impl ::core::cmp::Ord for #ident {
            fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&i32::from(*self), &i32::from(*other))
            }
        }

        impl ::core::hash::Hash for #ident {
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                ::core::hash::Hash::hash(&i32::from(*self), state)
            }
        }
    }
}

/// `Serialize`/`Deserialize` over the proto value for the enum and its catch-all payload,
/// delegating the format to `codec::enum_serde`.
///
/// Hand-written for the same reason as the comparison traits above: a derived `Deserialize` is
/// generated in the module that owns the payload's private field, so it builds the catch-all
/// directly and a known value can hide inside it. The enum's goes through `From<i32>`, the
/// payload's rejects the values the enum names.
fn serde(plan: &EnumPlan) -> TokenStream {
    let ident = &plan.ident;
    let payload = &plan.payload;
    let values = plan.named.iter().map(|value| {
        let name = unraw(&value.ident);
        let number = value.number;
        quote!((#name, #number))
    });
    let name = unraw(ident);
    let catch_all = &plan.unknown_variant;
    let unknown = unraw(catch_all);
    quote! {
        #[cfg(feature = "serde")]
        const _: () = {
            const VALUES: crate::codec::enum_serde::Values = &[#(#values),*];

            impl ::serde::Serialize for #ident {
                fn serialize<S: ::serde::Serializer>(
                    &self,
                    serializer: S,
                ) -> ::core::result::Result<S::Ok, S::Error> {
                    crate::codec::enum_serde::serialize(VALUES, i32::from(*self), serializer)
                }
            }

            impl<'de> ::serde::Deserialize<'de> for #ident {
                fn deserialize<D: ::serde::Deserializer<'de>>(
                    deserializer: D,
                ) -> ::core::result::Result<Self, D::Error> {
                    crate::codec::enum_serde::deserialize(VALUES, #name, #unknown, deserializer)
                        .map(Self::from)
                }
            }

            // The payload is its number, in and out, like the catch-all that carries it.
            impl ::serde::Serialize for #payload {
                fn serialize<S: ::serde::Serializer>(
                    &self,
                    serializer: S,
                ) -> ::core::result::Result<S::Ok, S::Error> {
                    serializer.serialize_i32(self.0)
                }
            }

            impl<'de> ::serde::Deserialize<'de> for #payload {
                fn deserialize<D: ::serde::Deserializer<'de>>(
                    deserializer: D,
                ) -> ::core::result::Result<Self, D::Error> {
                    let value = <i32 as ::serde::Deserialize>::deserialize(deserializer)?;
                    // `From<i32>` already knows which numbers are named; a payload is what is left.
                    match #ident::from(value) {
                        #ident::#catch_all(raw) => ::core::result::Result::Ok(raw),
                        _ => ::core::result::Result::Err(
                            <D::Error as ::serde::de::Error>::custom(::std::format!(
                                "`{}` names {value}, so it cannot be an unknown value",
                                #name,
                            )),
                        ),
                    }
                }
            }
        };
    }
}

/// The traits the expansion implements itself, which a site must therefore not derive.
const IMPLEMENTED: [&str; 5] = ["PartialEq", "Eq", "PartialOrd", "Ord", "Hash"];

/// Reject `#[derive(PartialEq)]` and friends, rather than leaving rustc to report `E0119` at the
/// attribute with no hint about which of the two impls it should keep.
fn reject_implemented_derives(input: &syn::DeriveInput, errors: &mut Errors) {
    use syn::spanned::Spanned as _;

    for attr in &input.attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        let Ok(paths) = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
        ) else {
            continue;
        };
        for path in &paths {
            let Some(name) = IMPLEMENTED.iter().find(|name| path.is_ident(name)) else {
                continue;
            };
            errors.at(
                path.span(),
                format!(
                    "an enumeration must not derive `{name}`: the expansion implements it in \
                     terms of the proto value, so that the two spellings of one value (the named \
                     variant, and the catch-all holding its number) are one value"
                ),
            );
        }
    }
}
