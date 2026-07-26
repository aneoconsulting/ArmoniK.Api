//! Expansion entry points for the derives.
//!
//! Struct messages, enums (plain and transparent) and flattened oneofs are
//! implemented; the generic mode lands with the later stages of the
//! direct-wire revamp.

use proc_macro2::TokenStream;
use syn::DeriveInput;

use crate::attrs::{self, AttrItem};
use crate::descriptor::{self, DescriptorIndex};
use crate::errors::Errors;
use crate::{codegen, resolve};

pub(crate) fn message(input: DeriveInput) -> syn::Result<TokenStream> {
    let index = load_index(&input)?;
    let entries = attrs::parse(&input.attrs)?;
    let has_oneof = entries
        .iter()
        .any(|entry| matches!(entry.item, AttrItem::Oneof(_)));
    let generic = entries
        .iter()
        .any(|entry| matches!(entry.item, AttrItem::Generic));
    // Enums are oneof-shaped: `message = ...` alone stands for a whole
    // message with a single (inferred) oneof, `oneof = ...` for one oneof
    // of a message, embedded in a struct.
    if has_oneof || (matches!(input.data, syn::Data::Enum(_)) && !generic) {
        let plan = resolve::oneof_plan(&input, &index).map_err(Errors::into_syn_error)?;
        Ok(codegen::oneof(&plan))
    } else {
        let plan = resolve::message_plan(&input, &index).map_err(Errors::into_syn_error)?;
        Ok(codegen::message(&plan))
    }
}

pub(crate) fn enumeration(input: DeriveInput) -> syn::Result<TokenStream> {
    let index = load_index(&input)?;
    let plan = resolve::enum_plan(&input, &index).map_err(Errors::into_syn_error)?;
    Ok(codegen::enumeration(&plan))
}

fn load_index(input: &DeriveInput) -> syn::Result<std::sync::Arc<DescriptorIndex>> {
    descriptor::index().map_err(|message| syn::Error::new(input.ident.span(), message))
}
