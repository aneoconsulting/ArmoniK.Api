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
    let mut absorbs = collect_absorbs(&input);
    if has_oneof || (matches!(input.data, syn::Data::Enum(_)) && !generic) {
        let plan = resolve::oneof_plan(&input, &index).map_err(Errors::into_syn_error)?;
        absorbs.extend(plan.absorbs.iter().cloned());
        out.extend(codegen::oneof(&plan));
    } else {
        let plan = resolve::message_plan(&input, &index).map_err(Errors::into_syn_error)?;
        out.extend(codegen::message(&plan));
    }
    out.extend(absorbed(absorbs));
    Ok(out)
}

pub(crate) fn enumeration(input: DeriveInput) -> syn::Result<TokenStream> {
    let index = load_index(&input)?;
    let plan = resolve::enum_plan(&input, &index).map_err(Errors::into_syn_error)?;
    let mut out = doc_anchors(&input, "Enum");
    let mut absorbs = collect_absorbs(&input);
    absorbs.extend(plan.absorbs.iter().cloned());
    out.extend(codegen::enumeration(&plan));
    out.extend(absorbed(absorbs));
    Ok(out)
}

/// The explicit `#[armonik(absorbs = "...")]` names on any field/variant of
/// the input (auto-collected transparent/inline ones come from the plan).
fn collect_absorbs(input: &DeriveInput) -> Vec<String> {
    fn push(attrs: &[syn::Attribute], out: &mut Vec<String>) {
        if let Ok(entries) = attrs::parse(attrs) {
            for entry in entries {
                if let AttrItem::Absorbs(lit) = entry.item {
                    out.push(lit.value());
                }
            }
        }
    }
    let mut out = Vec::new();
    push(&input.attrs, &mut out);
    match &input.data {
        syn::Data::Struct(data) => {
            for field in &data.fields {
                push(&field.attrs, &mut out);
            }
        }
        syn::Data::Enum(data) => {
            for variant in &data.variants {
                push(&variant.attrs, &mut out);
                for field in &variant.fields {
                    push(&field.attrs, &mut out);
                }
            }
        }
        syn::Data::Union(_) => {}
    }
    out
}

fn absorbed(mut names: Vec<String>) -> TokenStream {
    names.sort();
    names.dedup();
    codegen::absorbed_registrations(&names)
}

/// `#[armonik_macros::alias("proto.Name")]` on a `type` alias: re-emit the
/// alias and register `(proto name, Rust path)` the way a derive would, so
/// generic instantiations that carry no annotation of their own are still
/// harvested. No descriptor validation — the concrete instantiation is
/// covered by the differential harness like any generic type.
pub(crate) fn alias(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let proto: syn::LitStr = syn::parse2(attr).map_err(|err| {
        syn::Error::new(
            err.span(),
            "#[alias(...)] takes a single string literal: the full proto message name",
        )
    })?;
    let item_type: syn::ItemType = syn::parse2(item)?;
    let name = proto.value();
    let registrations = codegen::registrations(&item_type.ident, std::slice::from_ref(&name), None);
    Ok(quote::quote! {
        #item_type
        #registrations
    })
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
