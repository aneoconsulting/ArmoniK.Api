//! `service!`: RPC definitions for one service, validated against the
//! protobuf descriptor at expansion time.
//!
//! One invocation per service declares every RPC of that service:
//!
//! ```ignore
//! crate::rpc::service! {
//!     Results in crate::results @ "armonik.api.grpc.v1.results.Results";
//!     unexposed(WatchResults);
//!
//!     rpc ListResults(list::Request) -> list::Response;
//!     rpc DownloadResultData(download::Request) -> stream download::Response;
//!     rpc UploadResultData(stream upload::Request) -> upload::Response;
//! }
//! ```
//!
//! The header names the service marker type to emit, the module the request
//! and response paths are relative to, and the full proto service name.
//! `stream` sits where the proto puts it. The ergonomic name (server trait
//! method, telemetry label) is the module segment of the request path, or an
//! explicit `as name` override when several RPCs share a module.
//!
//! Validated against the descriptor, as spanned errors: the service exists,
//! every `rpc` names one of its methods, the `stream` keywords agree with the
//! streaming flags, no method is declared twice, and every method of the
//! service is declared or listed in `unexposed(...)`. What tokens cannot
//! prove — that the named types implement the method's input and output
//! messages — is emitted as const asserts over the codec's `NAMES`.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, LitStr, Path, Token};

use crate::descriptor::{DescriptorIndex, MethodMeta, ServiceMeta};

mod kw {
    syn::custom_keyword!(rpc);
    syn::custom_keyword!(unexposed);
    syn::custom_keyword!(stream);
}

pub(crate) struct ServiceDef {
    marker: Ident,
    module: Path,
    name: LitStr,
    unexposed: Vec<Ident>,
    rpcs: Vec<RpcDef>,
}

struct RpcDef {
    method: Ident,
    client_stream: Option<kw::stream>,
    request: Path,
    server_stream: Option<kw::stream>,
    response: Path,
    ergonomic: Option<Ident>,
}

impl Parse for ServiceDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let marker: Ident = input.parse()?;
        input.parse::<Token![in]>()?;
        let module: Path = input.parse()?;
        input.parse::<Token![@]>()?;
        let name: LitStr = input.parse()?;
        input.parse::<Token![;]>()?;

        let mut unexposed = Vec::new();
        if input.peek(kw::unexposed) {
            input.parse::<kw::unexposed>()?;
            let content;
            syn::parenthesized!(content in input);
            unexposed.extend(Punctuated::<Ident, Token![,]>::parse_terminated(&content)?);
            input.parse::<Token![;]>()?;
        }

        let mut rpcs = Vec::new();
        while !input.is_empty() {
            input.parse::<kw::rpc>()?;
            let method: Ident = input.parse()?;
            let args;
            syn::parenthesized!(args in input);
            let client_stream = parse_stream(&args)?;
            let request: Path = args.parse()?;
            if !args.is_empty() {
                return Err(args.error("expected a single request type"));
            }
            input.parse::<Token![->]>()?;
            let server_stream = parse_stream(input)?;
            let response: Path = input.parse()?;
            let ergonomic = if input.peek(Token![as]) {
                input.parse::<Token![as]>()?;
                Some(input.parse::<Ident>()?)
            } else {
                None
            };
            input.parse::<Token![;]>()?;
            rpcs.push(RpcDef {
                method,
                client_stream,
                request,
                server_stream,
                response,
                ergonomic,
            });
        }

        Ok(ServiceDef {
            marker,
            module,
            name,
            unexposed,
            rpcs,
        })
    }
}

fn parse_stream(input: ParseStream) -> syn::Result<Option<kw::stream>> {
    if input.peek(kw::stream) {
        Ok(Some(input.parse()?))
    } else {
        Ok(None)
    }
}

/// The three call shapes; drives kind markers, trait signatures and the
/// `serve_*` helper each route dispatches into.
#[derive(Clone, Copy, PartialEq)]
enum CallKind {
    Unary,
    ClientStream,
    ServerStream,
}

/// One `rpc` line, resolved against the descriptor.
struct Resolved<'a> {
    rpc: &'a RpcDef,
    meta: &'a MethodMeta,
    ergonomic: Ident,
    kind: CallKind,
}

pub(crate) fn expand(def: ServiceDef, index: &DescriptorIndex) -> syn::Result<TokenStream> {
    let full_name = def.name.value();
    let service = index.services.get(&full_name).ok_or_else(|| {
        let mut known = index.services.keys().cloned().collect::<Vec<_>>();
        known.sort();
        syn::Error::new(
            def.name.span(),
            format!(
                "no service `{full_name}` in the descriptor; known services: {}",
                known.join(", ")
            ),
        )
    })?;

    let resolved = validate(&def, service)?;

    let marker = &def.marker;
    let service_docs = &service.docs;
    let fingerprint = proc_macro2::Literal::u64_suffixed(index.fingerprint);

    let rpcs = resolved
        .iter()
        .map(|entry| expand_rpc(&def, entry, &full_name))
        .collect::<Vec<_>>();
    let server = expand_server(&def, &resolved, service_docs);

    Ok(quote! {
        #(#[doc = #service_docs])*
        pub struct #marker;

        impl crate::rpc::Service for #marker {
            const NAME: &'static str = #full_name;
        }

        const _: () = assert!(
            crate::__schema::DESCRIPTOR_FINGERPRINT == #fingerprint,
            "armonik: `service!` was expanded against a stale protobuf descriptor; \
             rebuild the crate"
        );

        #(#rpcs)*

        #server
    })
}

fn expand_rpc(def: &ServiceDef, entry: &Resolved<'_>, full_name: &str) -> TokenStream {
    let marker = &def.marker;
    let module = &def.module;
    let method = entry.rpc.method.to_string();
    let ergonomic = &entry.ergonomic;

    let kind = match entry.kind {
        CallKind::Unary => quote!(crate::rpc::Unary),
        CallKind::ServerStream => quote!(crate::rpc::ServerStream),
        CallKind::ClientStream => quote!(crate::rpc::ClientStream),
    };

    let request = &entry.rpc.request;
    let response = &entry.rpc.response;
    let path = format!("/{full_name}/{method}");
    let label = format!("{marker}::{ergonomic}");
    let docs = &entry.meta.docs;
    let input = &entry.meta.input;
    let output = &entry.meta.output;

    quote! {
        #(#[doc = #docs])*
        impl crate::rpc::Rpc for #module::#request {
            type Service = #marker;
            type Kind = #kind;
            type Response = #module::#response;

            const METHOD: &'static str = #method;
            const PATH: &'static str = #path;
            const LABEL: &'static str = #label;
        }

        const _: () = {
            assert!(
                crate::codec::names_contain(
                    <#module::#request as crate::codec::Msg>::NAMES,
                    #input,
                ),
                "the request type does not implement this RPC's input message",
            );
            assert!(
                crate::codec::names_contain(
                    <#module::#response as crate::codec::Msg>::NAMES,
                    #output,
                ),
                "the response type does not implement this RPC's output message",
            );
        };
    }
}

/// The server side of one invocation: the service trait (harvested docs,
/// streaming shapes from the descriptor), the one-line `Ext`, and the routing
/// table the generic `Router` dispatches through.
fn expand_server(
    def: &ServiceDef,
    resolved: &[Resolved<'_>],
    service_docs: &[String],
) -> TokenStream {
    let marker = &def.marker;
    let module = &def.module;
    let trait_ident = quote::format_ident!("{}Service", marker);
    let ext_ident = quote::format_ident!("{}ServiceExt", marker);
    let server_fn = quote::format_ident!("{}_server", snake(&marker.to_string()));

    let methods = resolved.iter().map(|entry| {
        let ergonomic = &entry.ergonomic;
        let docs = &entry.meta.docs;
        let request = &entry.rpc.request;
        let response = &entry.rpc.response;
        match entry.kind {
            CallKind::Unary => quote! {
                #(#[doc = #docs])*
                fn #ergonomic(
                    self: ::std::sync::Arc<Self>,
                    request: #module::#request,
                    context: crate::server::RequestContext,
                ) -> impl ::std::future::Future<
                    Output = ::std::result::Result<#module::#response, ::tonic::Status>,
                > + ::std::marker::Send;
            },
            CallKind::ServerStream => quote! {
                #(#[doc = #docs])*
                fn #ergonomic(
                    self: ::std::sync::Arc<Self>,
                    request: #module::#request,
                    context: crate::server::RequestContext,
                ) -> impl ::std::future::Future<
                    Output = ::std::result::Result<
                        impl ::futures::Stream<
                            Item = ::std::result::Result<#module::#response, ::tonic::Status>,
                        > + ::std::marker::Send,
                        ::tonic::Status,
                    >,
                > + ::std::marker::Send;
            },
            CallKind::ClientStream => quote! {
                #(#[doc = #docs])*
                fn #ergonomic(
                    self: ::std::sync::Arc<Self>,
                    request: impl ::futures::Stream<
                        Item = ::std::result::Result<#module::#request, ::tonic::Status>,
                    > + ::std::marker::Send + 'static,
                    context: crate::server::RequestContext,
                ) -> impl ::std::future::Future<
                    Output = ::std::result::Result<#module::#response, ::tonic::Status>,
                > + ::std::marker::Send;
            },
        }
    });

    let routes = resolved.iter().map(|entry| {
        let ergonomic = &entry.ergonomic;
        let request = &entry.rpc.request;
        let serve = match entry.kind {
            CallKind::Unary => quote!(serve_unary),
            CallKind::ServerStream => quote!(serve_server_stream),
            CallKind::ClientStream => quote!(serve_client_stream),
        };
        let span = format!("{trait_ident}::{ergonomic}");
        quote! {
            (
                <#module::#request as crate::rpc::Rpc>::PATH,
                |svc, req, config| {
                    ::std::boxed::Box::pin(crate::server::router::#serve(
                        svc,
                        req,
                        config,
                        |s: ::std::sync::Arc<S>, r, c| <S as #trait_ident>::#ergonomic(s, r, c),
                        ::tracing::debug_span!(#span),
                    ))
                },
            )
        }
    });

    let ext_doc = format!("Serve a [`{trait_ident}`] implementation as a gRPC service.");
    let server_fn_doc = "Wrap the service implementation into a \
                         [`Router`](crate::server::Router) accepted by \
                         `tonic::transport::Server::add_service`.";

    quote! {
        #[cfg(feature = "_gen-server")]
        #(#[doc = #service_docs])*
        pub trait #trait_ident {
            #(#methods)*
        }

        #[cfg(feature = "_gen-server")]
        #[doc = #ext_doc]
        pub trait #ext_ident {
            #[doc = #server_fn_doc]
            fn #server_fn(self) -> crate::server::Router<#marker, Self>
            where
                Self: Sized;
        }

        #[cfg(feature = "_gen-server")]
        impl<S> #ext_ident for S
        where
            S: #trait_ident + ::std::marker::Send + ::std::marker::Sync + 'static,
        {
            fn #server_fn(self) -> crate::server::Router<#marker, Self> {
                crate::server::Router::new(self)
            }
        }

        #[cfg(feature = "_gen-server")]
        impl<S> crate::server::router::Routes<S> for #marker
        where
            S: #trait_ident + ::std::marker::Send + ::std::marker::Sync + 'static,
        {
            const ROUTES: &'static [(&'static str, crate::server::router::RouteFn<S>)] = &[
                #(#routes),*
            ];
        }
    }
}

/// `HealthChecks` -> `health_checks`.
fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// The module segment of `list::Request`, the default ergonomic name.
fn request_module_segment(request: &Path) -> syn::Result<Ident> {
    if request.segments.len() < 2 {
        return Err(syn::Error::new_spanned(
            request,
            "the request path has no module segment to derive the method name from; \
             add an `as name` override",
        ));
    }
    Ok(request.segments[request.segments.len() - 2].ident.clone())
}

fn validate<'a>(def: &'a ServiceDef, service: &'a ServiceMeta) -> syn::Result<Vec<Resolved<'a>>> {
    let known = |ident: &Ident| service.methods.iter().any(|meta| ident == &meta.name);

    let mut resolved = Vec::new();
    let mut declared = HashSet::new();
    let mut ergonomics = HashSet::new();
    for rpc in &def.rpcs {
        let meta = service
            .methods
            .iter()
            .find(|meta| rpc.method == meta.name)
            .ok_or_else(|| {
                syn::Error::new(
                    rpc.method.span(),
                    format!("service `{}` has no method `{}`", def.name.value(), rpc.method),
                )
            })?;
        if !declared.insert(meta.name.clone()) {
            return Err(syn::Error::new(
                rpc.method.span(),
                format!("method `{}` is declared twice", rpc.method),
            ));
        }

        if meta.client_streaming != rpc.client_stream.is_some() {
            return Err(syn::Error::new(
                rpc.client_stream.map_or_else(|| rpc.method.span(), |kw| kw.span),
                format!(
                    "`{}` {} client-streaming in the proto",
                    rpc.method,
                    if meta.client_streaming { "is" } else { "is not" },
                ),
            ));
        }
        if meta.server_streaming != rpc.server_stream.is_some() {
            return Err(syn::Error::new(
                rpc.server_stream.map_or_else(|| rpc.method.span(), |kw| kw.span),
                format!(
                    "`{}` {} server-streaming in the proto",
                    rpc.method,
                    if meta.server_streaming { "is" } else { "is not" },
                ),
            ));
        }

        let kind = match (rpc.client_stream, rpc.server_stream) {
            (None, None) => CallKind::Unary,
            (None, Some(_)) => CallKind::ServerStream,
            (Some(_), None) => CallKind::ClientStream,
            (Some(stream), Some(_)) => {
                return Err(syn::Error::new(
                    stream.span,
                    "bidirectional streaming RPCs are not exposed; list the method in \
                     `unexposed(...)`",
                ))
            }
        };

        let ergonomic = match &rpc.ergonomic {
            Some(ident) => ident.clone(),
            None => request_module_segment(&rpc.request)?,
        };
        if !ergonomics.insert(ergonomic.to_string()) {
            return Err(syn::Error::new(
                ergonomic.span(),
                format!(
                    "two RPCs would both be named `{ergonomic}`; disambiguate with `as name`"
                ),
            ));
        }

        resolved.push(Resolved {
            rpc,
            meta,
            ergonomic,
            kind,
        });
    }

    for unexposed in &def.unexposed {
        if !known(unexposed) {
            return Err(syn::Error::new(
                unexposed.span(),
                format!(
                    "service `{}` has no method `{unexposed}`",
                    def.name.value()
                ),
            ));
        }
        if declared.contains(&unexposed.to_string()) {
            return Err(syn::Error::new(
                unexposed.span(),
                format!("`{unexposed}` is declared as an rpc and cannot also be unexposed"),
            ));
        }
    }

    let missing = service
        .methods
        .iter()
        .filter(|meta| {
            !declared.contains(&meta.name) && def.unexposed.iter().all(|ident| ident != &meta.name)
        })
        .map(|meta| meta.name.as_str())
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(syn::Error::new(
            def.marker.span(),
            format!(
                "service `{}` has undeclared methods: {}; add `rpc` lines or list them in \
                 `unexposed(...)`",
                def.name.value(),
                missing.join(", ")
            ),
        ));
    }

    Ok(resolved)
}
