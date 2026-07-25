//! Expansion entry points for the derives.
//!
//! Struct messages and enums (plain and transparent) are fully implemented;
//! oneof and generic modes land with the later stages of the direct-wire
//! revamp.

use proc_macro2::TokenStream;
use syn::DeriveInput;

use crate::descriptor::{self, DescriptorIndex};
use crate::errors::Errors;
use crate::{codegen, resolve};

pub(crate) fn message(input: DeriveInput) -> syn::Result<TokenStream> {
    let index = load_index(&input)?;
    let plan = resolve::message_plan(&input, &index).map_err(Errors::into_syn_error)?;
    Ok(codegen::message(&plan))
}

pub(crate) fn enumeration(input: DeriveInput) -> syn::Result<TokenStream> {
    let index = load_index(&input)?;
    let plan = resolve::enum_plan(&input, &index).map_err(Errors::into_syn_error)?;
    Ok(codegen::enumeration(&plan))
}

fn load_index(input: &DeriveInput) -> syn::Result<std::sync::Arc<DescriptorIndex>> {
    descriptor::index().map_err(|message| syn::Error::new(input.ident.span(), message))
}
