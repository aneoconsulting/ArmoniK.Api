//! Expansion entry points for the derives.

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
    let mut out = doc_anchors(&input, "Message");
    if has_oneof || (matches!(input.data, syn::Data::Enum(_)) && !generic) {
        let plan = resolve::oneof_plan(&input, &index).map_err(Errors::into_syn_error)?;
        out.extend(codegen::oneof(&plan));
    } else {
        let plan = resolve::message_plan(&input, &index).map_err(Errors::into_syn_error)?;
        out.extend(codegen::message(&plan));
    }
    Ok(out)
}

pub(crate) fn enumeration(input: DeriveInput) -> syn::Result<TokenStream> {
    let index = load_index(&input)?;
    let plan = resolve::enum_plan(&input, &index).map_err(Errors::into_syn_error)?;
    let mut out = doc_anchors(&input, "Enum");
    out.extend(codegen::enumeration(&plan));
    Ok(out)
}

/// Hover-documentation anchors: re-emit every `#[armonik(...)]` key token
/// of the input as an anonymous import of the deriving macro, respanned
/// onto the key. IDE hover on the otherwise-inert helper attribute keys
/// then resolves to this crate's derive — the single home of the grammar
/// documentation. The anonymous `const` compiles to nothing.
fn doc_anchors(input: &DeriveInput, derive: &str) -> TokenStream {
    let mut spans = attrs::key_spans(&input.attrs);
    match &input.data {
        syn::Data::Struct(data) => {
            for field in &data.fields {
                spans.extend(attrs::key_spans(&field.attrs));
            }
        }
        syn::Data::Enum(data) => {
            for variant in &data.variants {
                spans.extend(attrs::key_spans(&variant.attrs));
                for field in &variant.fields {
                    spans.extend(attrs::key_spans(&field.attrs));
                }
            }
        }
        syn::Data::Union(_) => {}
    }
    if spans.is_empty() {
        return TokenStream::new();
    }
    let uses = spans.iter().map(|span| {
        let derive = syn::Ident::new(derive, *span);
        quote::quote! {
            {
                #[allow(unused_imports)]
                use ::armonik_macros::#derive as _;
            }
        }
    });
    quote::quote! {
        const _: () = {
            #(#uses)*
        };
    }
}

fn load_index(input: &DeriveInput) -> syn::Result<std::sync::Arc<DescriptorIndex>> {
    descriptor::index().map_err(|message| syn::Error::new(input.ident.span(), message))
}
