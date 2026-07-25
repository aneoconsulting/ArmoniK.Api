//! Token emission from resolved plans.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

use crate::kind::{Cardinality, FieldKind};
use crate::resolve::{FieldAccess, FieldCodec, FieldPlan, MessagePlan, StructStyle};

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
    let ty = &plan.ty;
    let span = plan.span;
    let proto_path = &plan.proto_path;
    let mut asserts = TokenStream::new();

    if let Some(kind) = &plan.checks.kind {
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

    if !plan.checks.cardinalities.is_empty() {
        let patterns = plan
            .checks
            .cardinalities
            .iter()
            .map(cardinality_pattern)
            .collect::<Vec<_>>();
        let expected = plan
            .checks
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

    for name in &plan.checks.names {
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

    if let Some((key, value)) = &plan.checks.map_kinds {
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
                if is_singular_message {
                    merge_arms.push(quote! {
                        #tag => {
                            let seed = <Self as ::core::default::Default>::default().#access;
                            if seed != ::core::default::Default::default() && self.#access == seed {
                                self.#access = ::core::default::Default::default();
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
    quote_spanned! {ident.span()=>
        const _: () = {
            assert!(
                crate::__schema::DESCRIPTOR_FINGERPRINT == #fingerprint,
                #tripwire_message
            );
            #asserts
        };

        impl ::prost::Message for #ident {
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

        impl crate::codec::ProtoField for #ident {
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
