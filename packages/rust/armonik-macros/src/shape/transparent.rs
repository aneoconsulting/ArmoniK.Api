//! `#[armonik(transparent)]`: a single-field newtype delegating its whole impl to that field, so it
//! is wire-identical to the field's message and can stand for a whole RPC message.

use proc_macro2::{Span, TokenStream};
use quote::quote;

use crate::attr_site::field_access;
use crate::attrs::Errors;
use crate::descriptor::DescriptorIndex;
use crate::emit::{message_shaped, MessageBodies};
use crate::matcher::not_found;
use crate::plan::{MessagePlan, Slot, SlotCodec};
use syn::spanned::Spanned;

/// Plan for a `#[armonik(transparent)]` struct: a single-field newtype that delegates its whole
/// `prost::Message` impl to the field, so it is wire-identical to the inner message. The field is
/// not matched against the descriptor (the inner type already validates itself); only the named
/// proto message is checked to exist, and it is used for registration.
pub(crate) fn transparent_plan(
    input: &syn::DeriveInput,
    index: &DescriptorIndex,
    proto_names: Vec<(Span, String)>,
    mut errors: Errors,
) -> Result<MessagePlan, Errors> {
    if !input.generics.params.is_empty() {
        errors.at(
            input.ident.span(),
            "#[armonik(transparent)] structs cannot be generic",
        );
    }
    if proto_names.len() != 1 {
        errors.at(
            input.ident.span(),
            "#[armonik(transparent)] structs need exactly one \
             #[armonik(message = \"full.proto.Name\")]",
        );
    }
    for (span, name) in &proto_names {
        if !index.messages.contains_key(name) {
            errors.push(not_found(*span, "message", name));
        }
    }
    let syn::Data::Struct(data) = &input.data else {
        errors.at(
            input.ident.span(),
            "#[armonik(transparent)] expects a struct",
        );
        return Err(errors);
    };
    if data.fields.len() != 1 {
        errors.at(
            input.ident.span(),
            "#[armonik(transparent)] structs must have exactly one field, delegated to",
        );
        return Err(errors);
    }
    let field = data.fields.iter().next().expect("one field");
    let (_, access) = field_access(field, 0);
    let delegate = Slot {
        access: Some(access),
        span: field.ty.span(),
        tag: 0,
        codec: SlotCodec::Field {
            ty: Box::new(field.ty.clone()),
            adapter: None,
        },
        checks: None,
        proto_path: String::new(),
        // The delegate is not matched against the descriptor; the inner type documents itself.
        docs: Vec::new(),
    };

    errors.into_result()?;

    let docs = proto_names
        .first()
        .and_then(|(_, name)| index.messages.get(name))
        .map(|meta| meta.docs.clone())
        .unwrap_or_default();

    Ok(MessagePlan {
        ident: input.ident.clone(),
        proto_names: proto_names.into_iter().map(|(_, name)| name).collect(),
        docs,
        fields: vec![delegate],
        generics: input.generics.clone(),
        fingerprint: index.fingerprint,
        transparent: true,
        absorbs: Vec::new(),
    })
}

/// Codegen for a `#[armonik(transparent)]` struct: a single-field newtype whose `prost::Message`
/// impl delegates entirely to the field, so it is wire-identical to the inner message and can stand
/// for a whole RPC message. The `Normalize` projection delegates likewise.
pub(crate) fn transparent_message(plan: &MessagePlan, generics: &syn::Generics) -> TokenStream {
    let field = &plan.fields[0];
    let access = &field.access;
    let ty = field.ty().expect("the delegate carries a value");

    message_shaped(
        &plan.ident,
        generics,
        plan.fingerprint,
        &plan.proto_names,
        true,
        // Nothing to assert: the field is not matched against the descriptor, because the inner
        // type already validates itself.
        TokenStream::new(),
        MessageBodies {
            encode_raw: quote! { <#ty as ::prost::Message>::encode_raw(&self.#access, buf); },
            merge_field: quote! {
                <#ty as ::prost::Message>::merge_field(&mut self.#access, tag, wire_type, buf, ctx)
            },
            encoded_len: quote! { <#ty as ::prost::Message>::encoded_len(&self.#access) },
            normalize: vec![
                quote! { <#ty as crate::differential::Normalize>::normalize(message); },
            ],
        },
    )
}
