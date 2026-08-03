//! The `FromEnv` derive macro for `armonik-transport`'s own environment-variable reading.
//!
//! Internal to `armonik-transport`: the generated code names `crate::env::...` directly rather than
//! resolving its own crate name, which only holds because the derive is used only on types defined
//! inside that crate. Moving it to a separate consumer would need that resolution added.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Fields};

/// Derives `crate::env::FromEnv`.
///
/// On a struct: one call to `EnvSource::field` per named field, naming itself by its own Rust field
/// name. A field marked `#[env(skip)]` is left at its `Default::default()` instead, for a field this
/// mechanism cannot describe.
///
/// On an enum, which of two shapes it is decides what gets generated, the same way `Certificate`
/// (a union) and, say, a plain `LogLevel` (a C-like enum) read differently:
/// - One variant marked `#[env(bare)]`: that variant, a single-field tuple, is what a bare
///   environment string becomes, delegating to its inner type's own `FromEnv`. On the variant itself,
///   not naming it from the container, so renaming the variant cannot desynchronise it from what the
///   attribute means. Needed because which of possibly several data-carrying variants is "the bare
///   one" cannot be inferred from the shape alone, the way it can be for the case below; more than
///   one `#[env(bare)]` variant is a compile error.
/// - No variant marked: every variant must carry no data, and the environment string selects one by
///   name, matched case-insensitively against its Rust identifier. A data-carrying variant with none
///   marked `#[env(bare)]` is a compile error asking for one, rather than a silent guess.
#[proc_macro_derive(FromEnv, attributes(env))]
pub fn derive_from_env(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match &input.data {
        Data::Struct(data) => derive_struct(&input, data),
        Data::Enum(data) => derive_enum(&input, data),
        Data::Union(_) => syn::Error::new_spanned(&input, "FromEnv cannot be derived for a union")
            .to_compile_error()
            .into(),
    }
}

fn derive_struct(input: &DeriveInput, data: &DataStruct) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let Fields::Named(fields) = &data.fields else {
        return syn::Error::new_spanned(input, "FromEnv requires named fields")
            .to_compile_error()
            .into();
    };

    let field_inits = fields.named.iter().map(|field| {
        let ident = field.ident.as_ref().expect("checked: Fields::Named");
        if has_env_flag(&field.attrs, "skip") {
            quote! { #ident: ::core::default::Default::default() }
        } else {
            let name = ident.to_string();
            quote! { #ident: crate::env::FromEnv::from_env(&source.field(#name))? }
        }
    });

    quote! {
        impl #impl_generics crate::env::FromEnv for #name #ty_generics #where_clause {
            fn from_env(
                source: &crate::env::EnvSource<'_>,
            ) -> ::core::result::Result<Self, crate::env::EnvFieldError> {
                ::core::result::Result::Ok(Self {
                    #(#field_inits,)*
                })
            }
        }
    }
    .into()
}

fn derive_enum(input: &DeriveInput, data: &DataEnum) -> TokenStream {
    let mut bare_variants = data
        .variants
        .iter()
        .filter(|variant| has_env_flag(&variant.attrs, "bare"));

    let Some(first) = bare_variants.next() else {
        return derive_unit_enum(input, data);
    };
    if let Some(second) = bare_variants.next() {
        return syn::Error::new_spanned(
            second,
            format!(
                "only one variant can be `#[env(bare)]`; `{}` already is",
                first.ident,
            ),
        )
        .to_compile_error()
        .into();
    }

    derive_data_enum(input, first)
}

/// A union: the variant marked `#[env(bare)]` is what a bare environment string becomes, delegating
/// to its inner type's own `FromEnv`.
fn derive_data_enum(input: &DeriveInput, variant: &syn::Variant) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let variant_ident = &variant.ident;

    let Fields::Unnamed(unnamed) = &variant.fields else {
        return syn::Error::new_spanned(
            variant,
            "the bare variant must be a single-field tuple variant, e.g. `Path(String)`",
        )
        .to_compile_error()
        .into();
    };
    let Some(inner) = unnamed
        .unnamed
        .first()
        .filter(|_| unnamed.unnamed.len() == 1)
    else {
        return syn::Error::new_spanned(variant, "the bare variant must carry exactly one field")
            .to_compile_error()
            .into();
    };
    let inner_ty = &inner.ty;

    quote! {
        impl #impl_generics crate::env::FromEnv for #name #ty_generics #where_clause {
            fn from_env(
                source: &crate::env::EnvSource<'_>,
            ) -> ::core::result::Result<Self, crate::env::EnvFieldError> {
                <#inner_ty as crate::env::FromEnv>::from_env(source).map(Self::#variant_ident)
            }
        }
    }
    .into()
}

/// A C-like enum: every variant carries no data, and the environment string selects one by name,
/// case-insensitively.
fn derive_unit_enum(input: &DeriveInput, data: &DataEnum) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    if let Some(variant) = data
        .variants
        .iter()
        .find(|variant| !matches!(variant.fields, Fields::Unit))
    {
        return syn::Error::new_spanned(
            variant,
            format!(
                "`{}` carries data; mark the variant a bare environment string should become with \
                 `#[env(bare)]`",
                variant.ident,
            ),
        )
        .to_compile_error()
        .into();
    }

    let variant_idents: Vec<_> = data.variants.iter().map(|variant| &variant.ident).collect();
    let variant_names: Vec<String> = variant_idents
        .iter()
        .map(|ident| ident.to_string())
        .collect();

    quote! {
        impl #impl_generics crate::env::FromEnv for #name #ty_generics #where_clause {
            fn from_env(
                source: &crate::env::EnvSource<'_>,
            ) -> ::core::result::Result<Self, crate::env::EnvFieldError> {
                let (name, text) = source.read_text()?;
                #(
                    if text.eq_ignore_ascii_case(#variant_names) {
                        return ::core::result::Result::Ok(Self::#variant_idents);
                    }
                )*
                ::core::result::Result::Err(crate::env::EnvFieldError::not_a_variant(
                    name,
                    text,
                    &[#(#variant_names),*],
                ))
            }
        }
    }
    .into()
}

/// Whether an `#[env(..)]` attribute in `attrs` sets the bare flag `flag`, e.g. `skip` in
/// `#[env(skip)]`.
fn has_env_flag(attrs: &[syn::Attribute], flag: &str) -> bool {
    let mut found = false;
    for attr in attrs {
        if !attr.path().is_ident("env") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(flag) {
                found = true;
            }
            Ok(())
        });
    }
    found
}
