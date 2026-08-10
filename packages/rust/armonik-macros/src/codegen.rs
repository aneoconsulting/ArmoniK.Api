//! Token emission from resolved plans.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

use crate::descriptor::{Cardinality, FieldKind};
use crate::resolve::{EnumMode, EnumPlan, Expectation, FieldAccess, FieldCodec, MessagePlan};

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

/// Qualified dispatch prefix for a field's codec: `ProtoField` and `ProtoAdapter` share their
/// method names, so every fragment is written once and prefixed with whichever the field encodes
/// through.
fn dispatch(ty: &syn::Type, adapter: Option<&syn::Type>) -> TokenStream {
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
fn field_fragments(
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
fn normalize_impl(
    impl_generics: &syn::ImplGenerics,
    ident: &syn::Ident,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    fragments: &[TokenStream],
) -> TokenStream {
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
    let fingerprint = proc_macro2::Literal::u64_suffixed(plan.fingerprint);

    let mut generics = plan.generics.clone();
    for param in generics.type_params_mut() {
        param
            .bounds
            .push(syn::parse_quote!(crate::codec::ProtoField));
        // `Send`/`Sync` because `prost::Message` requires them. Nothing else: `PartialEq` and
        // `Debug` were here for the deleted `is_default` family and no emitted code needs them.
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
    let mut normalize_fragments = Vec::new();
    let mut asserts = TokenStream::new();

    for field in &plan.fields {
        let access = &field.access;
        let ty = &field.ty;
        let tag = field.tag;

        match &field.codec {
            FieldCodec::Field { adapter } => {
                let d = dispatch(ty, adapter.as_deref());
                let (_, encode, len) = field_fragments(&d, tag, quote!(&self.#access));
                encode_fragments.push(encode);
                len_fragments.push(quote! { len += #len; });
                merge_arms.push(quote! {
                    #tag => #d::merge_field(wire_type, &mut self.#access, buf, ctx)
                });
                if adapter.is_some() {
                    normalize_fragments.push(quote! {
                        #d::normalize_dynamic(message, #tag);
                    });
                } else {
                    asserts.extend(field_asserts_for(
                        ty,
                        field.span,
                        &field.proto_path,
                        &field.checks,
                        ident,
                    ));
                }
            }
            FieldCodec::OneofGroup { tags } => {
                encode_fragments.push(quote! {
                    <#ty as ::prost::Message>::encode_raw(&self.#access, buf);
                });
                merge_arms.push(quote! {
                    #(#tags)|* => <#ty as ::prost::Message>::merge_field(
                        &mut self.#access, tag, wire_type, buf, ctx,
                    )
                });
                len_fragments.push(quote! {
                    len += <#ty as ::prost::Message>::encoded_len(&self.#access);
                });
                normalize_fragments.push(quote! {
                    <#ty as crate::differential::Normalize>::normalize(message);
                });
            }
        }
    }

    let registrations = registrations(ident, proto_names);
    let normalize = normalize_impl(
        &impl_generics,
        ident,
        &ty_generics,
        where_clause,
        &normalize_fragments,
    );
    let message = message_impl(
        &impl_generics,
        ident,
        &ty_generics,
        where_clause,
        quote! { #(#encode_fragments)* },
        quote! {
            match tag {
                #(#merge_arms,)*
                _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        },
        quote! {
            #[allow(unused_mut)]
            let mut len = 0;
            #(#len_fragments)*
            len
        },
    );
    let proto_field = msg_impl(
        &impl_generics,
        ident,
        &ty_generics,
        where_clause,
        proto_names,
    );
    let tripwire = tripwire(&fingerprint);
    quote! {
        const _: () = {
            #tripwire
            #asserts
        };

        #registrations

        #normalize

        #message

        #proto_field
    }
}

/// The `prost::Message` impl skeleton shared by every emission site (plain struct, transparent
/// struct, transparent enum, whole-message oneof). `clear` is always a whole-value reset: every
/// derived type is `Default`, and the zero-default invariant makes that the proto zero.
fn message_impl(
    impl_generics: &syn::ImplGenerics,
    ident: &syn::Ident,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    encode_raw: TokenStream,
    merge_field: TokenStream,
    encoded_len: TokenStream,
) -> TokenStream {
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
                *self = ::core::default::Default::default();
            }
        }
    }
}

/// The one-line `Msg` implementation for a message-shaped type: the blanket `ProtoField` impl in
/// `codec` picks it up, so the type composes as a field of other derived messages.
fn msg_impl(
    impl_generics: &syn::ImplGenerics,
    ident: &syn::Ident,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    proto_names: &[String],
) -> TokenStream {
    quote! {
        impl #impl_generics crate::codec::Msg for #ident #ty_generics #where_clause {
            const NAMES: &'static [&'static str] = &[#(#proto_names),*];
        }
    }
}

/// Codegen for a `#[armonik(transparent)]` struct: a single-field newtype whose `prost::Message`
/// impl delegates entirely to the field, so it is wire-identical to the inner message and can stand
/// for a whole RPC message. The `Normalize` projection delegates likewise.
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

    let registrations = registrations(ident, proto_names);
    let normalize = normalize_impl(
        impl_generics,
        ident,
        ty_generics,
        where_clause,
        &[quote! { <#ty as crate::differential::Normalize>::normalize(message); }],
    );
    let message = message_impl(
        impl_generics,
        ident,
        ty_generics,
        where_clause,
        quote! { <#ty as ::prost::Message>::encode_raw(&self.#access, buf); },
        quote! { <#ty as ::prost::Message>::merge_field(&mut self.#access, tag, wire_type, buf, ctx) },
        quote! { <#ty as ::prost::Message>::encoded_len(&self.#access) },
    );
    let proto_field = msg_impl(impl_generics, ident, ty_generics, where_clause, proto_names);
    let tripwire = tripwire(&fingerprint);
    quote! {
        const _: () = { #tripwire };

        #registrations

        #normalize

        #message

        #proto_field
    }
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

/// Emission for oneof-shaped enums: one `prost::Message` impl either way, plus registration and the
/// `Msg` marker when the enum stands for a whole message. With sibling fields (non-oneof fields of
/// a whole-message enum), every variant carries all of them, the "no member set" default included,
/// which keeps the per-field merge stateless and order-independent: a sibling occurrence merges
/// into the current variant's slot, a member occurrence switches variants while carrying the
/// siblings over. A sibling-free enum is the degenerate case with an empty sibling list.
pub(crate) fn oneof(plan: &crate::resolve::OneofPlan) -> TokenStream {
    use crate::resolve::OneofVariantShape;

    let ident = &plan.ident;
    let proto_name = &plan.proto_name;
    let fingerprint = proc_macro2::Literal::u64_suffixed(plan.fingerprint);

    // Sibling machinery (empty and inert without siblings): all-variant patterns binding a subset
    // of the siblings, plus the sibling fields' fragments. Every variant carries every sibling, so
    // the fragments are emitted once *around* the member match rather than inside each of its arms,
    // and the arms only have to deal with the member.
    let sib_idents: Vec<&syn::Ident> = plan.siblings.iter().map(|sibling| &sibling.ident).collect();
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
    // Binds every sibling by reference, whatever the variant.
    let bind_siblings = (!sib_idents.is_empty()).then(|| {
        let all = pats(&sib_idents);
        quote! {
            #[allow(unused_parens)]
            let (#(#sib_idents),*) = match value {
                #(#all)|* => (#(#sib_idents),*),
            };
        }
    });
    // Ascending tags across the whole message: the siblings below the oneof's tags are written
    // before the member, the ones above it after. (The shapes the derive accepts never interleave
    // the two.)
    let min_member_tag = plan.variants.iter().map(|variant| variant.tag).min();
    let (low, high): (Vec<_>, Vec<_>) = plan
        .siblings
        .iter()
        .map(|sibling| {
            let sid = &sibling.ident;
            let d = dispatch(&sibling.ty, None);
            field_fragments(&d, sibling.tag, quote!(#sid))
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
        asserts.extend(field_asserts_for(
            &sibling.ty,
            sibling.span,
            &sibling.proto_path,
            &sibling.checks,
            ident,
        ));
    }

    for variant in &plan.variants {
        let var = &variant.ident;
        let tag = variant.tag;
        match &variant.shape {
            OneofVariantShape::Payload {
                ty,
                adapter,
                checks,
                binding,
            } => {
                let d = dispatch(ty, adapter.as_deref());
                if adapter.is_some() {
                    normalize_fragments.push(quote! {
                        #d::normalize_dynamic(message, #tag);
                    });
                } else {
                    asserts.extend(field_asserts_for(
                        ty,
                        variant.span,
                        &variant.proto_path,
                        checks,
                        ident,
                    ));
                }

                // The active member carries the oneof's presence, so it is emitted even with a
                // default payload, like every other field.
                let (_, encode, len) = field_fragments(&d, tag, quote!(payload));

                // Matching binds the member as `payload` and ignores the siblings; constructing one
                // needs them, so merging a member takes them along.
                let pattern = match binding {
                    None => quote!(Self::#var(payload)),
                    Some(field) => quote!(Self::#var { #field: payload, .. }),
                };
                let (construct, take) = match binding {
                    None => (quote!(Self::#var(payload)), None),
                    Some(field) => (
                        quote!(Self::#var { #field: payload, #(#sib_idents),* }),
                        Some({
                            let take_pats = pats(&sib_idents);
                            quote! {
                                #[allow(unused_parens)]
                                let (#(#sib_idents),*) = match value {
                                    #(#take_pats)|* => (#(::std::mem::take(#sib_idents)),*),
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
                        #d::merge_field(wire_type, &mut payload, buf, ctx)?;
                        *value = #construct;
                        ::core::result::Result::Ok(())
                    }
                });
            }
            OneofVariantShape::MarkerBool => {
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
            OneofVariantShape::MarkerMessage => {
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
                // The variant's parts are ordinary fields of the inline message; only its framing
                // is hand-rolled, since the message is absorbed and has no Rust type to delegate
                // to.
                let fragments: Vec<_> = parts
                    .iter()
                    .map(|part| {
                        let id = &part.ident;
                        field_fragments(&dispatch(&part.ty, None), part.tag, quote!(#id))
                    })
                    .collect();
                let encodes = fragments.iter().map(|(_, encode, _)| encode);
                let lens = fragments.iter().map(|(_, _, len)| len);
                let body_len = quote! {
                    let body_len = 0 #(+ #lens)*;
                };

                encode_arms.push(quote! {
                    Self::#var { #(#part_idents),* } => {
                        #body_len
                        ::prost::encoding::encode_key(
                            #tag,
                            ::prost::encoding::WireType::LengthDelimited,
                            buf,
                        );
                        ::prost::encoding::encode_varint(body_len as u64, buf);
                        #(#encodes)*
                    }
                });
                len_arms.push(quote! {
                    Self::#var { #(#part_idents),* } => {
                        #body_len
                        ::prost::encoding::key_len(#tag)
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
                        // Through prost's own framing, which brings the recursion and length
                        // limits `ctx` carries and rejects a body that runs past its declared end.
                        #[allow(unused_parens)]
                        let mut parts = (#(#part_idents),*);
                        ::prost::encoding::merge_loop(
                            &mut parts,
                            buf,
                            ctx,
                            |parts, buf, ctx| {
                                let (tag, wire_type) = ::prost::encoding::decode_key(buf)?;
                                #[allow(unused_parens)]
                                let (#(#part_idents),*) = parts;
                                match tag {
                                    #(
                                        #part_tags => <#part_tys as crate::codec::ProtoField>::merge_field(
                                            wire_type, #part_idents, buf, ctx,
                                        ),
                                    )*
                                    _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
                                }
                            },
                        )?;
                        #[allow(unused_parens)]
                        let (#(#part_idents),*) = parts;
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

    // A sibling occurrence merges in place, whatever the current variant.
    for sibling in &plan.siblings {
        let sid = &sibling.ident;
        let sty = &sibling.ty;
        let stag = sibling.tag;
        let self_pats = pats(&[sid]);
        merge_arms.push(quote! {
            #stag => {
                match value {
                    #(#self_pats)|* => {
                        <#sty as crate::codec::ProtoField>::merge_field(wire_type, #sid, buf, ctx)
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
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // A whole-message enum is additionally the message itself: it registers and gets the `Msg`
    // marker, which is what makes it usable as an RPC message and as a field of another message.
    // The `prost::Message` impl below is shared with the embedded case; nothing is layered on top
    // of it, because there is nothing left to add. The old forwarding layer wrapped the same match
    // in a second one whose default arm was `skip_field`, which the inner match already ends with.
    let whole_message = plan.whole_message.then(|| {
        let registrations = registrations(ident, std::slice::from_ref(&plan.proto_name));
        let msg = msg_impl(
            &impl_generics,
            ident,
            &ty_generics,
            where_clause,
            std::slice::from_ref(proto_name),
        );
        quote! {
            #registrations

            #msg
        }
    });

    // Emitted for embedded oneofs too: the containing message's `Normalize` delegates to it (the
    // members live on the parent's dynamic message).
    let normalize = normalize_impl(
        &impl_generics,
        ident,
        &ty_generics,
        where_clause,
        &normalize_fragments,
    );

    // `let value = self;` is the whole cost of the change: the emitted bodies are written against a
    // `value` binding, and `prost::Message` takes a receiver where the deleted `ProtoOneof` took an
    // argument.
    let message = message_impl(
        &impl_generics,
        ident,
        &ty_generics,
        where_clause,
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
