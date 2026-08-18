//! A plain struct: every field is a field of one proto message.
//!
//! Resolution above the divider, emission below it; the same split holds in every `shape` module.

use proc_macro2::{Span, TokenStream};
use quote::quote;

use crate::attr_site::{field_access, scan_attrs, unraw, Allowed, FieldAttrs};
use crate::attrs::Errors;
use crate::descriptor::DescriptorIndex;
use crate::emit::{
    bound_generics, message_shaped, slot_asserts, slot_merge_in_place, slot_write, MessageBodies,
    Presence,
};
use crate::matcher::{not_found, Found, Matcher};
use crate::plan::{Expectation, MessagePlan, Slot, SlotCodec};
use crate::shape::transparent::transparent_message;

/// Plan for a plain struct: every field is a field of the one proto message named at type level.
///
/// `proto_names` and `errors` come from [`crate::shape::resolve_message`], which owns the
/// type-level attribute scan and the choice between this shape, `generic` and `transparent`.
pub(crate) fn message_plan(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
    proto_names: Vec<(Span, String)>,
    mut errors: Errors,
) -> Result<MessagePlan, Errors> {
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
    // Messages a `with` adapter flattens away, so no Rust type stands for them.
    let mut absorbs = Vec::new();
    let mut matcher = Matcher::new(name, meta);

    for (field_index, field) in data.fields.iter().enumerate() {
        let (span, access) = field_access(field, field_index);
        // No `tag`: a descriptor-validated field takes its tag from the descriptor, and every one
        // of the six `tag = ...` sites in the crate is inside an `#[armonik(generic)]` struct,
        // which `generic_plan` handles. Spelling one here only ever restated what the proto says.
        let Some((
            FieldAttrs {
                rename,
                with,
                absorbs: declared,
                ..
            },
            _,
        )) = scan_attrs(
            &field.attrs,
            Allowed {
                rename: true,
                with: true,
                absorbs: true,
                ..Allowed::default()
            },
            "this armonik attribute is not valid on a message field",
            &mut errors,
        )
        else {
            continue;
        };
        absorbs.extend(declared);
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

        let (tag, checks, docs) = match resolved {
            Found::Oneof { tags } => {
                if with.is_some() {
                    errors.at(
                        span,
                        "with/tag attributes are not supported on oneof fields",
                    );
                    continue;
                }
                let min_tag = tags.iter().copied().min().unwrap_or_default();
                fields.push(Slot {
                    access: Some(access),
                    span,
                    tag: min_tag,
                    codec: SlotCodec::Oneof {
                        ty: Box::new(field.ty.clone()),
                        tags,
                    },
                    checks: None,
                    proto_path,
                    // A oneof is reached through a Rust field named after the *declaration*, which
                    // carries no comment of its own in the descriptor.
                    docs: Vec::new(),
                });
                continue;
            }
            Found::Field(field_meta) => {
                let checks = match &with {
                    Some(_) => None,
                    None => Expectation::of(field_meta),
                };
                (field_meta.tag, checks, field_meta.docs.clone())
            }
        };
        fields.push(Slot {
            access: Some(access),
            span,
            tag,
            codec: SlotCodec::Field {
                ty: Box::new(field.ty.clone()),
                adapter: with.map(Box::new),
            },
            checks,
            proto_path,
            docs,
        });
    }

    // Completeness: every proto field and oneof must be covered by a Rust field.
    matcher.check_complete(input.ident.span(), &mut errors);

    errors.into_result()?;

    fields.sort_by_key(|field| field.tag);
    Ok(MessagePlan {
        ident: input.ident.clone(),
        proto_names: proto_names.into_iter().map(|(_, name)| name).collect(),
        docs: meta.docs.clone(),
        fields,
        absorbs,
        generics: input.generics.clone(),
        fingerprint: index.fingerprint,
        transparent: false,
    })
}

pub(crate) fn message(plan: &MessagePlan) -> TokenStream {
    let ident = &plan.ident;
    let generics = bound_generics(&plan.generics);

    if plan.transparent {
        return transparent_message(plan, &generics);
    }

    // A generic type carries its fields' tags and instantiated shapes to wherever it is
    // instantiated, because it cannot be checked where it is declared: it names no proto message.
    // Every `#[armonik_macros::alias]` over it then asserts them against the message it registers
    // under. The `SHAPE`s are written against the type parameters, so each instantiation reports
    // its own.
    let generic_fields = plan.proto_names.is_empty().then(|| {
        let params = &plan.generics;
        let (_, ty_generics, _) = params.split_for_impl();
        let entries = plan.fields.iter().map(|field| {
            let tag = field.tag;
            let dispatch = crate::emit::slot_dispatch(field);
            quote! { (#tag, #dispatch::SHAPE) }
        });
        quote! {
            impl #generics crate::codec::GenericFields for #ident #ty_generics {
                const FIELDS: &'static [(u32, crate::codec::Shape)] = &[#(#entries),*];
            }
        }
    });

    let mut encode_fragments = Vec::new();
    let mut merge_arms = Vec::new();
    let mut len_fragments = Vec::new();
    let mut normalize_fragments = Vec::new();
    let mut asserts = TokenStream::new();

    // Every field of a struct is shared: there is one alternative and it owns nothing, so each
    // slot is written from `self` and merges in place. That is the whole of the struct case.
    for field in &plan.fields {
        let access = field.access.as_ref().expect("a struct field is reachable");
        let tag = field.tag;
        asserts.extend(slot_asserts(field, ident));

        let written = slot_write(field, &quote!(&self.#access), Presence::Implicit);
        encode_fragments.push(written.encode);
        let len = written.len;
        len_fragments.push(quote! { len += #len; });
        normalize_fragments.extend(written.normalize);

        let merge = slot_merge_in_place(field, &quote!(&mut self.#access));
        // A whole oneof answers to every one of its members' tags.
        let keys = match &field.codec {
            SlotCodec::Oneof { tags, .. } => quote!(#(#tags)|*),
            _ => quote!(#tag),
        };
        merge_arms.push(quote! { #keys => #merge });
    }

    let expansion = message_shaped(
        ident,
        &generics,
        plan.fingerprint,
        &plan.proto_names,
        true,
        asserts,
        MessageBodies {
            encode_raw: quote! { #(#encode_fragments)* },
            merge_field: quote! {
                match tag {
                    #(#merge_arms,)*
                    _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            },
            encoded_len: quote! {
                #[allow(unused_mut)]
                let mut len = 0;
                #(#len_fragments)*
                len
            },
            normalize: normalize_fragments,
        },
    );

    quote! {
        #expansion
        #generic_fields
    }
}
