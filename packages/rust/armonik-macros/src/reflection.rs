//! The field reflection a derive emits next to each struct: the `__armonik_fields_*` callback macro
//! carrying the fields CPS-style, and the flat `__armonik_ty_*` aliases naming their types.
//!
//! `service!` continues the callback into `__emit_convenience`, its only consumer (`callback.rs`
//! holds the parsing side). This is the emitting half; it lives here rather than in `lib.rs`, which
//! is the first file anyone opens and is otherwise the grammar reference.

use proc_macro2::TokenStream as TokenStream2;
use syn::DeriveInput;

use crate::names;

/// Field reflection for the `service!` convenience emission: a callback macro forwarding each
/// field's name and sugar class in declaration order (which is the generated method's parameter
/// order), plus flat per-field type aliases so the consuming proc macro can name field and element
/// types from another module. The aliases resolve the field's type tokens here, where they mean the
/// right thing. `__emit_convenience` is the consuming side.
pub(crate) fn reflection(input: &DeriveInput) -> TokenStream2 {
    let syn::Data::Struct(data) = &input.data else {
        return TokenStream2::new();
    };
    let syn::Fields::Named(fields) = &data.fields else {
        return TokenStream2::new();
    };
    if !input.generics.params.is_empty() {
        return TokenStream2::new();
    }

    let snake = names::snake(&input.ident.to_string());
    let fields_macro = quote::format_ident!("__armonik_fields_{snake}");

    let mut units = Vec::new();
    let mut aliases = Vec::new();
    let mut alias = |suffix: &String, ty: &dyn quote::ToTokens| {
        let name = quote::format_ident!("__armonik_ty_{snake}_{suffix}");
        aliases.push(quote::quote! {
            #[doc(hidden)]
            #[allow(non_camel_case_types, dead_code)]
            pub(crate) type #name = #ty;
        });
    };
    for field in &fields.named {
        let name = field.ident.as_ref().expect("named");
        let ty = &field.ty;
        let class = sugar(ty);
        alias(&name.to_string(), &ty);
        match &class {
            Sugar::Iter(elem) => alias(&format!("{name}_elem"), elem),
            Sugar::Filters(elem) => alias(&format!("{name}_elem"), elem),
            Sugar::Pairs(key, value) => {
                alias(&format!("{name}_key"), key);
                alias(&format!("{name}_value"), value);
            }
            Sugar::Plain | Sugar::Into => {}
        }
        let class = match class {
            Sugar::Plain => quote::quote!(plain),
            Sugar::Into => quote::quote!(into),
            Sugar::Iter(_) => quote::quote!(iter),
            Sugar::Pairs(..) => quote::quote!(pairs),
            Sugar::Filters(_) => quote::quote!(filters),
        };
        units.push(quote::quote!([#name #class]));
    }

    quote::quote! {
        #[doc(hidden)]
        macro_rules! #fields_macro {
            ($($cont:tt)::* ! { $($ctx:tt)* }) => {
                $($cont)::* ! { $($ctx)* fields { #(#units)* } }
            };
        }
        #[doc(hidden)]
        pub(crate) use #fields_macro;

        #(#aliases)*
    }
}
/// How the generated signature widens a request field's type, and how the body converts it back.
/// Conservative: anything unrecognized passes through unchanged, and a whole method opts out with
/// `manual` on its rpc line.
#[allow(clippy::large_enum_variant)] // transient parse-time value, a handful per struct
enum Sugar {
    Plain,
    Into,
    Iter(syn::Type),
    Pairs(syn::Type, syn::Type),
    Filters(syn::Path),
}
fn sugar(ty: &syn::Type) -> Sugar {
    let syn::Type::Path(path) = ty else {
        return Sugar::Plain;
    };
    let Some(segment) = path.path.segments.last() else {
        return Sugar::Plain;
    };
    let arg = |index: usize| match &segment.arguments {
        syn::PathArguments::AngleBracketed(args) => {
            args.args.iter().nth(index).and_then(|arg| match arg {
                syn::GenericArgument::Type(ty) => Some(ty.clone()),
                _ => None,
            })
        }
        _ => None,
    };
    match segment.ident.to_string().as_str() {
        "String" | "Bytes" => Sugar::Into,
        "Vec" => match arg(0) {
            // `Vec<u8>` is a payload, not a collection of convertibles.
            Some(syn::Type::Path(elem)) if elem.path.is_ident("u8") => Sugar::Into,
            Some(elem) => Sugar::Iter(elem),
            None => Sugar::Plain,
        },
        "HashMap" => match (arg(0), arg(1)) {
            (Some(key), Some(value)) => Sugar::Pairs(key, value),
            _ => Sugar::Plain,
        },
        // The per-service `filter::Or`, whose sibling `Field` is the element type of the
        // nested-iterator sugar.
        "Or" => {
            let mut field = path.path.clone();
            field.segments.last_mut().expect("segment").ident =
                syn::Ident::new("Field", segment.ident.span());
            Sugar::Filters(field)
        }
        _ => Sugar::Plain,
    }
}
