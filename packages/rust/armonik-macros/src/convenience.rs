//! `__emit_convenience`: the consuming side of the field reflection the
//! derives emit — builds one client convenience method per RPC, entirely from
//! the request struct's fields.
//!
//! `service!` emits, per convenience-eligible RPC, an invocation of the
//! request type's `__armonik_fields_*` callback with this macro as the
//! continuation; the callback appends a `fields { [name class]* }` block (the
//! handshake and its shared parsing live in `callback.rs`). For
//! `project { auto }` a second hop through the *response* type's callback
//! decides the projection (exactly one field → project it, else whole
//! response). Field and element types are never transported as tokens —
//! relative paths would re-resolve wrongly here — but as the flat
//! `__armonik_ty_*` aliases the derive defines next to each struct.
//!
//! The generated method: parameters mirror the request fields in declaration
//! order, widened per sugar class (`String`/`Bytes` → `impl Into`, `Vec<T>` →
//! `impl IntoIterator<Item = impl Into<T>>`, `HashMap<K, V>` → pair iterators,
//! `filter::Or` → nested iterators); the body is `self.call(request)` plus
//! the projection. `manual` on the rpc line opts a method out entirely.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Path};

use crate::callback::{braced, fields, Class};

mod kw {
    syn::custom_keyword!(marker);
    syn::custom_keyword!(method);
    syn::custom_keyword!(request);
    syn::custom_keyword!(response);
    syn::custom_keyword!(kind);
    syn::custom_keyword!(project);
    syn::custom_keyword!(deprecated);
    syn::custom_keyword!(docs);
    syn::custom_keyword!(fields);
    syn::custom_keyword!(unary);
    syn::custom_keyword!(server_stream);
    syn::custom_keyword!(auto);
    syn::custom_keyword!(whole);
    syn::custom_keyword!(field);
    syn::custom_keyword!(discard);
}

pub(crate) struct Emit {
    marker: Path,
    method: Ident,
    request: Path,
    response: Path,
    server_stream: bool,
    project: Project,
    deprecated: bool,
    docs: Vec<LitStr>,
    request_fields: Vec<(Ident, Class)>,
    response_fields: Option<Vec<(Ident, Class)>>,
}

enum Project {
    Auto,
    Whole,
    Discard,
    Field(Ident),
}

impl Parse for Emit {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<kw::marker>()?;
        let marker = braced(input, |c| c.parse())?;
        input.parse::<kw::method>()?;
        let method = braced(input, |c| c.parse())?;
        input.parse::<kw::request>()?;
        let request = braced(input, |c| c.parse())?;
        input.parse::<kw::response>()?;
        let response = braced(input, |c| c.parse())?;
        input.parse::<kw::kind>()?;
        let server_stream = braced(input, |c| {
            if c.peek(kw::unary) {
                c.parse::<kw::unary>()?;
                Ok(false)
            } else {
                c.parse::<kw::server_stream>()?;
                Ok(true)
            }
        })?;
        input.parse::<kw::project>()?;
        let project = braced(input, |c| {
            Ok(if c.peek(kw::auto) {
                c.parse::<kw::auto>()?;
                Project::Auto
            } else if c.peek(kw::whole) {
                c.parse::<kw::whole>()?;
                Project::Whole
            } else if c.peek(kw::discard) {
                c.parse::<kw::discard>()?;
                Project::Discard
            } else {
                c.parse::<kw::field>()?;
                Project::Field(c.parse()?)
            })
        })?;
        input.parse::<kw::deprecated>()?;
        let deprecated = braced(input, |c| {
            let value: syn::LitBool = c.parse()?;
            Ok(value.value)
        })?;
        input.parse::<kw::docs>()?;
        let docs = braced(input, |c| {
            let mut docs = Vec::new();
            while !c.is_empty() {
                docs.push(c.parse()?);
            }
            Ok(docs)
        })?;

        input.parse::<kw::fields>()?;
        let request_fields = braced(input, fields)?;
        let response_fields = if input.peek(kw::fields) {
            input.parse::<kw::fields>()?;
            Some(braced(input, fields)?)
        } else {
            None
        };

        Ok(Emit {
            marker,
            method,
            request,
            response,
            server_stream,
            project,
            deprecated,
            docs,
            request_fields,
            response_fields,
        })
    }
}

/// The parent module path and the `__armonik_*` mangling stem of a type path
/// (`crate::sessions::get::Request` → `crate::sessions::get`, `request`).
fn split(path: &Path) -> syn::Result<(Path, String)> {
    if path.segments.len() < 2 {
        return Err(syn::Error::new_spanned(path, "expected a `module::Type` path"));
    }
    let mut segments = path.segments.iter().cloned().collect::<Vec<_>>();
    let last = segments.pop().expect("checked above");
    Ok((
        Path {
            leading_colon: path.leading_colon,
            segments: segments.into_iter().collect(),
        },
        crate::service::snake(&last.ident.to_string()),
    ))
}

pub(crate) fn expand(tokens: TokenStream) -> syn::Result<TokenStream> {
    let emit: Emit = syn::parse2(tokens.clone())?;

    // `auto` projection needs the response's fields: chain through its
    // callback once, then decide.
    let project = match &emit.project {
        Project::Auto => match &emit.response_fields {
            None => {
                let (parent, stem) = split(&emit.response)?;
                let callback = format_ident!("__armonik_fields_{stem}");
                return Ok(quote! {
                    #parent::#callback! { armonik_macros::__emit_convenience! { #tokens } }
                });
            }
            Some(fields) => match &fields[..] {
                [] => Project::Discard,
                [(name, _)] => Project::Field(name.clone()),
                _ => Project::Whole,
            },
        },
        Project::Whole => Project::Whole,
        Project::Discard => Project::Discard,
        Project::Field(name) => Project::Field(name.clone()),
    };

    let (req_parent, req_stem) = split(&emit.request)?;
    let alias = |suffix: String| {
        let name = format_ident!("__armonik_ty_{req_stem}_{suffix}");
        quote!(#req_parent::#name)
    };

    let params = emit.request_fields.iter().map(|(name, class)| {
        let ty = alias(name.to_string());
        let param: TokenStream = match class {
            Class::Plain => quote!(#ty),
            Class::Into => quote!(impl ::core::convert::Into<#ty>),
            Class::Iter => {
                let elem = alias(format!("{name}_elem"));
                quote!(impl ::core::iter::IntoIterator<Item = impl ::core::convert::Into<#elem>>)
            }
            Class::Pairs => {
                let key = alias(format!("{name}_key"));
                let value = alias(format!("{name}_value"));
                quote!(impl ::core::iter::IntoIterator<
                    Item = (impl ::core::convert::Into<#key>, impl ::core::convert::Into<#value>),
                >)
            }
            Class::Filters => {
                let elem = alias(format!("{name}_elem"));
                quote!(impl ::core::iter::IntoIterator<
                    Item = impl ::core::iter::IntoIterator<Item = #elem>,
                >)
            }
        };
        quote!(#name: #param)
    });

    let conversions = emit.request_fields.iter().map(|(name, class)| {
        let value = match class {
            Class::Plain => quote!(#name),
            Class::Into => quote!(#name.into()),
            Class::Iter => quote!(crate::utils::IntoCollection::into_collect(#name)),
            Class::Pairs => quote! {
                #name.into_iter().map(|(key, value)| (key.into(), value.into())).collect()
            },
            Class::Filters => quote!(crate::utils::into_filters(#name)),
        };
        quote!(#name: #value)
    });

    let request = &emit.request;
    let literal = quote!(#request { #(#conversions),* });
    let response = &emit.response;
    let error = quote!(crate::client::RequestError);

    let (ret, body) = match (&project, emit.server_stream) {
        (Project::Whole, false) => (
            quote!(#response),
            quote!(self.call(#literal).await),
        ),
        (Project::Field(field), false) => {
            let (resp_parent, resp_stem) = split(&emit.response)?;
            let ty = format_ident!("__armonik_ty_{resp_stem}_{field}");
            (
                quote!(#resp_parent::#ty),
                quote!(Ok(self.call(#literal).await?.#field)),
            )
        }
        (Project::Discard, false) => (quote!(()), {
            quote! {
                self.call(#literal).await?;
                Ok(())
            }
        }),
        (Project::Whole, true) => (
            quote!(::futures::stream::BoxStream<
                'static,
                ::core::result::Result<#response, #error>,
            >),
            quote!(self.call(#literal).await),
        ),
        (Project::Field(field), true) => {
            let (resp_parent, resp_stem) = split(&emit.response)?;
            let ty = format_ident!("__armonik_ty_{resp_stem}_{field}");
            (
                quote!(::futures::stream::BoxStream<
                    'static,
                    ::core::result::Result<#resp_parent::#ty, #error>,
                >),
                quote! {
                    Ok(::futures::StreamExt::boxed(::futures::StreamExt::map(
                        self.call(#literal).await?,
                        |item| item.map(|response| response.#field),
                    )))
                },
            )
        }
        (Project::Discard, true) => {
            return Err(syn::Error::new_spanned(
                &emit.method,
                "`=> ()` is not valid on a server-streaming rpc",
            ))
        }
        (Project::Auto, _) => unreachable!("resolved above"),
    };

    let marker = &emit.marker;
    let method = &emit.method;
    let docs = &emit.docs;
    let deprecated = emit.deprecated.then(|| quote!(#[deprecated]));

    Ok(quote! {
        impl<T: crate::client::Channel> crate::client::ServiceClient<#marker, T> {
            #(#[doc = #docs])*
            #deprecated
            pub async fn #method(
                &mut self,
                #(#params),*
            ) -> ::core::result::Result<#ret, #error> {
                #body
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use quote::quote;

    fn expand(input: proc_macro2::TokenStream) -> String {
        super::expand(input).expect("expands").to_string()
    }

    #[test]
    fn unary_whole_with_all_sugar_classes() {
        let out = expand(quote! {
            marker { Sessions } method { list }
            request { crate::sessions::list::Request } response { crate::sessions::list::Response }
            kind { unary } project { whole } deprecated { false }
            docs { " Get a sessions list." }
            fields { [filters filters] [sort plain] [ids iter] [map pairs] [name into] }
        });
        assert!(out.contains("pub async fn list"), "{out}");
        assert!(out.contains("filters : impl :: core :: iter :: IntoIterator < Item = impl :: core :: iter :: IntoIterator < Item = crate :: sessions :: list :: __armonik_ty_request_filters_elem >"), "{out}");
        assert!(out.contains("sort : crate :: sessions :: list :: __armonik_ty_request_sort"), "{out}");
        assert!(out.contains("name : impl :: core :: convert :: Into < crate :: sessions :: list :: __armonik_ty_request_name >"), "{out}");
        assert!(out.contains("filters : crate :: utils :: into_filters (filters)"), "{out}");
        assert!(out.contains("ids : crate :: utils :: IntoCollection :: into_collect (ids)"), "{out}");
        assert!(out.contains("map : map . into_iter () . map (| (key , value) | (key . into () , value . into ())) . collect ()"), "{out}");
        assert!(out.contains("name : name . into ()"), "{out}");
        assert!(out.contains("self . call (crate :: sessions :: list :: Request {"), "{out}");
        assert!(out.contains("# [doc = \" Get a sessions list.\"]"), "{out}");
        assert!(!out.contains("deprecated"), "{out}");
    }

    #[test]
    fn explicit_projection_returns_the_field_alias() {
        let out = expand(quote! {
            marker { Results } method { get }
            request { crate::results::get::Request } response { crate::results::get::Response }
            kind { unary } project { field result } deprecated { true }
            docs { }
            fields { [id into] }
        });
        assert!(out.contains("Result < crate :: results :: get :: __armonik_ty_response_result , crate :: client :: RequestError >"), "{out}");
        assert!(out.contains("Ok (self . call (crate :: results :: get :: Request { id : id . into () }) . await ? . result)"), "{out}");
        assert!(out.contains("# [deprecated]"), "{out}");
    }

    #[test]
    fn discard_drops_the_response() {
        let out = expand(quote! {
            marker { Submitter } method { cancel }
            request { crate::submitter::cancel::Request } response { crate::submitter::cancel::Response }
            kind { unary } project { discard } deprecated { false }
            docs { }
            fields { [filter plain] }
        });
        assert!(out.contains(". await ? ; Ok (())"), "{out}");
        assert!(out.contains("Result < () , crate :: client :: RequestError >"), "{out}");
    }

    #[test]
    fn auto_without_response_fields_chains_the_response_callback() {
        let out = expand(quote! {
            marker { Sessions } method { get }
            request { crate::sessions::get::Request } response { crate::sessions::get::Response }
            kind { unary } project { auto } deprecated { false }
            docs { }
            fields { [session_id into] }
        });
        assert!(out.contains("crate :: sessions :: get :: __armonik_fields_response !"), "{out}");
        assert!(out.contains("armonik_macros :: __emit_convenience !"), "{out}");
        assert!(!out.contains("pub async fn"), "{out}");
    }

    #[test]
    fn auto_projects_a_single_response_field_and_returns_whole_otherwise() {
        let single = expand(quote! {
            marker { Sessions } method { get }
            request { crate::sessions::get::Request } response { crate::sessions::get::Response }
            kind { unary } project { auto } deprecated { false }
            docs { }
            fields { [session_id into] }
            fields { [session plain] }
        });
        assert!(single.contains(". await ? . session)"), "{single}");

        let multi = expand(quote! {
            marker { Sessions } method { list }
            request { crate::sessions::list::Request } response { crate::sessions::list::Response }
            kind { unary } project { auto } deprecated { false }
            docs { }
            fields { }
            fields { [sessions plain] [total plain] }
        });
        assert!(multi.contains("Result < crate :: sessions :: list :: Response ,"), "{multi}");
    }

    #[test]
    fn server_stream_returns_a_stream_and_maps_projections() {
        let whole = expand(quote! {
            marker { Events } method { subscribe }
            request { crate::events::subscribe::Request } response { crate::events::subscribe::Response }
            kind { server_stream } project { whole } deprecated { false }
            docs { }
            fields { [session_id into] }
        });
        assert!(whole.contains("BoxStream < 'static , :: core :: result :: Result < crate :: events :: subscribe :: Response ,"), "{whole}");

        let projected = expand(quote! {
            marker { Results } method { download }
            request { crate::results::download::Request } response { crate::results::download::Response }
            kind { server_stream } project { field data_chunk } deprecated { false }
            docs { }
            fields { [result_id into] }
        });
        assert!(projected.contains("__armonik_ty_response_data_chunk"), "{projected}");
        assert!(projected.contains("| item | item . map (| response | response . data_chunk)"), "{projected}");
    }

    #[test]
    fn discard_is_rejected_on_server_streams() {
        let err = super::expand(quote! {
            marker { X } method { broken }
            request { crate::x::y::Request } response { crate::x::y::Response }
            kind { server_stream } project { discard } deprecated { false }
            docs { }
            fields { }
        })
        .unwrap_err()
        .to_string();
        assert!(err.contains("server-streaming"), "{err}");
    }
}
