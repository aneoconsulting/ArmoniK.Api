//! `#[reflect]`: field reflection for a type alias of a derived message.
//!
//! The convenience emission names a message's field types through the flat
//! `__armonik_ty_*` aliases the derive puts next to the struct, and finds them
//! by mangling the type path written on the rpc line: for
//! `count_tasks::Response` it looks up `__armonik_ty_response_*` in
//! `count_tasks`. An alias (`pub type Response = Count;`) has no reflection of
//! its own (the derive emitted it next to `Count`, under the `count` stem),
//! which leaves such a response projectable only as a whole.
//!
//! This attribute carries the reflection over to the alias:
//!
//! ```ignore
//! #[armonik_macros::reflect]
//! pub type Response = super::super::Count;
//! ```
//!
//! re-emits the alias, then re-exports the source module's reflection under the
//! alias's own stem: the `__armonik_fields_*` callback (so `auto` projection
//! and, on a request alias, the parameter list work) and one renaming import
//! per field type alias. The field names come from chaining once through the
//! source callback into `__emit_reflect`.
//!
//! Everything is emitted as `use` items *in the alias's module*, so the alias's
//! own relative right-hand side is all the macro needs; contrast
//! `__emit_convenience`, which expands in a different module and therefore
//! cannot be handed relative paths.
//!
//! The reflection's module is the right-hand side with the type's snake name
//! appended (`super::super::Count` gives `super::super::count`), this crate's
//! one-object-per-file convention; a right-hand side that already spells the
//! defining module (`super::super::count::Count`) is taken as is.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemType, Path, PathArguments, PathSegment, Type};

use crate::callback::{braced, fields, Class};
use crate::names::snake;

mod kw {
    syn::custom_keyword!(source);
    syn::custom_keyword!(stem);
    syn::custom_keyword!(target);
    syn::custom_keyword!(fields);
}

pub(crate) fn expand_attribute(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let item: ItemType = syn::parse2(item)?;
    if !attr.is_empty() {
        return Err(syn::Error::new(
            item.ident.span(),
            "#[armonik_macros::reflect] takes no arguments",
        ));
    }
    let (source, stem) = source(&item)?;
    let target = format_ident!(
        "{}",
        snake(&item.ident.to_string()),
        span = item.ident.span()
    );
    let source_callback = format_ident!("__armonik_fields_{stem}");
    let target_callback = format_ident!("__armonik_fields_{target}");

    Ok(quote! {
        #item

        #[doc(hidden)]
        #[allow(unused_imports)]
        pub(crate) use #source::#source_callback as #target_callback;

        #source::#source_callback! { armonik_macros::__emit_reflect! {
            source { #source }
            stem { #stem }
            target { #target }
        } }
    })
}

/// The module holding the aliased struct's reflection, and the struct's
/// mangling stem, from the alias's right-hand side.
fn source(item: &ItemType) -> syn::Result<(Path, Ident)> {
    let Type::Path(ty) = &*item.ty else {
        return Err(syn::Error::new_spanned(
            &item.ty,
            "`reflect` needs a path right-hand side naming a derived message",
        ));
    };
    if ty.qself.is_some() {
        return Err(syn::Error::new_spanned(
            &item.ty,
            "a qualified right-hand side carries no reflection",
        ));
    }
    let mut segments = ty.path.segments.iter().cloned().collect::<Vec<_>>();
    let last = segments.pop().expect("a path has at least one segment");
    if !matches!(last.arguments, PathArguments::None) {
        return Err(syn::Error::new_spanned(
            &item.ty,
            "a generic right-hand side carries no reflection (the derive emits none)",
        ));
    }
    let stem = format_ident!(
        "{}",
        snake(&last.ident.to_string()),
        span = last.ident.span()
    );
    if segments.last().is_none_or(|parent| parent.ident != stem) {
        segments.push(PathSegment::from(stem.clone()));
    }
    Ok((
        Path {
            leading_colon: ty.path.leading_colon,
            segments: segments.into_iter().collect(),
        },
        stem,
    ))
}

/// The continuation of the source callback: the field type aliases, re-exported
/// under the alias's stem.
pub(crate) struct Reflect {
    source: Path,
    stem: Ident,
    target: Ident,
    fields: Vec<(Ident, Class)>,
}

impl Parse for Reflect {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<kw::source>()?;
        let source = braced(input, |c| c.parse())?;
        input.parse::<kw::stem>()?;
        let stem = braced(input, |c| c.parse())?;
        input.parse::<kw::target>()?;
        let target = braced(input, |c| c.parse())?;
        input.parse::<kw::fields>()?;
        let fields = braced(input, fields)?;
        Ok(Reflect {
            source,
            stem,
            target,
            fields,
        })
    }
}

pub(crate) fn expand(tokens: TokenStream) -> syn::Result<TokenStream> {
    let reflect: Reflect = syn::parse2(tokens)?;
    let Reflect {
        source,
        stem,
        target,
        fields,
    } = &reflect;

    let imports = fields
        .iter()
        .flat_map(|(name, class)| class.suffixes(name))
        .map(|suffix| {
            let from = format_ident!("__armonik_ty_{stem}_{suffix}");
            let to = format_ident!("__armonik_ty_{target}_{suffix}");
            quote! {
                #[doc(hidden)]
                #[allow(non_camel_case_types, unused_imports)]
                pub(crate) use #source::#from as #to;
            }
        });

    Ok(quote!(#(#imports)*))
}

#[cfg(test)]
mod tests {
    use quote::quote;

    fn attribute(item: proc_macro2::TokenStream) -> String {
        super::expand_attribute(proc_macro2::TokenStream::new(), item)
            .expect("expands")
            .to_string()
    }

    #[test]
    fn the_alias_keeps_its_item_and_gains_the_callback_and_the_chain() {
        let out = attribute(quote! {
            /// Docs stay.
            pub type Response = super::super::Count;
        });
        assert!(
            out.contains("pub type Response = super :: super :: Count ;"),
            "{out}"
        );
        assert!(out.contains("# [doc = r\" Docs stay.\"]"), "{out}");
        assert!(
            out.contains(
                "pub (crate) use super :: super :: count :: __armonik_fields_count \
                 as __armonik_fields_response ;"
            ),
            "{out}"
        );
        assert!(
            out.contains(
                "super :: super :: count :: __armonik_fields_count ! \
                 { armonik_macros :: __emit_reflect ! { source { super :: super :: count } \
                 stem { count } target { response } } }"
            ),
            "{out}"
        );
    }

    #[test]
    fn a_right_hand_side_spelling_the_defining_module_is_taken_as_is() {
        let out = attribute(quote! {
            pub type Response = super::super::count::Count;
        });
        assert!(out.contains("source { super :: super :: count }"), "{out}");
        assert!(!out.contains("count :: count"), "{out}");
    }

    #[test]
    fn a_generic_right_hand_side_is_rejected() {
        let err = super::expand_attribute(
            proc_macro2::TokenStream::new(),
            quote!(
                pub type Sort = super::Sort<Field>;
            ),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("generic"), "{err}");
    }

    #[test]
    fn every_field_type_alias_of_the_source_is_re_exported() {
        let out = super::expand(quote! {
            source { super::super::count } stem { count } target { response }
            fields { [values pairs] [ids iter] [name into] }
        })
        .expect("expands")
        .to_string();
        for suffix in [
            "values",
            "values_key",
            "values_value",
            "ids",
            "ids_elem",
            "name",
        ] {
            assert!(
                out.contains(&format!(
                    "pub (crate) use super :: super :: count :: __armonik_ty_count_{suffix} \
                     as __armonik_ty_response_{suffix} ;"
                )),
                "{suffix}: {out}"
            );
        }
    }
}
