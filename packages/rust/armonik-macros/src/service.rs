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

    validate(&def, service)?;

    let marker = &def.marker;
    let service_docs = &service.docs;
    let fingerprint = proc_macro2::Literal::u64_suffixed(index.fingerprint);

    let rpcs = def
        .rpcs
        .iter()
        .map(|rpc| {
            let meta = service
                .methods
                .iter()
                .find(|meta| rpc.method == meta.name)
                .expect("validated");
            expand_rpc(&def, rpc, meta, &full_name)
        })
        .collect::<syn::Result<Vec<_>>>()?;

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
    })
}

fn expand_rpc(
    def: &ServiceDef,
    rpc: &RpcDef,
    meta: &MethodMeta,
    full_name: &str,
) -> syn::Result<TokenStream> {
    let marker = &def.marker;
    let module = &def.module;
    let method = rpc.method.to_string();
    let ergonomic = match &rpc.ergonomic {
        Some(ident) => ident.clone(),
        None => request_module_segment(&rpc.request)?,
    };

    let kind = match (rpc.client_stream, rpc.server_stream) {
        (None, None) => quote!(crate::rpc::Unary),
        (None, Some(_)) => quote!(crate::rpc::ServerStream),
        (Some(_), None) => quote!(crate::rpc::ClientStream),
        (Some(stream), Some(_)) => {
            return Err(syn::Error::new(
                stream.span,
                "bidirectional streaming RPCs are not exposed; list the method in `unexposed(...)`",
            ))
        }
    };

    let request = &rpc.request;
    let response = &rpc.response;
    let path = format!("/{full_name}/{method}");
    let label = format!("{marker}::{ergonomic}");
    let docs = &meta.docs;
    let input = &meta.input;
    let output = &meta.output;

    Ok(quote! {
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
    })
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

fn validate(def: &ServiceDef, service: &ServiceMeta) -> syn::Result<()> {
    let known = |ident: &Ident| service.methods.iter().any(|meta| ident == &meta.name);

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

    Ok(())
}
