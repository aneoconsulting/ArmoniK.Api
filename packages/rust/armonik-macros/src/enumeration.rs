//! `#[armonik_macros::enumeration]`: a proto enum, plain or flattened through a chain of
//! single-field wrapper messages.

use proc_macro2::{Span, TokenStream};
use quote::quote;

use crate::attrs::{self, scan_attrs, unraw, Allowed, AttrItem, FieldAttrs};
use crate::descriptor::{DescriptorIndex, FieldKind, MessageMeta};
use crate::emit::{message_shaped, placeholder_bodies, tripwire, MessageBodies};
use crate::generator::Generator;
use crate::matcher::{not_found, unknown_name};
use crate::plan::{anchored, respan, CatchAll, EnumMode, EnumPlan, EnumValue};

/// Resolve `#[armonik_macros::enumeration]`. A proto enum and a transparent wrapper chain around
/// one are two modes of a single plan rather than two shapes; the wire emitter reads the mode back
/// off the plan. Total: whatever failed is recorded and the plan degrades (poisoned values, a
/// missing catch-all, an unresolved wrapper path), so emission always has something to say.
pub(crate) fn resolve_enumeration(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
    proto_names: &[(Span, String)],
    generator: &mut Generator,
) -> EnumPlan {
    let derived_comparisons = reject_implemented_derives(input, generator);

    let entries = match attrs::parse(&input.attrs) {
        Ok(entries) => entries,
        Err(error) => {
            generator.record(error);
            Vec::new()
        }
    };

    let mut transparent = false;
    for entry in &entries {
        match &entry.item {
            AttrItem::Transparent => transparent = true,
            _ => generator.error(
                entry.span,
                "this armonik attribute is not valid at type level on derive(Enum)",
            ),
        }
    }

    // Resolve the proto enum(s) the variants are matched against, and the wrapper tag in
    // transparent mode. A name that does not resolve is recorded and skipped: the variants below
    // then degrade to poisoned values rather than to a second error each.
    let mut proto_enums: Vec<(String, &crate::descriptor::EnumMeta)> = Vec::new();
    // Intermediate wrapper messages walked through in transparent mode: they have no Rust type, so
    // they are registered as absorbed.
    let mut absorbs: Vec<crate::plan::Absorbed> = Vec::new();
    let mode = if transparent {
        // What the argument names follows the mode: here the wrapper messages the enum is
        // flattened out of, and in the other arm the proto enum itself.
        if proto_names.is_empty() {
            generator.error(
                input.ident.span(),
                "#[armonik(transparent)] requires the single-field wrapper message as the \
                 macro's argument: #[armonik_macros::enumeration(\"full.proto.Name\")]",
            );
        }
        let mut wrapper_path: Option<Vec<u32>> = None;
        for (span, name) in proto_names {
            // Follow the chain of single-field wrappers down to the enum.
            let mut current = name.clone();
            let mut path = Vec::new();
            let enum_name = loop {
                let Some(meta) = index.messages.get(&current) else {
                    generator.record(not_found(*span, "message", &current));
                    break None;
                };
                let Some(field) = MessageMeta::sole_field(Some(meta), &current, *span, generator)
                else {
                    break None;
                };
                path.push(field.tag);
                match &field.kind {
                    FieldKind::Enum(inner) => break Some(inner.clone()),
                    FieldKind::Message(inner) => {
                        // A wrapper layer between the root message and the enum: no Rust type
                        // stands for it.
                        absorbs.push(crate::plan::Absorbed::always(inner.clone()));
                        current = inner.clone();
                    }
                    other => {
                        generator.error(
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
                    generator.error(
                        *span,
                        "transparent wrapper messages disagree on the wrapper tag path",
                    );
                }
            } else {
                wrapper_path = Some(path);
            }
            match index.enums.get(&enum_name) {
                Some(enum_meta) => proto_enums.push((enum_name.clone(), enum_meta)),
                None => generator.record(not_found(*span, "enum", &enum_name)),
            }
        }
        EnumMode::Transparent {
            names: crate::resolve::claimed(proto_names),
            path: wrapper_path,
        }
    } else {
        if proto_names.is_empty() {
            generator.error(
                input.ident.span(),
                "missing the proto enum this type stands for: \
                 #[armonik_macros::enumeration(\"full.proto.Name\")]",
            );
        }
        for (span, name) in proto_names {
            match index.enums.get(name) {
                Some(meta) => proto_enums.push((name.clone(), meta)),
                None => generator.record(not_found(*span, "enum", name)),
            }
        }
        EnumMode::Plain {
            // The names that resolved, not the claimed ones: `SHAPE` is what other types' field
            // asserts compare against, and a name that resolved nothing has no field expecting it,
            // so the reduced (possibly empty, hence unchecked) list keeps one bad name from
            // echoing at every field typed by this enum.
            names: proto_enums.iter().map(|(name, _)| name.clone()).collect(),
        }
    };

    let docs = match &mode {
        EnumMode::Plain { .. } => proto_enums
            .first()
            .map(|(_, meta)| meta.docs.clone())
            .unwrap_or_default(),
        EnumMode::Transparent { names, .. } => {
            index.message_docs(names.first().map(String::as_str))
        }
    };
    let plan = EnumPlan {
        ident: respan(&input.ident),
        catch_all: None,
        docs,
        named: Vec::new(),
        poisoned: Vec::new(),
        zero_variant: None,
        has_std_default: false,
        derived_comparisons,
        is_enum: true,
        mode,
        fingerprint: index.fingerprint,
        absorbs,
    };

    let syn::Data::Enum(data) = &input.data else {
        generator.error(
            input.ident.span(),
            "#[armonik_macros::enumeration] expects an enum",
        );
        return EnumPlan {
            is_enum: false,
            ..plan
        };
    };
    let mut plan = plan;

    // Collect variants: unit variants matched by name, plus exactly one catch-all tuple variant
    // whose payload struct the derive emits. A variant that fails here is kept as poisoned: it
    // still exists on the Rust side, so the matches over the enum keep an arm for it, and the
    // payload struct a stray tuple variant names is still emitted so the item resolves.
    let mut named: Vec<(syn::Ident, String)> = Vec::new();
    for variant in &data.variants {
        plan.has_std_default |= variant
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("default"));
        let scanned = scan_attrs(
            &variant.attrs,
            Allowed {
                rename: true,
                ..Allowed::default()
            },
            "this armonik attribute is not valid on a derive(Enum) variant",
            generator,
        );
        let Some(FieldAttrs { rename, .. }) = scanned else {
            plan.poison(&variant.ident, None);
            continue;
        };

        match &variant.fields {
            syn::Fields::Unit => {
                let proto_name = rename.unwrap_or_else(|| unraw(&variant.ident));
                named.push((variant.ident.clone(), proto_name));
            }
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let payload = match &fields.unnamed[0].ty {
                    syn::Type::Path(path) if path.qself.is_none() => path
                        .path
                        .get_ident()
                        .map(|ident| anchored(ident, fields.paren_token.span.open())),
                    _ => None,
                };
                let Some(payload) = payload else {
                    generator.error(
                        variant.ident.span(),
                        "the catch-all payload must be a bare type name; the derive emits \
                         that struct",
                    );
                    plan.poison(&variant.ident, None);
                    continue;
                };
                if plan.catch_all.is_some() {
                    generator.error(variant.ident.span(), "#[armonik_macros::enumeration] expects exactly one catch-all tuple variant");
                    plan.poison(&variant.ident, Some(payload));
                } else {
                    plan.catch_all = Some(CatchAll {
                        variant: respan(&variant.ident),
                        payload,
                    });
                }
            }
            _ => {
                generator.error(
                    variant.ident.span(),
                    "#[armonik_macros::enumeration] variants must be unit variants or the single \
                     catch-all tuple variant",
                );
                plan.poison(&variant.ident, None);
            }
        }
    }
    if plan.catch_all.is_none() {
        generator.error(
            input.ident.span(),
            "#[armonik_macros::enumeration] requires a catch-all tuple variant, \
             e.g. `Unknown(UnknownTaskStatus)`",
        );
    }

    // Match every named variant against every proto enum; they must agree. What each match
    // consumes is recorded per enum, so the completeness pass below reads what this loop decided
    // instead of re-implementing the matching rule. The zero value starts out consumed: the
    // catch-all covers it losslessly, and the emitted `UNSPECIFIED` const names it.
    let mut consumed: Vec<Vec<bool>> = proto_enums
        .iter()
        .map(|(_, meta)| meta.values.iter().map(|(_, value)| *value == 0).collect())
        .collect();
    for (ident, proto_name) in &named {
        let mut number: Option<i32> = None;
        let mut docs: Vec<String> = Vec::new();
        for (position, (enum_name, meta)) in proto_enums.iter().enumerate() {
            let simple = enum_name.rsplit('.').next().unwrap_or(enum_name);
            let matched = meta.values.iter().position(|(value_name, _)| {
                value_name == proto_name
                    || crate::names::variant_name(simple, value_name) == *proto_name
            });
            match matched.map(|at| (at, &meta.values[at].1)) {
                Some((at, value)) => {
                    consumed[position][at] = true;
                    // Unified enums agree on their values, so the first one documents them.
                    if docs.is_empty() {
                        docs = meta.value_docs[at].clone();
                    }
                    if *number.get_or_insert(*value) != *value {
                        generator.error(
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
                    generator.record(unknown_name(
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
        match number {
            Some(number) => {
                if number == 0 {
                    plan.zero_variant = Some(respan(ident));
                }
                plan.named.push(EnumValue {
                    ident: respan(ident),
                    number,
                    docs,
                });
            }
            // No number: the value did not match (its error is recorded above), or there was no
            // proto enum to match against (the name-level error already covers every variant).
            None => plan.poison(ident, None),
        }
    }

    // Completeness: every proto value the matching loop left unconsumed, checked only when every
    // variant resolved: an unconsumed value otherwise already has its probable explanation on
    // screen, and one mistake reads as one error.
    if plan.poisoned.is_empty() {
        for (position, (enum_name, meta)) in proto_enums.iter().enumerate() {
            for (at, (value_name, value)) in meta.values.iter().enumerate() {
                if !consumed[position][at] {
                    generator.error(
                        input.ident.span(),
                        format!(
                            "proto enum value `{enum_name}.{value_name}` (= {value}) is not \
                             covered by any Rust variant"
                        ),
                    );
                }
            }
        }
    }

    plan
}

/// The wire half. A plain proto enum is an `int32` varint, so it implements `ProtoField` directly;
/// a transparent wrapper chain is message-shaped and goes through the same bundle as every struct.
/// These are the crate's two families, and the enumeration is the one type that can be either; the
/// choice is read off [`EnumMode`] here because no other macro has one.
pub(crate) fn wire(plan: &EnumPlan, generator: &Generator) -> TokenStream {
    match &plan.mode {
        EnumMode::Plain { names } => plain_wire(plan, names),
        EnumMode::Transparent { names, path } => {
            transparent_wire(plan, names, path.as_deref(), generator)
        }
    }
}

/// `path` is `None` when the chain did not resolve, which is the degenerate case of the same
/// expansion: real signatures over placeholder bodies, and the claimed names still on `Msg` so the
/// rpc asserts stay quiet.
fn transparent_wire(
    plan: &EnumPlan,
    names: &[String],
    path: Option<&[u32]>,
    generator: &Generator,
) -> TokenStream {
    let bodies = match path {
        // The chain as a codec type rather than a runtime walk over the tags: the enum at the
        // bottom, one `Wrapper` per level above it, and the outermost tag written by the message
        // itself.
        Some(path) => {
            let (root, nested) = path.split_first().expect("non-empty wrapper path");
            let codec = nested.iter().rev().fold(
                quote!(crate::codec::adapters::EnumLeaf),
                |inner, tag| quote!(crate::codec::adapters::Wrapper<#inner, #tag>),
            );
            chain_bodies(root, &codec)
        }
        None => placeholder_bodies(),
    };
    message_shaped(
        &plan.ident,
        &syn::Generics::default(),
        plan.fingerprint,
        names,
        true,
        generator.poisoned(),
        // The variants were checked against the proto enum by resolution; there is no field here
        // whose Rust type a const-assert could check.
        TokenStream::new(),
        bodies,
    )
}

/// The wire form of a resolved wrapper chain: the outermost tag, and the codec for everything under
/// it.
fn chain_bodies(root: &u32, codec: &TokenStream) -> MessageBodies {
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
    }
}

/// A proto enum on the wire: an `int32` varint, reached through `ProtoField` rather than through the
/// `Msg` blanket, because a proto enum is not a message.
fn plain_wire(plan: &EnumPlan, names: &[String]) -> TokenStream {
    let ident = &plan.ident;
    let tripwire = tripwire(plan.fingerprint);
    // The wire delegates lean on the `From` conversions the value items only emit for an enum;
    // anything else keeps the signatures and degrades every body to a placeholder (which consumes
    // its arguments, so the failing build carries no unused-variable noise on top of its error).
    let body = |silence: TokenStream, real: TokenStream| {
        if plan.is_enum {
            real
        } else {
            quote! { let _ = #silence; ::core::unimplemented!() }
        }
    };
    let encode = body(
        quote!((tag, value, buf)),
        quote! { crate::codec::enumeration::encode(tag, value, buf); },
    );
    let merge = body(
        quote!((wire_type, value, buf, ctx)),
        quote! { crate::codec::enumeration::merge(wire_type, value, buf, ctx) },
    );
    let len = body(
        quote!((tag, value)),
        quote! { crate::codec::enumeration::encoded_len(tag, value) },
    );
    let is_zero = body(quote!(value), quote! { i32::from(*value) == 0 });
    let encode_repeated = body(
        quote!((tag, values, buf)),
        quote! { crate::codec::enumeration::encode_repeated(tag, values, buf); },
    );
    let len_repeated = body(
        quote!((tag, values)),
        quote! { crate::codec::enumeration::encoded_len_repeated(tag, values) },
    );
    let merge_repeated = body(
        quote!((wire_type, values, buf, ctx)),
        quote! { crate::codec::enumeration::merge_repeated(wire_type, values, buf, ctx) },
    );
    quote! {
        const _: () = { #tripwire };

        impl crate::codec::ProtoField for #ident {
            const SHAPE: crate::codec::Shape =
                crate::codec::Shape::enumeration(&[#(#names),*]);

            fn encode_field(tag: u32, value: &Self, buf: &mut impl ::prost::bytes::BufMut) {
                #encode
            }

            fn merge_field(
                wire_type: ::prost::encoding::WireType,
                value: &mut Self,
                buf: &mut impl ::prost::bytes::Buf,
                ctx: ::prost::encoding::DecodeContext,
            ) -> ::core::result::Result<(), ::prost::DecodeError> {
                #merge
            }

            fn encoded_len_field(tag: u32, value: &Self) -> usize {
                #len
            }

            // Whatever the enum calls it, the zero value is the one an implicit-presence field
            // leaves out.
            fn is_zero(value: &Self) -> bool {
                #is_zero
            }

            fn encode_repeated(tag: u32, values: &[Self], buf: &mut impl ::prost::bytes::BufMut) {
                #encode_repeated
            }

            fn encoded_len_repeated(tag: u32, values: &[Self]) -> usize {
                #len_repeated
            }

            fn merge_repeated(
                wire_type: ::prost::encoding::WireType,
                values: &mut ::std::vec::Vec<Self>,
                buf: &mut impl ::prost::bytes::Buf,
                ctx: ::prost::encoding::DecodeContext,
            ) -> ::core::result::Result<(), ::prost::DecodeError> {
                #merge_repeated
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
    // Every value-level item matches over variants; an item that is not an enum has none, so there
    // is nothing to say (and a struct's own derives, `Default` among them, must not be collided
    // with).
    if !plan.is_enum {
        return TokenStream::new();
    }
    let ident = &plan.ident;
    let comparison = (!plan.derived_comparisons).then(|| comparison(plan));
    let serde = serde(plan);

    // A placeholder can stand in for code, but not for a name: every payload struct the item
    // mentions is emitted (the catch-all's, and any a poisoned stray tuple variant names), while
    // the bodies that need what did not resolve degrade to `unimplemented!()`.
    let payload_structs = plan
        .catch_all
        .iter()
        .map(|catch_all| &catch_all.payload)
        .chain(
            plan.poisoned
                .iter()
                .filter_map(|value| value.payload.as_ref()),
        )
        .map(|payload| {
            let payload_doc = format!(
                "Raw value of an `{ident}` not known to this crate version (or the \
                 unspecified zero value). Only constructible from a value this crate \
                 version does not name, so a known value can never hide inside the \
                 catch-all variant.",
            );
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
            }
        });
    let payload_structs = quote!(#(#payload_structs)*);

    let unimplemented = quote!(::core::unimplemented!());
    let from_named_arms = plan.named.iter().map(|value| {
        let (variant, number) = (&value.ident, value.number);
        quote!(#number => Self::#variant)
    });
    let from_i32_body = match &plan.catch_all {
        Some(CatchAll { variant, payload }) => quote! {
            match value {
                #(#from_named_arms,)*
                value => Self::#variant(#payload(value)),
            }
        },
        None => quote!(let _ = value; #unimplemented),
    };
    let into_named_arms = plan.named.iter().map(|value| {
        let (variant, number) = (&value.ident, value.number);
        quote!(#ident::#variant => #number)
    });
    // Poisoned variants have no number: their arms keep the match exhaustive over what the user
    // wrote, and panic in a build the recorded error already fails.
    let into_poisoned_arms = plan.poisoned.iter().map(|value| {
        let variant = &value.ident;
        quote!(#ident::#variant { .. } => #unimplemented)
    });
    let into_catch_all_arm = plan.catch_all.as_ref().map(|catch_all| {
        let variant = &catch_all.variant;
        quote!(#ident::#variant(raw) => raw.0,)
    });

    let default_impl = (!plan.has_std_default).then(|| {
        let default_expr = match (&plan.zero_variant, &plan.catch_all) {
            (Some(variant), _) => quote!(Self::#variant),
            (None, Some(_)) => quote!(Self::UNSPECIFIED),
            (None, None) => unimplemented.clone(),
        };
        quote! {
            impl ::core::default::Default for #ident {
                fn default() -> Self {
                    #default_expr
                }
            }
        }
    });
    // A const cannot hold a placeholder, so without a catch-all there is no `UNSPECIFIED`; the
    // recorded error is what the build fails with.
    let unspecified_const = match (&plan.zero_variant, &plan.catch_all) {
        (None, Some(CatchAll { variant, payload })) => Some(quote! {
            impl #ident {
                /// The unspecified (zero) value. Compare with `==` rather than matching on it:
                /// the comparison traits are implemented in terms of the proto value, which
                /// makes the type non-structural-match.
                pub const UNSPECIFIED: Self = Self::#variant(#payload(0));
            }
        }),
        _ => None,
    };

    quote! {
        #payload_structs

        impl ::core::convert::From<i32> for #ident {
            /// Normalizing: known values always map to their named variants.
            fn from(value: i32) -> Self {
                #from_i32_body
            }
        }

        impl ::core::convert::From<#ident> for i32 {
            fn from(value: #ident) -> Self {
                match value {
                    #(#into_named_arms,)*
                    #(#into_poisoned_arms,)*
                    #into_catch_all_arm
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
    let values = plan.named.iter().map(|value| {
        let name = unraw(&value.ident);
        let number = value.number;
        quote!((#name, #number))
    });
    let name = unraw(ident);
    // Deserializing needs the catch-all, both to name and to build; the payload impls belong to
    // the struct it declares. Without one, `Serialize` still works through `i32::from`, and
    // `Deserialize` degrades to a placeholder body next to the recorded error.
    let (deserialize_body, payload_impls) = match &plan.catch_all {
        Some(CatchAll { variant, payload }) => {
            let unknown = unraw(variant);
            (
                quote! {
                    crate::codec::enum_serde::deserialize(VALUES, #name, #unknown, deserializer)
                        .map(Self::from)
                },
                Some(quote! {
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
                            // `From<i32>` already knows which numbers are named; a payload is what
                            // is left.
                            match #ident::from(value) {
                                #ident::#variant(raw) => ::core::result::Result::Ok(raw),
                                _ => ::core::result::Result::Err(
                                    <D::Error as ::serde::de::Error>::custom(::std::format!(
                                        "`{}` names {value}, so it cannot be an unknown value",
                                        #name,
                                    )),
                                ),
                            }
                        }
                    }
                }),
            )
        }
        None => (quote!(let _ = deserializer; ::core::unimplemented!()), None),
    };
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
                    #deserialize_body
                }
            }

            #payload_impls
        };
    }
}

/// The traits the expansion implements itself, which a site must therefore not derive.
const IMPLEMENTED: [&str; 5] = ["PartialEq", "Eq", "PartialOrd", "Ord", "Hash"];

/// Reject `#[derive(PartialEq)]` and friends, rather than leaving rustc to report `E0119` at the
/// attribute with no hint about which of the two impls it should keep. Returns whether any was
/// found: the emitted comparison impls are then withheld wholesale, since the derived ones satisfy
/// the bounds and two sets would be the very `E0119` this error preempts.
fn reject_implemented_derives(input: &syn::DeriveInput, generator: &mut Generator) -> bool {
    use syn::spanned::Spanned as _;

    let mut found = false;
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
            found = true;
            generator.error(
                path.span(),
                format!(
                    "an enumeration must not derive `{name}`: the expansion implements it in \
                     terms of the proto value, so that the two spellings of one value (the named \
                     variant, and the catch-all holding its number) are one value"
                ),
            );
        }
    }
    found
}
