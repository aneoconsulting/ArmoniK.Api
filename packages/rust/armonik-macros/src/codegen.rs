//! Token emission from resolved plans.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

use crate::kind::{Cardinality, FieldKind};
use crate::resolve::{EnumMode, EnumPlan, FieldAccess, FieldCodec, FieldPlan, MessagePlan};

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

/// Register the type's proto name(s) via `armonik-types`' `register!` macro —
/// the single home of the registry's layout (the `linkme` slice, the feature
/// gates, and the `_differential` round-trip/`Normalize` hooks). A plain type
/// registers `message:`; a `#[armonik(replace(...))]` type registers `replace:`
/// so the shared proto name stays unambiguous. Empty `names` (generic types,
/// covered through their aliases) register nothing.
pub(crate) fn registrations(
    ident: &syn::Ident,
    names: &[String],
    replace: Option<&crate::attrs::ReplaceSpec>,
) -> TokenStream {
    if names.is_empty() {
        return TokenStream::new();
    }
    match replace {
        None => quote! {
            crate::register!(message: #ident, #(#names),*);
        },
        Some(spec) => {
            let service = &spec.service;
            let method = &spec.method;
            let target = &spec.target;
            let direction = match spec.direction {
                crate::attrs::Direction::Input => quote!(input),
                crate::attrs::Direction::Output => quote!(output),
            };
            let mut out = TokenStream::new();
            for name in names {
                out.extend(quote! {
                    crate::register!(replace: #ident,
                        message = #name,
                        service = #service,
                        method = #method,
                        #direction,
                        target = #target);
                });
            }
            out
        }
    }
}

/// Register proto messages a flattening construct swallows into its parent
/// (a `with` adapter's `absorbs`, a transparent chain's middle wrappers, an
/// inline struct variant's message), so they have no Rust type of their own.
/// `armonik`'s build script prunes them from the stubs and the differential
/// harness counts them as covered.
pub(crate) fn absorbed_registrations(names: &[String]) -> TokenStream {
    if names.is_empty() {
        return TokenStream::new();
    }
    quote! {
        crate::register!(absorbed: #(#names),*);
    }
}

/// Test-only `Normalize` impl: the type's value-level projection for the
/// differential harness, stitched from the same constructs that shape the
/// codec (adapters, presence markers, wrapper chains, oneof delegation).
fn normalize_impl(
    impl_generics: &syn::ImplGenerics,
    ident: &syn::Ident,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    fragments: &[TokenStream],
) -> TokenStream {
    quote! {
        #[cfg(feature = "_differential")]
        impl #impl_generics crate::differential::Normalize for #ident #ty_generics #where_clause {
            fn normalize(
                message: &mut crate::differential::prost_reflect::DynamicMessage,
            ) {
                let _ = &message;
                #(#fragments)*
            }
        }
    }
}

/// The compile-time tripwire: a const-assert that the descriptor fingerprint
/// the derive was expanded against still matches the schema baked into the
/// crate. Emitted (in a `const _: () = { ... };` block) by every derive.
fn tripwire(fingerprint: &proc_macro2::Literal) -> TokenStream {
    quote! {
        assert!(
            crate::__schema::DESCRIPTOR_FINGERPRINT == #fingerprint,
            "armonik: a derive was expanded against a stale protobuf descriptor; \
             rebuild the crate"
        );
    }
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

    if plan.transparent {
        return transparent_message(
            plan,
            &impl_generics,
            &ty_generics,
            where_clause,
            fingerprint,
        );
    }

    let mut encode_fragments = Vec::new();
    let mut merge_arms = Vec::new();
    let mut len_fragments = Vec::new();
    let mut clear_fragments = Vec::new();
    let mut normalize_fragments = Vec::new();
    let mut asserts = TokenStream::new();

    for field in &plan.fields {
        let access = &field.access;
        let ty = &field.ty;
        let tag = field.tag;

        match &field.codec {
            FieldCodec::Plain => {
                encode_fragments.push(quote! {
                    if !<#ty as crate::codec::ProtoField>::is_default(&self.#access) {
                        <#ty as crate::codec::ProtoField>::encode_field(#tag, &self.#access, buf);
                    }
                });
                merge_arms.push(quote! {
                    #tag => <#ty as crate::codec::ProtoField>::merge_field(
                        wire_type, &mut self.#access, buf, ctx,
                    )
                });
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
                normalize_fragments.push(quote! {
                    <#adapter as crate::codec::ProtoAdapter<#ty>>::normalize_dynamic(message, #tag);
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
                normalize_fragments.push(quote! {
                    <#ty as crate::differential::Normalize>::normalize(message);
                });
            }
        }
    }

    let registrations = registrations(ident, proto_names, plan.replace.as_ref());
    let normalize = normalize_impl(
        &impl_generics,
        ident,
        &ty_generics,
        where_clause,
        &normalize_fragments,
    );
    let proto_field = message_proto_field(&impl_generics, ident, &ty_generics, where_clause, proto_names);
    let tripwire = tripwire(&fingerprint);
    quote! {
        const _: () = {
            #tripwire
            #asserts
        };

        #registrations

        #normalize

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
        }

        #proto_field
    }
}

/// The `ProtoField` impl for a message type, delegating to `codec::message`.
/// Shared by the plain-struct and `transparent` codegen paths.
fn message_proto_field(
    impl_generics: &syn::ImplGenerics,
    ident: &syn::Ident,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    proto_names: &[String],
) -> TokenStream {
    quote! {
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
        }
    }
}

/// Codegen for a `#[armonik(transparent)]` struct: a single-field newtype
/// whose `prost::Message` impl delegates entirely to the field, so it is
/// wire-identical to the inner message and can stand for a whole RPC message
/// in the stub signatures. The `Normalize` projection delegates likewise.
fn transparent_message(
    plan: &MessagePlan,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    fingerprint: proc_macro2::Literal,
) -> TokenStream {
    let ident = &plan.ident;
    let proto_names = &plan.proto_names;
    let field = &plan.fields[0];
    let access = &field.access;
    let ty = &field.ty;

    let registrations = registrations(ident, proto_names, plan.replace.as_ref());
    let proto_field = message_proto_field(impl_generics, ident, ty_generics, where_clause, proto_names);
    let tripwire = tripwire(&fingerprint);
    quote! {
        const _: () = { #tripwire };

        #registrations

        #[cfg(feature = "_differential")]
        impl #impl_generics crate::differential::Normalize for #ident #ty_generics #where_clause {
            fn normalize(
                message: &mut crate::differential::prost_reflect::DynamicMessage,
            ) {
                <#ty as crate::differential::Normalize>::normalize(message);
            }
        }

        impl #impl_generics ::prost::Message for #ident #ty_generics #where_clause {
            fn encode_raw(&self, buf: &mut impl ::prost::bytes::BufMut) {
                <#ty as ::prost::Message>::encode_raw(&self.#access, buf);
            }

            fn merge_field(
                &mut self,
                tag: u32,
                wire_type: ::prost::encoding::WireType,
                buf: &mut impl ::prost::bytes::Buf,
                ctx: ::prost::encoding::DecodeContext,
            ) -> ::core::result::Result<(), ::prost::DecodeError> {
                <#ty as ::prost::Message>::merge_field(&mut self.#access, tag, wire_type, buf, ctx)
            }

            fn encoded_len(&self) -> usize {
                <#ty as ::prost::Message>::encoded_len(&self.#access)
            }

            fn clear(&mut self) {
                <#ty as ::prost::Message>::clear(&mut self.#access);
            }
        }

        #proto_field
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
        EnumMode::Transparent { names, path } => {
            let registrations = registrations(ident, names, None);
            quote! {
                #registrations

                // Zero, absent and present-but-empty carry no information
                // at any depth of the wrapper chain.
                #[cfg(feature = "_differential")]
                impl crate::differential::Normalize for #ident {
                    fn normalize(
                        message: &mut crate::differential::prost_reflect::DynamicMessage,
                    ) {
                        crate::differential::wrapper_chain(message);
                    }
                }

                // Transparent enums also ARE their outermost wrapper message,
                // so they can stand for RPC messages in stub signatures.
                impl ::prost::Message for #ident {
                    fn encode_raw(&self, buf: &mut impl ::prost::bytes::BufMut) {
                        crate::codec::wrapper_enum::encode_raw(&[#(#path),*], self, buf);
                    }

                    fn merge_field(
                        &mut self,
                        tag: u32,
                        wire_type: ::prost::encoding::WireType,
                        buf: &mut impl ::prost::bytes::Buf,
                        ctx: ::prost::encoding::DecodeContext,
                    ) -> ::core::result::Result<(), ::prost::DecodeError> {
                        crate::codec::wrapper_enum::merge_root_field(
                            &[#(#path),*], tag, wire_type, self, buf, ctx,
                        )
                    }

                    fn encoded_len(&self) -> usize {
                        crate::codec::wrapper_enum::encoded_len_raw(&[#(#path),*], self)
                    }

                    fn clear(&mut self) {
                        *self = Self::from(0);
                    }
                }

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

                    fn clear_field(value: &mut Self) {
                        *value = Self::from(0);
                    }
                }
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

    if !plan.siblings.is_empty() {
        return oneof_with_siblings(plan);
    }

    let ident = &plan.ident;
    let proto_name = &plan.proto_name;
    let tags = &plan.tags;
    let fingerprint = proc_macro2::Literal::u128_suffixed(plan.fingerprint);

    let mut encode_arms = Vec::new();
    let mut len_arms = Vec::new();
    let mut merge_arms = Vec::new();
    let mut asserts = TokenStream::new();
    let mut normalize_fragments = Vec::new();

    for variant in &plan.variants {
        let var = &variant.ident;
        let tag = variant.tag;
        match &variant.shape {
            OneofVariantShape::SiblingPayload { .. } => {
                unreachable!("sibling variants are emitted by oneof_with_siblings")
            }
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
                            ::core::default::Default::default()
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
            OneofVariantShape::Adapter { ty, adapter } => {
                normalize_fragments.push(quote! {
                    <#adapter as crate::codec::ProtoAdapter<#ty>>::normalize_dynamic(
                        message, #tag,
                    );
                });
                // Oneof presence is significant: the member is always
                // emitted; the adapter's is_default is not consulted.
                encode_arms.push(quote! {
                    Self::#var(payload) => {
                        <#adapter as crate::codec::ProtoAdapter<#ty>>::encode_field(
                            #tag, payload, buf,
                        );
                    }
                });
                len_arms.push(quote! {
                    Self::#var(payload) => {
                        <#adapter as crate::codec::ProtoAdapter<#ty>>::encoded_len_field(
                            #tag, payload,
                        )
                    }
                });
                merge_arms.push(quote! {
                    #tag => {
                        let mut payload = if let Self::#var(payload) = value {
                            ::std::mem::take(payload)
                        } else {
                            ::core::default::Default::default()
                        };
                        <#adapter as crate::codec::ProtoAdapter<#ty>>::merge_field(
                            wire_type, &mut payload, buf, ctx,
                        )?;
                        *value = Self::#var(payload);
                        ::core::result::Result::Ok(())
                    }
                });
            }
            OneofVariantShape::MarkerBool => {
                // Only the member's presence survives (an explicit `false`
                // reads as set).
                normalize_fragments.push(quote! {
                    crate::differential::bool_marker(message, #tag);
                });
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
                        quote!(<#ty as ::core::default::Default>::default())
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
                        let mut body = crate::codec::read_delimited(buf)?;
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
        let registrations = registrations(ident, std::slice::from_ref(&plan.proto_name), None);
        let generics = syn::Generics::default();
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let proto_field = message_proto_field(
            &impl_generics,
            ident,
            &ty_generics,
            where_clause,
            std::slice::from_ref(proto_name),
        );
        quote! {
            #registrations

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

            #proto_field
        }
    });

    // Emitted for embedded oneofs too: the containing message's `Normalize`
    // delegates to it (the members live on the parent's dynamic message).
    let generics = syn::Generics::default();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let normalize = normalize_impl(
        &impl_generics,
        ident,
        &ty_generics,
        where_clause,
        &normalize_fragments,
    );

    let tripwire = tripwire(&fingerprint);
    quote! {
        const _: () = {
            #tripwire
            #asserts
        };

        #normalize

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

/// Emission for a whole-message enum with sibling fields: every variant
/// (including the "no member set" default) carries all non-oneof fields of
/// the message, which keeps the per-field merge stateless and
/// order-independent — a sibling occurrence merges into the current
/// variant's slot, and a member occurrence switches variants while carrying
/// the siblings over. The enum IS the message: it gets `prost::Message` and
/// `ProtoField` implementations, no `ProtoOneof`.
fn oneof_with_siblings(plan: &crate::resolve::OneofPlan) -> TokenStream {
    use crate::resolve::OneofVariantShape;

    let ident = &plan.ident;
    let proto_name = &plan.proto_name;
    let fingerprint = proc_macro2::Literal::u128_suffixed(plan.fingerprint);

    // Every variant ident (members + default), for patterns spanning all of
    // them; `pats` builds one pattern per variant binding a subset of the
    // sibling fields, avoiding unused-binding warnings.
    let variant_idents: Vec<&syn::Ident> = plan
        .variants
        .iter()
        .map(|variant| &variant.ident)
        .chain(plan.default_variant.iter())
        .collect();
    let pats = |bound: &[&syn::Ident]| -> Vec<TokenStream> {
        variant_idents
            .iter()
            .map(|variant| quote!(Self::#variant { #(#bound,)* .. }))
            .collect()
    };
    let sib_idents: Vec<&syn::Ident> = plan.siblings.iter().map(|sibling| &sibling.ident).collect();

    let mut asserts = TokenStream::new();
    for sibling in &plan.siblings {
        asserts.extend(field_asserts_for(
            &sibling.ty,
            sibling.span,
            &sibling.proto_path,
            &sibling.checks,
            ident,
        ));
    }

    // Sibling encode/len statements, keyed by tag so the member can be
    // interleaved in canonical tag order.
    let sibling_entries: Vec<(u32, TokenStream, TokenStream)> = plan
        .siblings
        .iter()
        .map(|sibling| {
            let sid = &sibling.ident;
            let sty = &sibling.ty;
            let stag = sibling.tag;
            (
                stag,
                quote! {
                    if !<#sty as crate::codec::ProtoField>::is_default(#sid) {
                        <#sty as crate::codec::ProtoField>::encode_field(#stag, #sid, buf);
                    }
                },
                quote! {
                    if !<#sty as crate::codec::ProtoField>::is_default(#sid) {
                        len += <#sty as crate::codec::ProtoField>::encoded_len_field(#stag, #sid);
                    }
                },
            )
        })
        .collect();

    let mut encode_arms = Vec::new();
    let mut len_arms = Vec::new();
    let mut merge_arms = Vec::new();
    let mut normalize_fragments = Vec::new();

    for variant in &plan.variants {
        let var = &variant.ident;
        let tag = variant.tag;
        let OneofVariantShape::SiblingPayload {
            payload,
            ty,
            adapter,
            checks,
        } = &variant.shape
        else {
            unreachable!("sibling-mode variants are always SiblingPayload");
        };
        if let Some(adapter) = adapter {
            normalize_fragments.push(quote! {
                <#adapter as crate::codec::ProtoAdapter<#ty>>::normalize_dynamic(message, #tag);
            });
        }
        if adapter.is_none() {
            asserts.extend(field_asserts_for(
                ty,
                variant.span,
                &variant.proto_path,
                checks,
                ident,
            ));
        }

        // Oneof presence is significant: the member is always emitted.
        let encode_payload = match adapter {
            Some(adapter) => quote! {
                <#adapter as crate::codec::ProtoAdapter<#ty>>::encode_field(#tag, #payload, buf);
            },
            None => quote! {
                <#ty as crate::codec::ProtoField>::encode_field(#tag, #payload, buf);
            },
        };
        let len_payload = match adapter {
            Some(adapter) => quote! {
                len += <#adapter as crate::codec::ProtoAdapter<#ty>>::encoded_len_field(
                    #tag, #payload,
                );
            },
            None => quote! {
                len += <#ty as crate::codec::ProtoField>::encoded_len_field(#tag, #payload);
            },
        };
        let mut entries = sibling_entries.clone();
        entries.push((tag, encode_payload, len_payload));
        entries.sort_by_key(|(tag, _, _)| *tag);
        let encodes = entries.iter().map(|(_, encode, _)| encode);
        let lens = entries.iter().map(|(_, _, len)| len);

        encode_arms.push(quote! {
            Self::#var { #payload, #(#sib_idents),* } => {
                #(#encodes)*
            }
        });
        len_arms.push(quote! {
            Self::#var { #payload, #(#sib_idents),* } => {
                let mut len = 0;
                #(#lens)*
                len
            }
        });

        let merge_payload = match adapter {
            Some(adapter) => quote! {
                <#adapter as crate::codec::ProtoAdapter<#ty>>::merge_field(
                    wire_type, &mut payload, buf, ctx,
                )?;
            },
            None => quote! {
                <#ty as crate::codec::ProtoField>::merge_field(
                    wire_type, &mut payload, buf, ctx,
                )?;
            },
        };
        let take_pats = pats(&sib_idents);
        merge_arms.push(quote! {
            #tag => {
                // Switch variants, carrying the siblings over.
                #[allow(unused_parens)]
                let (#(#sib_idents),*) = match self {
                    #(#take_pats)|* => (#(::std::mem::take(#sib_idents)),*),
                };
                let mut payload = if let Self::#var { #payload, .. } = self {
                    ::std::mem::take(#payload)
                } else {
                    ::core::default::Default::default()
                };
                #merge_payload
                *self = Self::#var { #payload: payload, #(#sib_idents),* };
                ::core::result::Result::Ok(())
            }
        });
    }

    for sibling in &plan.siblings {
        let sid = &sibling.ident;
        let sty = &sibling.ty;
        let stag = sibling.tag;
        let self_pats = pats(&[sid]);
        merge_arms.push(quote! {
            #stag => {
                match self {
                    #(#self_pats)|* => {
                        <#sty as crate::codec::ProtoField>::merge_field(wire_type, #sid, buf, ctx)
                    }
                }
            }
        });
    }

    let default_encode_arm = plan.default_variant.as_ref().map(|var| {
        let encodes = sibling_entries.iter().map(|(_, encode, _)| encode);
        quote! {
            Self::#var { #(#sib_idents),* } => {
                #(#encodes)*
            }
        }
    });
    let default_len_arm = plan.default_variant.as_ref().map(|var| {
        let lens = sibling_entries.iter().map(|(_, _, len)| len);
        quote! {
            Self::#var { #(#sib_idents),* } => {
                let mut len = 0;
                #(#lens)*
                len
            }
        }
    });

    let registrations = registrations(ident, std::slice::from_ref(&plan.proto_name), None);
    let generics = syn::Generics::default();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let normalize = normalize_impl(
        &impl_generics,
        ident,
        &ty_generics,
        where_clause,
        &normalize_fragments,
    );
    let proto_field = message_proto_field(
        &impl_generics,
        ident,
        &ty_generics,
        where_clause,
        std::slice::from_ref(proto_name),
    );
    let tripwire = tripwire(&fingerprint);
    quote! {
        const _: () = {
            #tripwire
            #asserts
        };

        #registrations

        #normalize

        impl ::prost::Message for #ident {
            fn encode_raw(&self, buf: &mut impl ::prost::bytes::BufMut) {
                match self {
                    #(#encode_arms)*
                    #default_encode_arm
                }
            }

            fn merge_field(
                &mut self,
                tag: u32,
                wire_type: ::prost::encoding::WireType,
                buf: &mut impl ::prost::bytes::Buf,
                ctx: ::prost::encoding::DecodeContext,
            ) -> ::core::result::Result<(), ::prost::DecodeError> {
                match tag {
                    #(#merge_arms)*
                    _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }

            fn encoded_len(&self) -> usize {
                match self {
                    #(#len_arms)*
                    #default_len_arm
                }
            }

            fn clear(&mut self) {
                *self = ::core::default::Default::default();
            }
        }

        #proto_field
    }
}
