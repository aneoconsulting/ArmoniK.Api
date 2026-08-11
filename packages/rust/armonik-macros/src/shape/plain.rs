//! A plain struct: every field is a field of one proto message.
//!
//! Resolution above the divider, emission below it; the same split holds in every `shape` module.

use proc_macro2::{Span, TokenStream};
use quote::quote;

use crate::attr_site::{field_access, scan_attrs, unraw, Allowed, FieldAttrs};
use crate::attrs::{self, AttrItem, Errors};
use crate::descriptor::DescriptorIndex;
use crate::emit::{
    dispatch, field_asserts_for, field_fragments, message_impl, msg_impl, normalize_impl,
    registrations, tripwire,
};
use crate::matcher::{not_found, Found, Matcher};
use crate::plan::{Expectation, FieldCodec, FieldPlan, MessagePlan};
use crate::shape::generic::generic_plan;
use crate::shape::transparent::{transparent_message, transparent_plan};

pub(crate) fn message_plan(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
) -> Result<MessagePlan, Errors> {
    let mut errors = Errors::new();

    let entries = match attrs::parse(&input.attrs) {
        Ok(entries) => entries,
        Err(err) => return Err(Errors::from(err)),
    };

    let mut proto_names: Vec<(Span, String)> = Vec::new();
    let mut generic = false;
    let mut transparent = false;
    for entry in &entries {
        match &entry.item {
            AttrItem::Message(lit) => proto_names.push((entry.span, lit.value())),
            AttrItem::Generic => generic = true,
            AttrItem::Transparent => transparent = true,
            AttrItem::Oneof(_) => {
                errors.at(entry.span, "this armonik attribute mode is not valid here");
            }
            _ => errors.push(syn::Error::new(
                entry.span,
                "this armonik attribute is not valid at type level on a struct",
            )),
        }
    }
    if generic {
        if !proto_names.is_empty() {
            errors.at(
                input.ident.span(),
                "#[armonik(generic)] types are not validated against the descriptor; \
                 remove the message attribute",
            );
            return Err(errors);
        }
        return generic_plan(input, index, errors);
    }
    if transparent {
        return transparent_plan(input, index, proto_names, errors);
    }
    if proto_names.is_empty() {
        errors.at(
            input.ident.span(),
            "missing #[armonik(message = \"full.proto.Name\")] \
             (or #[armonik(generic)] with explicit tags)",
        );
        return Err(errors);
    }
    if !input.generics.params.is_empty() {
        errors.at(
            input.ident.span(),
            "descriptor-validated types cannot be generic; use #[armonik(generic)]",
        );
        return Err(errors);
    }

    // One proto message per struct. `message = ...` is repeatable on an *enum*, where a unified
    // type stands for several identical protos; the struct side of that never had a user, and the
    // machinery for it (resolve every field against every message, then check the messages agree on
    // its tag, kind and cardinality) was 55 lines nothing exercised.
    for (span, _) in proto_names.iter().skip(1) {
        errors.at(
            *span,
            "a struct stands for one proto message; declare one #[armonik(message = ...)]",
        );
    }
    let (name, meta) = {
        let (span, name) = &proto_names[0];
        match index.messages.get(name) {
            Some(meta) => (name.as_str(), meta),
            None => {
                errors.push(not_found(*span, "message", name));
                return Err(errors);
            }
        }
    };

    let syn::Data::Struct(data) = &input.data else {
        errors.at(
            input.ident.span(),
            "#[armonik_macros::message] with `message = ...` expects a struct \
             (use `oneof = ...` for flattened oneofs)",
        );
        return Err(errors);
    };

    let mut fields = Vec::new();
    let mut matcher = Matcher::new(name, meta);

    for (field_index, field) in data.fields.iter().enumerate() {
        let (span, access) = field_access(field, field_index);
        // No `tag`: a descriptor-validated field takes its tag from the descriptor, and every one
        // of the six `tag = ...` sites in the crate is inside an `#[armonik(generic)]` struct,
        // which `generic_plan` handles. Spelling one here only ever restated what the proto says.
        let Some((FieldAttrs { rename, with, .. }, _)) = scan_attrs(
            &field.attrs,
            Allowed {
                rename: true,
                with: true,
                absorbs: true,
                ..Allowed::default()
            },
            "this armonik attribute is not valid on a message field",
            &mut errors,
        ) else {
            continue;
        };
        let with = with.map(|(_, ty)| ty);

        let proto_name = match (&rename, &field.ident) {
            (Some(name), _) => name.clone(),
            (None, Some(ident)) => unraw(ident),
            (None, None) => {
                errors.at(
                    span,
                    "tuple struct fields need #[armonik(rename = \"proto_field_name\")]",
                );
                continue;
            }
        };

        let Some(resolved) = matcher.find(&proto_name, span, &mut errors) else {
            continue;
        };

        let proto_path = format!("{name}.{proto_name}");

        let (tag, checks) = match resolved {
            Found::Oneof { tags } => {
                if with.is_some() {
                    errors.at(
                        span,
                        "with/tag attributes are not supported on oneof fields",
                    );
                    continue;
                }
                let min_tag = tags.iter().copied().min().unwrap_or_default();
                fields.push(FieldPlan {
                    access,
                    ty: field.ty.clone(),
                    span,
                    tag: min_tag,
                    codec: FieldCodec::OneofGroup { tags },
                    checks: None,
                    proto_path,
                });
                continue;
            }
            Found::Field(field_meta) => {
                let checks = match &with {
                    Some(_) => None,
                    None => Expectation::of(field_meta),
                };
                (field_meta.tag, checks)
            }
        };
        fields.push(FieldPlan {
            access,
            ty: field.ty.clone(),
            span,
            tag,
            codec: FieldCodec::Field {
                adapter: with.map(Box::new),
            },
            checks,
            proto_path,
        });
    }

    // Completeness: every proto field and oneof must be covered by a Rust field.
    matcher.check_complete(input.ident.span(), &mut errors);

    errors.into_result()?;

    fields.sort_by_key(|field| field.tag);
    Ok(MessagePlan {
        ident: input.ident.clone(),
        proto_names: proto_names.into_iter().map(|(_, name)| name).collect(),
        fields,
        generics: input.generics.clone(),
        fingerprint: index.fingerprint,
        transparent: false,
    })
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
