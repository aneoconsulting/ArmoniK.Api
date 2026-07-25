//! Expansion entry points for the derives.
//!
//! Struct messages are fully implemented; oneof/generic modes and the enum
//! derive land with the later stages of the direct-wire revamp.

use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::DeriveInput;

use crate::attrs::{self, AttrItem};
use crate::descriptor::{self, DescriptorIndex};
use crate::{codegen, resolve};

pub(crate) fn message(input: DeriveInput) -> syn::Result<TokenStream> {
    let index = load_index(&input)?;
    let plan =
        resolve::message_plan(&input, &index).map_err(crate::errors::Errors::into_syn_error)?;
    Ok(codegen::message(&plan))
}

pub(crate) fn enumeration(input: DeriveInput) -> syn::Result<TokenStream> {
    let entries = attrs::parse(&input.attrs)?;
    let index = load_index(&input)?;

    let mut enums = Vec::new();
    let mut transparent_messages = Vec::new();
    let mut transparent = false;
    for entry in &entries {
        match &entry.item {
            AttrItem::Enum(lit) => enums.push((entry.span, lit.value())),
            AttrItem::Message(lit) => transparent_messages.push((entry.span, lit.value())),
            AttrItem::Transparent => transparent = true,
            _ => {}
        }
    }

    if transparent {
        if transparent_messages.is_empty() {
            return Err(syn::Error::new(
                input.ident.span(),
                "#[armonik(transparent)] requires #[armonik(message = \"full.proto.Name\")] \
                 naming the single-field wrapper message",
            ));
        }
        for (span, name) in &transparent_messages {
            if !index.messages.contains_key(name) {
                return Err(syn::Error::new(
                    *span,
                    format!("proto message `{name}` not found in the compiled descriptor set"),
                ));
            }
        }
    } else if enums.is_empty() {
        return Err(syn::Error::new(
            input.ident.span(),
            "missing #[armonik(enum = \"full.proto.Name\")]",
        ));
    }
    for (span, name) in &enums {
        if !index.enums.contains_key(name) {
            return Err(syn::Error::new(
                *span,
                format!("proto enum `{name}` not found in the compiled descriptor set"),
            ));
        }
    }

    Ok(fingerprint_tripwire(&input, &index))
}

fn load_index(input: &DeriveInput) -> syn::Result<std::sync::Arc<DescriptorIndex>> {
    descriptor::index().map_err(|message| syn::Error::new(input.ident.span(), message))
}

/// Emit a const-assert pinning the fingerprint of the descriptor this
/// expansion was validated against: if any caching layer ever replays the
/// expansion against a rebuilt descriptor, compilation fails instead of
/// silently drifting.
fn fingerprint_tripwire(input: &DeriveInput, index: &DescriptorIndex) -> TokenStream {
    let fingerprint = proc_macro2::Literal::u128_suffixed(index.fingerprint);
    quote_spanned! {input.ident.span()=>
        const _: () = assert!(
            crate::__schema::DESCRIPTOR_FINGERPRINT == #fingerprint,
            "armonik: a derive was expanded against a stale protobuf descriptor; \
             rebuild the crate"
        );
    }
}
