//! Token emission from resolved plans.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

use crate::kind::{Cardinality, FieldKind};
use crate::resolve::{
    EnumMode, EnumPlan, FieldAccess, FieldCodec, FieldPlan, MessagePlan, StructStyle,
};

impl quote::ToTokens for FieldAccess {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            FieldAccess::Named(ident) => quote::ToTokens::to_tokens(ident, tokens),
            FieldAccess::Indexed(index) => quote::ToTokens::to_tokens(index, tokens),
        }
    }
}

/// Runtime path of a descriptor kind, for const-assert patterns.
fn kind_pattern(kind: &FieldKind) -> TokenStream {
    let variant = match kind {
        FieldKind::Double => quote!(Double),
        FieldKind::Float => quote!(Float),
        FieldKind::Int32 => quote!(Int32),
        FieldKind::Int64 => quote!(Int64),
        FieldKind::UInt32 => quote!(UInt32),
        FieldKind::UInt64 => quote!(UInt64),
        FieldKind::SInt32 => quote!(SInt32),
        FieldKind::SInt64 => quote!(SInt64),
        FieldKind::Fixed32 => quote!(Fixed32),
        FieldKind::Fixed64 => quote!(Fixed64),
        FieldKind::SFixed32 => quote!(SFixed32),
        FieldKind::SFixed64 => quote!(SFixed64),
        FieldKind::Bool => quote!(Bool),
        FieldKind::String => quote!(String),
        FieldKind::Bytes => quote!(Bytes),
        FieldKind::Message(_) => quote!(Message),
        FieldKind::Enum(_) => quote!(Enum),
    };
    quote!(crate::codec::FieldKind::#variant)
}

fn kind_description(kind: &FieldKind) -> String {
    match kind {
        FieldKind::Message(name) => format!("message {name}"),
        FieldKind::Enum(name) => format!("enum {name}"),
        other => format!("{other:?}").to_lowercase(),
    }
}

fn cardinality_pattern(cardinality: &Cardinality) -> TokenStream {
    match cardinality {
        Cardinality::Singular => quote!(crate::codec::Cardinality::Singular),
        Cardinality::Optional => quote!(crate::codec::Cardinality::Optional),
        Cardinality::Repeated { .. } => quote!(crate::codec::Cardinality::Repeated),
        Cardinality::Map { .. } => quote!(crate::codec::Cardinality::Map),
    }
}

fn cardinality_description(cardinality: &Cardinality) -> &'static str {
    match cardinality {
        Cardinality::Singular => "singular",
        Cardinality::Optional => "optional (explicit presence)",
        Cardinality::Repeated { .. } => "repeated",
        Cardinality::Map { .. } => "map",
    }
}

/// Const-asserts checking the Rust field type against the descriptor.
fn field_asserts(plan: &FieldPlan, type_ident: &syn::Ident) -> TokenStream {
    field_asserts_for(
        &plan.ty,
        plan.span,
        &plan.proto_path,
        &plan.checks,
        type_ident,
    )
}

fn field_asserts_for(
    ty: &syn::Type,
    span: proc_macro2::Span,
    proto_path: &str,
    checks: &crate::resolve::FieldChecks,
    type_ident: &syn::Ident,
) -> TokenStream {
    let mut asserts = TokenStream::new();

    if let Some(kind) = &checks.kind {
        let pattern = kind_pattern(kind);
        let message = format!(
            "armonik: field of `{type_ident}` maps to proto field `{proto_path}` of kind {}, \
             but the Rust type has a different wire kind",
            kind_description(kind),
        );
        asserts.extend(quote_spanned! {span=>
            assert!(
                matches!(<#ty as crate::codec::ProtoField>::KIND, #pattern),
                #message
            );
        });
    }

    if !checks.cardinalities.is_empty() {
        let patterns = checks
            .cardinalities
            .iter()
            .map(cardinality_pattern)
            .collect::<Vec<_>>();
        let expected = checks
            .cardinalities
            .iter()
            .map(cardinality_description)
            .collect::<Vec<_>>()
            .join(" or ");
        let message = format!(
            "armonik: proto field `{proto_path}` is {expected}, but the Rust type of the \
             corresponding field of `{type_ident}` has a different cardinality",
        );
        asserts.extend(quote_spanned! {span=>
            assert!(
                matches!(<#ty as crate::codec::ProtoField>::CARDINALITY, #(#patterns)|*),
                #message
            );
        });
    }

    for name in &checks.names {
        let message = format!(
            "armonik: proto field `{proto_path}` has type `{name}`, which the Rust type of \
             the corresponding field of `{type_ident}` does not stand for",
        );
        asserts.extend(quote_spanned! {span=>
            assert!(
                crate::codec::names_match(<#ty as crate::codec::ProtoField>::NAMES, #name),
                #message
            );
        });
    }

    if let Some((key, value)) = &checks.map_kinds {
        let key_pattern = kind_pattern(key);
        let value_pattern = kind_pattern(value);
        let message = format!(
            "armonik: proto map field `{proto_path}` is a map<{}, {}>, but the Rust map type \
             of the corresponding field of `{type_ident}` has different key/value kinds",
            kind_description(key),
            kind_description(value),
        );
        asserts.extend(quote_spanned! {span=>
            assert!(
                matches!(<#ty as crate::codec::ProtoField>::MAP_KEY_KIND, #key_pattern)
                    && matches!(<#ty as crate::codec::ProtoField>::MAP_VALUE_KIND, #value_pattern),
                #message
            );
        });
    }

    asserts
}

pub(crate) fn message(plan: &MessagePlan) -> TokenStream {
    let ident = &plan.ident;
    let proto_names = &plan.proto_names;
    let fingerprint = proc_macro2::Literal::u128_suffixed(plan.fingerprint);

    let mut generics = plan.generics.clone();
    for param in generics.type_params_mut() {
        param
            .bounds
            .push(syn::parse_quote!(crate::codec::ProtoField));
        param.bounds.push(syn::parse_quote!(::core::cmp::PartialEq));
        param.bounds.push(syn::parse_quote!(::core::fmt::Debug));
        param.bounds.push(syn::parse_quote!(::core::marker::Send));
        param.bounds.push(syn::parse_quote!(::core::marker::Sync));
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut encode_fragments = Vec::new();
    let mut merge_arms = Vec::new();
    let mut len_fragments = Vec::new();
    let mut clear_fragments = Vec::new();
    let mut wire_inits = Vec::new();
    let mut asserts = TokenStream::new();

    for field in &plan.fields {
        let access = &field.access;
        let ty = &field.ty;
        let tag = field.tag;

        // Wire-absence seed: singular message fields keep the API default
        // (like the historical `unwrap_or` conversions), everything else
        // decodes from the proto zero value.
        let keeps_api_default = matches!(&field.checks.kind, Some(FieldKind::Message(_)))
            && field.checks.cardinalities.contains(&Cardinality::Singular);
        wire_inits.push((
            access,
            if keeps_api_default {
                quote!(__default.#access)
            } else if matches!(field.codec, FieldCodec::Plain) {
                let ty = &field.ty;
                if plan.generic {
                    quote!(
                        if matches!(
                            <#ty as crate::codec::ProtoField>::KIND,
                            crate::codec::FieldKind::Message
                        ) && matches!(
                            <#ty as crate::codec::ProtoField>::CARDINALITY,
                            crate::codec::Cardinality::Singular
                        ) {
                            __default.#access
                        } else {
                            <#ty as crate::codec::ProtoField>::wire_default()
                        }
                    )
                } else {
                    quote!(<#ty as crate::codec::ProtoField>::wire_default())
                }
            } else {
                quote!(::core::default::Default::default())
            },
        ));

        match &field.codec {
            FieldCodec::Plain => {
                encode_fragments.push(quote! {
                    if !<#ty as crate::codec::ProtoField>::is_default(&self.#access) {
                        <#ty as crate::codec::ProtoField>::encode_field(#tag, &self.#access, buf);
                    }
                });
                // Singular message fields: when the containing type has a
                // custom `Default` (e.g. `TaskOptions::max_duration` =
                // infinite), the decode seed differs from the proto zero
                // value. Wire occurrences must merge from the zero value
                // (absence keeps the seed), otherwise a partial message
                // would inherit pieces of the seed.
                let is_singular_message = matches!(&field.checks.kind, Some(FieldKind::Message(_)))
                    && field.checks.cardinalities.contains(&Cardinality::Singular);
                if plan.generic {
                    // The seed rule is decided at runtime for generic types.
                    merge_arms.push(quote! {
                        #tag => {
                            if matches!(
                                <#ty as crate::codec::ProtoField>::KIND,
                                crate::codec::FieldKind::Message
                            ) && matches!(
                                <#ty as crate::codec::ProtoField>::CARDINALITY,
                                crate::codec::Cardinality::Singular
                            ) {
                                let seed = <Self as ::core::default::Default>::default().#access;
                                let wire_zero = <#ty as crate::codec::ProtoField>::wire_default();
                                if seed != wire_zero && self.#access == seed {
                                    self.#access = wire_zero;
                                }
                            }
                            <#ty as crate::codec::ProtoField>::merge_field(
                                wire_type, &mut self.#access, buf, ctx,
                            )
                        }
                    });
                } else if is_singular_message {
                    merge_arms.push(quote! {
                        #tag => {
                            let seed = <Self as ::core::default::Default>::default().#access;
                            let wire_zero = <#ty as crate::codec::ProtoField>::wire_default();
                            if seed != wire_zero && self.#access == seed {
                                self.#access = wire_zero;
                            }
                            <#ty as crate::codec::ProtoField>::merge_field(
                                wire_type, &mut self.#access, buf, ctx,
                            )
                        }
                    });
                } else {
                    merge_arms.push(quote! {
                        #tag => <#ty as crate::codec::ProtoField>::merge_field(
                            wire_type, &mut self.#access, buf, ctx,
                        )
                    });
                }
                len_fragments.push(quote! {
                    if !<#ty as crate::codec::ProtoField>::is_default(&self.#access) {
                        len += <#ty as crate::codec::ProtoField>::encoded_len_field(#tag, &self.#access);
                    }
                });
                clear_fragments.push(quote! {
                    <#ty as crate::codec::ProtoField>::clear_field(&mut self.#access);
                });
                asserts.extend(field_asserts(field, ident));
            }
            FieldCodec::Adapter(adapter) => {
                encode_fragments.push(quote! {
                    if !<#adapter as crate::codec::ProtoAdapter<_>>::is_default(&self.#access) {
                        <#adapter as crate::codec::ProtoAdapter<_>>::encode_field(#tag, &self.#access, buf);
                    }
                });
                merge_arms.push(quote! {
                    #tag => <#adapter as crate::codec::ProtoAdapter<_>>::merge_field(
                        wire_type, &mut self.#access, buf, ctx,
                    )
                });
                len_fragments.push(quote! {
                    if !<#adapter as crate::codec::ProtoAdapter<_>>::is_default(&self.#access) {
                        len += <#adapter as crate::codec::ProtoAdapter<_>>::encoded_len_field(#tag, &self.#access);
                    }
                });
                clear_fragments.push(quote! {
                    <#adapter as crate::codec::ProtoAdapter<_>>::clear_field(&mut self.#access);
                });
            }
            FieldCodec::OneofGroup { tags } => {
                encode_fragments.push(quote! {
                    <#ty as crate::codec::ProtoOneof>::encode_oneof(&self.#access, buf);
                });
                merge_arms.push(quote! {
                    #(#tags)|* => <#ty as crate::codec::ProtoOneof>::merge_oneof(
                        tag, wire_type, &mut self.#access, buf, ctx,
                    )
                });
                len_fragments.push(quote! {
                    len += <#ty as crate::codec::ProtoOneof>::encoded_len_oneof(&self.#access);
                });
                clear_fragments.push(quote! {
                    self.#access = ::core::default::Default::default();
                });
            }
        }
    }

    // `wire_default()` constructor; tuple structs need positional order.
    let wire_default_body = match plan.style {
        StructStyle::Unit => quote!(Self),
        StructStyle::Named => {
            let inits = wire_inits
                .iter()
                .map(|(access, init)| quote!(#access: #init));
            quote!(Self { #(#inits),* })
        }
        StructStyle::Tuple => {
            let mut ordered: Vec<_> = wire_inits
                .iter()
                .map(|(access, init)| {
                    let index = match access {
                        FieldAccess::Indexed(index) => index.index,
                        FieldAccess::Named(_) => unreachable!("tuple structs have indexed fields"),
                    };
                    (index, init)
                })
                .collect();
            ordered.sort_by_key(|(index, _)| *index);
            let inits = ordered.into_iter().map(|(_, init)| init);
            quote!(Self(#(#inits),*))
        }
    };

    let tripwire_message = "armonik: a derive was expanded against a stale protobuf descriptor; \
                            rebuild the crate";
    quote! {
        const _: () = {
            assert!(
                crate::__schema::DESCRIPTOR_FINGERPRINT == #fingerprint,
                #tripwire_message
            );
            #asserts
        };

        impl #impl_generics ::prost::Message for #ident #ty_generics #where_clause {
            fn encode_raw(&self, buf: &mut impl ::prost::bytes::BufMut) {
                #(#encode_fragments)*
            }

            fn merge_field(
                &mut self,
                tag: u32,
                wire_type: ::prost::encoding::WireType,
                buf: &mut impl ::prost::bytes::Buf,
                ctx: ::prost::encoding::DecodeContext,
            ) -> ::core::result::Result<(), ::prost::DecodeError> {
                match tag {
                    #(#merge_arms,)*
                    _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }

            fn encoded_len(&self) -> usize {
                #[allow(unused_mut)]
                let mut len = 0;
                #(#len_fragments)*
                len
            }

            fn clear(&mut self) {
                #(#clear_fragments)*
            }

            // Seed from `wire_default` instead of the provided methods'
            // `Self::default()`: custom API defaults must not leak into
            // fields that are absent on the wire (except where documented).
            fn decode(
                mut buf: impl ::prost::bytes::Buf,
            ) -> ::core::result::Result<Self, ::prost::DecodeError>
            where
                Self: ::core::default::Default,
            {
                let mut message = <Self as crate::codec::ProtoField>::wire_default();
                ::prost::Message::merge(&mut message, &mut buf)?;
                ::core::result::Result::Ok(message)
            }

            fn decode_length_delimited(
                buf: impl ::prost::bytes::Buf,
            ) -> ::core::result::Result<Self, ::prost::DecodeError>
            where
                Self: ::core::default::Default,
            {
                let mut message = <Self as crate::codec::ProtoField>::wire_default();
                ::prost::Message::merge_length_delimited(&mut message, buf)?;
                ::core::result::Result::Ok(message)
            }
        }

        impl #impl_generics crate::codec::ProtoField for #ident #ty_generics #where_clause {
            const KIND: crate::codec::FieldKind = crate::codec::FieldKind::Message;
            const NAMES: &'static [&'static str] = &[#(#proto_names),*];

            fn encode_field(tag: u32, value: &Self, buf: &mut impl ::prost::bytes::BufMut) {
                crate::codec::message::encode(tag, value, buf);
            }

            fn merge_field(
                wire_type: ::prost::encoding::WireType,
                value: &mut Self,
                buf: &mut impl ::prost::bytes::Buf,
                ctx: ::prost::encoding::DecodeContext,
            ) -> ::core::result::Result<(), ::prost::DecodeError> {
                crate::codec::message::merge(wire_type, value, buf, ctx)
            }

            fn encoded_len_field(tag: u32, value: &Self) -> usize {
                crate::codec::message::encoded_len(tag, value)
            }

            fn is_default(value: &Self) -> bool {
                crate::codec::message::is_default(value)
            }

            fn encode_repeated(tag: u32, values: &[Self], buf: &mut impl ::prost::bytes::BufMut) {
                crate::codec::message::encode_repeated(tag, values, buf);
            }

            fn encoded_len_repeated(tag: u32, values: &[Self]) -> usize {
                crate::codec::message::encoded_len_repeated(tag, values)
            }

            // `merge_repeated` deliberately uses the trait default, which
            // seeds new elements with `wire_default()`.

            fn wire_default() -> Self {
                #[allow(unused_variables)]
                let __default = <Self as ::core::default::Default>::default();
                #wire_default_body
            }
        }
    }
}

pub(crate) fn enumeration(plan: &EnumPlan) -> TokenStream {
    let ident = &plan.ident;
    let other = &plan.other_variant;
    let payload = &plan.payload;
    let fingerprint = proc_macro2::Literal::u128_suffixed(plan.fingerprint);

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
                pub const UNSPECIFIED: Self = Self::#other(#payload(0));
            }
        }
    });

    let proto_field = match &plan.mode {
        EnumMode::Plain { names } => quote! {
            impl crate::codec::ProtoField for #ident {
                const KIND: crate::codec::FieldKind = crate::codec::FieldKind::Enum;
                const NAMES: &'static [&'static str] = &[#(#names),*];

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

                fn is_default(value: &Self) -> bool {
                    crate::codec::enumeration::is_default(value)
                }

                fn wire_default() -> Self {
                    Self::from(0)
                }

                fn clear_field(value: &mut Self) {
                    *value = Self::from(0);
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
        EnumMode::Transparent { names, path } => quote! {
            impl crate::codec::ProtoField for #ident {
                const KIND: crate::codec::FieldKind = crate::codec::FieldKind::Message;
                const NAMES: &'static [&'static str] = &[#(#names),*];

                fn encode_field(tag: u32, value: &Self, buf: &mut impl ::prost::bytes::BufMut) {
                    crate::codec::wrapper_enum::encode(tag, &[#(#path),*], value, buf);
                }

                fn merge_field(
                    wire_type: ::prost::encoding::WireType,
                    value: &mut Self,
                    buf: &mut impl ::prost::bytes::Buf,
                    ctx: ::prost::encoding::DecodeContext,
                ) -> ::core::result::Result<(), ::prost::DecodeError> {
                    crate::codec::wrapper_enum::merge(&[#(#path),*], wire_type, value, buf, ctx)
                }

                fn encoded_len_field(tag: u32, value: &Self) -> usize {
                    crate::codec::wrapper_enum::encoded_len(tag, &[#(#path),*], value)
                }

                fn is_default(value: &Self) -> bool {
                    crate::codec::wrapper_enum::is_default(value)
                }

                fn wire_default() -> Self {
                    Self::from(0)
                }

                fn clear_field(value: &mut Self) {
                    *value = Self::from(0);
                }
            }
        },
    };

    let tripwire_message = "armonik: a derive was expanded against a stale protobuf descriptor; \
                            rebuild the crate";
    quote! {
        const _: () = assert!(
            crate::__schema::DESCRIPTOR_FINGERPRINT == #fingerprint,
            #tripwire_message
        );

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
                    other => Self::#other(#payload(other)),
                }
            }
        }

        impl ::core::convert::From<#ident> for i32 {
            fn from(value: #ident) -> Self {
                match value {
                    #(#into_named_arms,)*
                    #ident::#other(raw) => raw.0,
                }
            }
        }

        #unspecified_const

        #default_impl

        #proto_field
    }
}

pub(crate) fn oneof(plan: &crate::resolve::OneofPlan) -> TokenStream {
    use crate::resolve::OneofVariantShape;

    let ident = &plan.ident;
    let proto_name = &plan.proto_name;
    let tags = &plan.tags;
    let fingerprint = proc_macro2::Literal::u128_suffixed(plan.fingerprint);

    let mut encode_arms = Vec::new();
    let mut len_arms = Vec::new();
    let mut merge_arms = Vec::new();
    let mut asserts = TokenStream::new();

    for variant in &plan.variants {
        let var = &variant.ident;
        let tag = variant.tag;
        match &variant.shape {
            OneofVariantShape::Payload { ty, checks } => {
                // Oneof presence is significant: the member is always
                // emitted, even with a default payload.
                encode_arms.push(quote! {
                    Self::#var(payload) => {
                        <#ty as crate::codec::ProtoField>::encode_field(#tag, payload, buf);
                    }
                });
                len_arms.push(quote! {
                    Self::#var(payload) => {
                        <#ty as crate::codec::ProtoField>::encoded_len_field(#tag, payload)
                    }
                });
                merge_arms.push(quote! {
                    #tag => {
                        let mut payload = if let Self::#var(payload) = value {
                            ::std::mem::take(payload)
                        } else {
                            <#ty as crate::codec::ProtoField>::wire_default()
                        };
                        <#ty as crate::codec::ProtoField>::merge_field(
                            wire_type, &mut payload, buf, ctx,
                        )?;
                        *value = Self::#var(payload);
                        ::core::result::Result::Ok(())
                    }
                });
                asserts.extend(field_asserts_for(
                    ty,
                    variant.span,
                    &variant.proto_path,
                    checks,
                    ident,
                ));
            }
            OneofVariantShape::MarkerBool => {
                encode_arms.push(quote! {
                    Self::#var => {
                        <bool as crate::codec::ProtoField>::encode_field(#tag, &true, buf);
                    }
                });
                len_arms.push(quote! {
                    Self::#var => {
                        <bool as crate::codec::ProtoField>::encoded_len_field(#tag, &true)
                    }
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
            OneofVariantShape::MarkerMessage => {
                encode_arms.push(quote! {
                    Self::#var => {
                        ::prost::encoding::encode_key(
                            #tag,
                            ::prost::encoding::WireType::LengthDelimited,
                            buf,
                        );
                        ::prost::encoding::encode_varint(0, buf);
                    }
                });
                len_arms.push(quote! {
                    Self::#var => {
                        ::prost::encoding::encoded_len_varint(u64::from(#tag) << 3) + 1
                    }
                });
                merge_arms.push(quote! {
                    #tag => {
                        ::prost::encoding::skip_field(wire_type, tag, buf, ctx)?;
                        *value = Self::#var;
                        ::core::result::Result::Ok(())
                    }
                });
            }
            OneofVariantShape::Inline { parts } => {
                let part_idents: Vec<_> = parts.iter().map(|part| &part.ident).collect();
                let part_tys: Vec<_> = parts.iter().map(|part| &part.ty).collect();
                let part_tags: Vec<_> = parts.iter().map(|part| part.tag).collect();
                let part_seeds: Vec<_> = parts
                    .iter()
                    .map(|part| {
                        let ty = &part.ty;
                        // Inline parts have no containing Default to inherit
                        // from; they seed from the wire default.
                        let _ = part.keeps_api_default;
                        quote!(<#ty as crate::codec::ProtoField>::wire_default())
                    })
                    .collect();

                encode_arms.push(quote! {
                    Self::#var { #(#part_idents),* } => {
                        #[allow(unused_mut)]
                        let mut body_len = 0;
                        #(
                            if !<#part_tys as crate::codec::ProtoField>::is_default(#part_idents) {
                                body_len += <#part_tys as crate::codec::ProtoField>::encoded_len_field(#part_tags, #part_idents);
                            }
                        )*
                        ::prost::encoding::encode_key(
                            #tag,
                            ::prost::encoding::WireType::LengthDelimited,
                            buf,
                        );
                        ::prost::encoding::encode_varint(body_len as u64, buf);
                        #(
                            if !<#part_tys as crate::codec::ProtoField>::is_default(#part_idents) {
                                <#part_tys as crate::codec::ProtoField>::encode_field(#part_tags, #part_idents, buf);
                            }
                        )*
                    }
                });
                len_arms.push(quote! {
                    Self::#var { #(#part_idents),* } => {
                        #[allow(unused_mut)]
                        let mut body_len = 0;
                        #(
                            if !<#part_tys as crate::codec::ProtoField>::is_default(#part_idents) {
                                body_len += <#part_tys as crate::codec::ProtoField>::encoded_len_field(#part_tags, #part_idents);
                            }
                        )*
                        ::prost::encoding::encoded_len_varint(u64::from(#tag) << 3)
                            + ::prost::encoding::encoded_len_varint(body_len as u64)
                            + body_len
                    }
                });
                merge_arms.push(quote! {
                    #tag => {
                        ::prost::encoding::check_wire_type(
                            ::prost::encoding::WireType::LengthDelimited,
                            wire_type,
                        )?;
                        #[allow(unused_parens)]
                        let (#(mut #part_idents),*) = if let Self::#var { #(#part_idents),* } = value {
                            (#(::std::mem::take(#part_idents)),*)
                        } else {
                            (#(#part_seeds),*)
                        };
                        let len = ::prost::encoding::decode_varint(buf)? as usize;
                        if ::prost::bytes::Buf::remaining(buf) < len {
                            // prost offers no other public constructor.
                            #[allow(deprecated)]
                            return ::core::result::Result::Err(
                                ::prost::DecodeError::new("buffer underflow"),
                            );
                        }
                        let mut body = ::prost::bytes::Buf::take(buf, len);
                        while ::prost::bytes::Buf::has_remaining(&body) {
                            let (tag, wire_type) = ::prost::encoding::decode_key(&mut body)?;
                            match tag {
                                #(
                                    #part_tags => <#part_tys as crate::codec::ProtoField>::merge_field(
                                        wire_type, &mut #part_idents, &mut body, ctx.clone(),
                                    )?,
                                )*
                                _ => ::prost::encoding::skip_field(
                                    wire_type, tag, &mut body, ctx.clone(),
                                )?,
                            }
                        }
                        *value = Self::#var { #(#part_idents),* };
                        ::core::result::Result::Ok(())
                    }
                });
                for part in parts {
                    asserts.extend(field_asserts_for(
                        &part.ty,
                        part.span,
                        &part.proto_path,
                        &part.checks,
                        ident,
                    ));
                }
            }
        }
    }

    let default_encode_arm = plan.default_variant.as_ref().map(|var| {
        quote! { Self::#var => {} }
    });
    let default_len_arm = plan.default_variant.as_ref().map(|var| {
        quote! { Self::#var => 0, }
    });

    let whole_message = plan.whole_message.then(|| {
        quote! {
            impl ::prost::Message for #ident {
                fn encode_raw(&self, buf: &mut impl ::prost::bytes::BufMut) {
                    crate::codec::ProtoOneof::encode_oneof(self, buf);
                }

                fn merge_field(
                    &mut self,
                    tag: u32,
                    wire_type: ::prost::encoding::WireType,
                    buf: &mut impl ::prost::bytes::Buf,
                    ctx: ::prost::encoding::DecodeContext,
                ) -> ::core::result::Result<(), ::prost::DecodeError> {
                    match tag {
                        #(#tags)|* => crate::codec::ProtoOneof::merge_oneof(
                            tag, wire_type, self, buf, ctx,
                        ),
                        _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
                    }
                }

                fn encoded_len(&self) -> usize {
                    crate::codec::ProtoOneof::encoded_len_oneof(self)
                }

                fn clear(&mut self) {
                    *self = ::core::default::Default::default();
                }
            }

            impl crate::codec::ProtoField for #ident {
                const KIND: crate::codec::FieldKind = crate::codec::FieldKind::Message;
                const NAMES: &'static [&'static str] = &[#proto_name];

                fn encode_field(tag: u32, value: &Self, buf: &mut impl ::prost::bytes::BufMut) {
                    crate::codec::message::encode(tag, value, buf);
                }

                fn merge_field(
                    wire_type: ::prost::encoding::WireType,
                    value: &mut Self,
                    buf: &mut impl ::prost::bytes::Buf,
                    ctx: ::prost::encoding::DecodeContext,
                ) -> ::core::result::Result<(), ::prost::DecodeError> {
                    crate::codec::message::merge(wire_type, value, buf, ctx)
                }

                fn encoded_len_field(tag: u32, value: &Self) -> usize {
                    crate::codec::message::encoded_len(tag, value)
                }

                fn is_default(value: &Self) -> bool {
                    crate::codec::message::is_default(value)
                }

                fn encode_repeated(tag: u32, values: &[Self], buf: &mut impl ::prost::bytes::BufMut) {
                    crate::codec::message::encode_repeated(tag, values, buf);
                }

                fn encoded_len_repeated(tag: u32, values: &[Self]) -> usize {
                    crate::codec::message::encoded_len_repeated(tag, values)
                }
            }
        }
    });

    let tripwire_message = "armonik: a derive was expanded against a stale protobuf descriptor; \
                            rebuild the crate";
    quote! {
        const _: () = {
            assert!(
                crate::__schema::DESCRIPTOR_FINGERPRINT == #fingerprint,
                #tripwire_message
            );
            #asserts
        };

        impl crate::codec::ProtoOneof for #ident {
            fn encode_oneof(value: &Self, buf: &mut impl ::prost::bytes::BufMut) {
                match value {
                    #(#encode_arms)*
                    #default_encode_arm
                }
            }

            fn merge_oneof(
                tag: u32,
                wire_type: ::prost::encoding::WireType,
                value: &mut Self,
                buf: &mut impl ::prost::bytes::Buf,
                ctx: ::prost::encoding::DecodeContext,
            ) -> ::core::result::Result<(), ::prost::DecodeError> {
                match tag {
                    #(#merge_arms)*
                    _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }

            fn encoded_len_oneof(value: &Self) -> usize {
                match value {
                    #(#len_arms)*
                    #default_len_arm
                }
            }
        }

        #whole_message
    }
}
