//! `#[armonik_macros::enumeration]`: a proto enum, plain or flattened through a chain of
//! single-field wrapper messages.

use proc_macro2::{Span, TokenStream};
use quote::quote;

use crate::attr_site::{scan_attrs, unraw, Allowed, FieldAttrs};
use crate::attrs::{self, AttrItem, Errors};
use crate::descriptor::{DescriptorIndex, FieldKind};
use crate::emit::{message_impl, msg_impl, normalize_impl, registrations, tripwire};
use crate::matcher::{not_found, unknown_name};
use crate::plan::{EnumMode, EnumPlan};

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
    let mut resolved: Vec<(syn::Ident, i32)> = Vec::new();
    let mut zero_variant = None;
    for (ident, proto_name) in &named {
        let mut number: Option<i32> = None;
        for (enum_name, meta) in &proto_enums {
            let simple = enum_name.rsplit('.').next().unwrap_or(enum_name);
            let matched = meta.values.iter().find(|(value_name, _)| {
                value_name == proto_name
                    || crate::names::variant_name(simple, value_name) == *proto_name
            });
            match matched {
                Some((_, value)) => {
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
            resolved.push((ident.clone(), number));
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

    Ok(EnumPlan {
        ident: input.ident.clone(),
        unknown_variant,
        payload,
        named: resolved,
        zero_variant,
        has_std_default,
        mode,
        fingerprint: index.fingerprint,
        absorbs,
    })
}

pub(crate) fn enumeration(plan: &EnumPlan) -> TokenStream {
    let ident = &plan.ident;
    let unknown = &plan.unknown_variant;
    let payload = &plan.payload;
    let fingerprint = proc_macro2::Literal::u64_suffixed(plan.fingerprint);

    let payload_doc = format!(
        "Raw value of an `{ident}` not known to this crate version (or the \
         unspecified zero value). Only constructible by decoding, so a known \
         value can never hide inside the catch-all variant.",
    );

    let from_named_arms = plan
        .named
        .iter()
        .map(|(variant, number)| quote!(#number => Self::#variant));
    let into_named_arms = plan
        .named
        .iter()
        .map(|(variant, number)| quote!(#ident::#variant => #number));

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
                /// Unspecified (zero) value; usable in `match` patterns.
                pub const UNSPECIFIED: Self = Self::#unknown(#payload(0));
            }
        }
    });

    let proto_field = match &plan.mode {
        EnumMode::Plain { names } => quote! {
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
        },
        EnumMode::Transparent { names, path } => {
            let registrations = registrations(ident, names);
            let generics = syn::Generics::default();
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
            // Zero, absent and present-but-empty carry no information at any depth of the wrapper
            // chain.
            let normalize = normalize_impl(
                &impl_generics,
                ident,
                &ty_generics,
                where_clause,
                &[quote! { crate::differential::wrapper_chain(message); }],
            );
            // Transparent enums also ARE their outermost wrapper message, so they can stand for
            // whole RPC messages.
            let message = message_impl(
                &impl_generics,
                ident,
                &ty_generics,
                where_clause,
                quote! { crate::codec::wrapper_enum::encode_raw(&[#(#path),*], self, buf); },
                quote! {
                    crate::codec::wrapper_enum::merge_root_field(
                        &[#(#path),*], tag, wire_type, self, buf, ctx,
                    )
                },
                quote! { crate::codec::wrapper_enum::encoded_len_raw(&[#(#path),*], self) },
            );
            // As a field, the enum is its wrapper message: the blanket `ProtoField` impl frames the
            // `prost::Message` impl above.
            let msg = msg_impl(&impl_generics, ident, &ty_generics, where_clause, names);
            quote! {
                #registrations

                #normalize

                #message

                #msg
            }
        }
    };

    let tripwire = tripwire(&fingerprint);
    quote! {
        const _: () = { #tripwire };

        #[doc = #payload_doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

        #proto_field
    }
}
