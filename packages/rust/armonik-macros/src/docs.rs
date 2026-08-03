//! Doc harvesting for the message types: the `#[armonik_macros::message]` /
//! `#[armonik_macros::enumeration]` attribute macros re-emit the annotated
//! item with `#[doc]` attributes extracted from the protos' comments
//! (type, fields, oneof variants, enum values), then append the same
//! expansion the old derives produced.
//!
//! An attribute macro (unlike a derive) may rewrite the item — that is the
//! whole reason these are attributes: the proto prose becomes uncopyable, as
//! it already is for the services. Injected docs come first; hand-written doc
//! comments remain after them, for Rust-specific notes. The `#[armonik(...)]`
//! attributes are consumed here and stripped from the re-emitted item.

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::attrs::{self, AttrItem};
use crate::descriptor::EnumMeta;

pub(crate) enum Mode {
    Message,
    Enumeration,
}

pub(crate) fn expand(mut input: DeriveInput, mode: Mode) -> syn::Result<TokenStream> {
    // The old derive expansion first, over the pristine input (it reads the
    // `#[armonik(...)]` attributes).
    let expansion = match mode {
        Mode::Message => crate::expand_message(input.clone())?,
        Mode::Enumeration => crate::expand_enumeration(input.clone())?,
    };

    inject(&mut input, &mode)?;
    strip(&mut input);

    Ok(quote! {
        #input
        #expansion
    })
}

/// Inject the harvested `#[doc]`s: on the type, its named fields, its oneof
/// variants (matched to members like the resolver does, by snake_cased name
/// or `rename`), its struct-variant fields, and its enum values.
fn inject(input: &mut DeriveInput, mode: &Mode) -> syn::Result<()> {
    // The proto the type stands for: the first `message =` / `enum =` name
    // (unified types agree on their shape, the first one documents it).
    // `generic` types name no proto and get nothing.
    let entries = attrs::parse(&input.attrs)?;
    let proto = entries.iter().find_map(|entry| match &entry.item {
        AttrItem::Message(lit) | AttrItem::Enum(lit) => Some(lit.value()),
        _ => None,
    });
    let Some(proto) = proto else {
        return Ok(());
    };
    let index = crate::load_index(input)?;

    if let (Mode::Enumeration, Some(meta)) = (&mode, index.enums.get(&proto)) {
        return inject_enumeration(input, meta);
    }
    let Some(meta) = index.messages.get(&proto) else {
        // Transparent enums name wrapper *messages*; type docs only.
        if let Some(meta) = index.enums.get(&proto) {
            prepend(&mut input.attrs, &meta.docs);
        }
        return Ok(());
    };

    prepend(&mut input.attrs, &meta.docs);

    let field_docs = |attrs: &[syn::Attribute], ident: Option<&syn::Ident>| -> Vec<String> {
        let name = renamed(attrs).or_else(|| ident.map(ToString::to_string));
        name.and_then(|name| {
            meta.fields
                .iter()
                .find(|field| field.name == name)
                .map(|field| field.docs.clone())
        })
        .unwrap_or_default()
    };

    match &mut input.data {
        syn::Data::Struct(data) => {
            for field in &mut data.fields {
                let docs = field_docs(&field.attrs, field.ident.as_ref());
                prepend(&mut field.attrs, &docs);
            }
        }
        syn::Data::Enum(data) => {
            // Whole-message and embedded-oneof enums: variants are oneof
            // members; struct-variant fields are sibling or inlined fields.
            for variant in &mut data.variants {
                let name = renamed(&variant.attrs)
                    .unwrap_or_else(|| crate::service::snake(&variant.ident.to_string()));
                let docs = meta
                    .fields
                    .iter()
                    .find(|field| field.name == name)
                    .map(|field| field.docs.clone())
                    .unwrap_or_default();
                prepend(&mut variant.attrs, &docs);
                if let syn::Fields::Named(fields) = &mut variant.fields {
                    for field in &mut fields.named {
                        let docs = field_docs(&field.attrs, field.ident.as_ref());
                        prepend(&mut field.attrs, &docs);
                    }
                }
            }
        }
        syn::Data::Union(_) => {}
    }
    Ok(())
}

fn inject_enumeration(input: &mut DeriveInput, meta: &EnumMeta) -> syn::Result<()> {
    prepend(&mut input.attrs, &meta.docs);

    let syn::Data::Enum(data) = &mut input.data else {
        return Ok(());
    };
    // prost-style value matching, as the resolver does: the value name with
    // the enum-name prefix stripped, PascalCased — or the full name via
    // `rename`.
    let prefix = format!("{}_", crate::service::snake(&input.ident.to_string())).to_uppercase();
    for variant in &mut data.variants {
        let docs = meta
            .values
            .iter()
            .zip(&meta.value_docs)
            .find(|((name, _), _)| match renamed(&variant.attrs) {
                Some(rename) => name == &rename,
                None => {
                    variant.ident == pascal(name.strip_prefix(&prefix).unwrap_or(name))
                }
            })
            .map(|(_, docs)| docs.clone())
            .unwrap_or_default();
        prepend(&mut variant.attrs, &docs);
    }
    Ok(())
}

/// The `#[armonik(rename = "...")]` value among `attrs`, if any.
fn renamed(attrs: &[syn::Attribute]) -> Option<String> {
    attrs::parse(attrs).ok().and_then(|entries| {
        entries.iter().find_map(|entry| match &entry.item {
            AttrItem::Rename(lit) => Some(lit.value()),
            _ => None,
        })
    })
}

/// `SCREAMING_SNAKE` (or anything underscored) to `PascalCase`.
fn pascal(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect()
}

/// Put the harvested docs *before* the existing attributes, so hand-written
/// doc comments read as additional notes after the proto prose.
fn prepend(attrs: &mut Vec<syn::Attribute>, docs: &[String]) {
    for line in docs.iter().rev() {
        attrs.insert(0, syn::parse_quote!(#[doc = #line]));
    }
}

/// Remove every `#[armonik(...)]` attribute: they were consumed by the
/// expansion and are not registered anywhere once the item is re-emitted.
fn strip(input: &mut DeriveInput) {
    fn retain(attrs: &mut Vec<syn::Attribute>) {
        attrs.retain(|attr| !attr.path().is_ident("armonik"));
    }
    retain(&mut input.attrs);
    match &mut input.data {
        syn::Data::Struct(data) => {
            for field in &mut data.fields {
                retain(&mut field.attrs);
            }
        }
        syn::Data::Enum(data) => {
            for variant in &mut data.variants {
                retain(&mut variant.attrs);
                for field in &mut variant.fields {
                    retain(&mut field.attrs);
                }
            }
        }
        syn::Data::Union(_) => {}
    }
}
