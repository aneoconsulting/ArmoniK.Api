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
//! prove, that the named types implement the method's input and output
//! messages, is emitted as const asserts over the codec's `NAMES`.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Ident, LitStr, Path, Token};

use crate::descriptor::{MethodMeta, ServiceMeta};

mod kw {
    syn::custom_keyword!(rpc);
    syn::custom_keyword!(unexposed);
    syn::custom_keyword!(stream);
    syn::custom_keyword!(manual);
    syn::custom_keyword!(deprecated);
}

pub(crate) struct ServiceDef {
    marker: Ident,
    module: Path,
    name: LitStr,
    unexposed: Vec<Ident>,
    deprecated: bool,
    rpcs: Vec<RpcDef>,
}

struct RpcDef {
    method: Ident,
    client_stream: Option<kw::stream>,
    request: Path,
    server_stream: Option<kw::stream>,
    response: Path,
    ergonomic: Option<Ident>,
    project: Project,
    manual: bool,
}

/// What the generated convenience method returns, as the rpc line spells it: the whole response
/// when the line says nothing, one projected field after `=> field`, or nothing after `=> ()`.
///
/// There used to be a fifth reading, `auto`, under which a bare line meant "project the single
/// field, or return the whole response if there are several". That made 30 methods' return type a
/// function of a proto message's field count: adding a second field to any of them silently
/// changed the public API. It also cost the emitter a second pass through the response type's
/// reflection callback, which was the only reason a response needed one.
enum Project {
    Whole,
    Discard,
    Field(Ident),
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
        let deprecated = input.parse::<Option<kw::deprecated>>()?.is_some();
        if deprecated {
            input.parse::<Token![;]>()?;
        }

        let mut rpcs = Vec::new();
        while !input.is_empty() {
            input.parse::<kw::rpc>()?;
            let method: Ident = input.parse()?;
            let args;
            syn::parenthesized!(args in input);
            let client_stream = args.parse()?;
            let request: Path = args.parse()?;
            if !args.is_empty() {
                return Err(args.error("expected a single request type"));
            }
            input.parse::<Token![->]>()?;
            let server_stream = input.parse()?;
            let response: Path = input.parse()?;
            let ergonomic = match input.parse::<Option<Token![as]>>()? {
                Some(_) => Some(input.parse::<Ident>()?),
                None => None,
            };
            let project = if input.peek(Token![=>]) {
                input.parse::<Token![=>]>()?;
                if input.peek(syn::token::Paren) {
                    let unit;
                    syn::parenthesized!(unit in input);
                    if !unit.is_empty() {
                        return Err(unit.error("expected `()` (discard the response)"));
                    }
                    Project::Discard
                } else {
                    Project::Field(input.parse()?)
                }
            } else {
                Project::Whole
            };
            let manual = input.parse::<Option<kw::manual>>()?.is_some();
            input.parse::<Token![;]>()?;
            rpcs.push(RpcDef {
                method,
                client_stream,
                request,
                server_stream,
                response,
                ergonomic,
                project,
                manual,
            });
        }

        Ok(ServiceDef {
            marker,
            module,
            name,
            unexposed,
            deprecated,
            rpcs,
        })
    }
}

/// The three call shapes; drives kind markers, trait signatures and the `serve_*` helper each route
/// dispatches into.
#[derive(Clone, Copy, PartialEq)]
enum CallKind {
    Unary,
    ClientStream,
    ServerStream,
}

/// Everything that varies with the call shape, in one place.
///
/// These four facts used to sit in four matches in three functions: the `Rpc::Kind` marker in
/// [`expand_rpc`], the sugar tag in [`expand_convenience`], and the signature shape and the
/// `serve_*` helper in [`expand_server`]. Nothing tied them together, so a fourth call shape would
/// be four edits and four chances to miss one. Here an arm is one shape, stated once.
struct KindFacts {
    /// The `crate::rpc::*` marker the `Rpc` impl names.
    marker: TokenStream,
    /// The `server::router::serve_*` the route dispatches into.
    serve: Ident,
    /// The tag `__emit_convenience` reads. `None` for a client-streaming rpc, which emits no
    /// convenience method at all: it is required to carry `manual` (see [`validate`]).
    sugar: Option<TokenStream>,
    /// Where a `stream` keyword sits, which is what the trait signature reads: a streaming side
    /// takes or returns `impl Stream` instead of the message.
    client_streams: bool,
    server_streams: bool,
}

impl CallKind {
    fn facts(self) -> KindFacts {
        match self {
            CallKind::Unary => KindFacts {
                marker: quote!(crate::rpc::Unary),
                serve: format_ident!("serve_unary"),
                sugar: Some(quote!(unary)),
                client_streams: false,
                server_streams: false,
            },
            CallKind::ServerStream => KindFacts {
                marker: quote!(crate::rpc::ServerStream),
                serve: format_ident!("serve_server_stream"),
                sugar: Some(quote!(server_stream)),
                client_streams: false,
                server_streams: true,
            },
            CallKind::ClientStream => KindFacts {
                marker: quote!(crate::rpc::ClientStream),
                serve: format_ident!("serve_client_stream"),
                sugar: None,
                client_streams: true,
                server_streams: false,
            },
        }
    }
}

/// One `rpc` line, resolved against the descriptor.
struct Resolved<'a> {
    rpc: &'a RpcDef,
    meta: &'a MethodMeta,
    ergonomic: Ident,
    kind: CallKind,
}

/// Resolve one `service!` invocation against the descriptor and emit it.
///
/// The index is loaded here rather than by the entry point, so a descriptor that fails to load reads
/// as the reason this service could not be resolved, and `service` stays a single call. The two
/// attribute macros own their index the same way, through `shape::resolve_*`.
pub(crate) fn expand(def: ServiceDef) -> syn::Result<TokenStream> {
    let index = crate::descriptor::index()
        .map_err(|message| syn::Error::new(proc_macro2::Span::call_site(), message))?;
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

    let Validated {
        rpcs: resolved,
        unexposed_messages,
    } = validate(&def, service)?;

    let marker = &def.marker;
    let service_docs = &service.docs;
    let fingerprint = proc_macro2::Literal::u64_suffixed(index.fingerprint);

    let rpcs = resolved
        .iter()
        .map(|entry| expand_rpc(&def, entry, &full_name))
        .collect::<Vec<_>>();
    let server = expand_server(&def, &resolved, service_docs);
    let conveniences = resolved
        .iter()
        .map(|entry| expand_convenience(&def, entry))
        .collect::<syn::Result<Vec<_>>>()?;

    // The unexposed RPCs' messages have no Rust type; register them for the differential harness's
    // coverage ratchet, so the message allowlist is derived from the same declaration as the RPC
    // one. Resolved by `validate`, which is the pass that already had to find them.
    let unexposed = (!unexposed_messages.is_empty()).then(|| {
        quote! {
            crate::register!(unexposed: #(#unexposed_messages),*);
        }
    });

    // The client alias, from the same declaration as the marker and with the same harvested docs.
    // `client/<svc>.rs` used to spell both by hand, and two of the twelve transcriptions had already
    // drifted from the proto prose they were copied from.
    let deprecation = def.deprecated.then(|| quote!(#[deprecated]));
    let alias = quote! {
        #[cfg(feature = "_gen-client")]
        #(#[doc = #service_docs])*
        #deprecation
        pub type Client<T = ::tonic::transport::Channel> =
            crate::client::ServiceClient<#marker, T>;
    };

    Ok(quote! {
        #(#[doc = #service_docs])*
        pub struct #marker;

        impl crate::rpc::Service for #marker {
            const NAME: &'static str = #full_name;
        }

        #alias

        const _: () = assert!(
            crate::__schema::DESCRIPTOR_FINGERPRINT == #fingerprint,
            "armonik: `service!` was expanded against a stale protobuf descriptor; \
             rebuild the crate"
        );

        #(#rpcs)*

        #unexposed

        #(#conveniences)*

        #server
    })
}

/// The client convenience method of one RPC: an invocation of the request type's field-reflection
/// callback, continued into `__emit_convenience`, which builds the method from the fields (see
/// `convenience.rs`). `manual` lines emit nothing: the opt-out for custom wiring or a wrong sugar
/// default. Client-streaming lines are required to carry it (enforced in `validate`), so this one
/// condition covers them too.
fn expand_convenience(def: &ServiceDef, entry: &Resolved<'_>) -> syn::Result<TokenStream> {
    if entry.rpc.manual {
        return Ok(TokenStream::new());
    }

    let module = &def.module;
    let marker = &def.marker;
    let method = &entry.ergonomic;
    let docs = &entry.meta.docs;
    let deprecated = def.deprecated;

    let request = &entry.rpc.request;
    let response = &entry.rpc.response;
    if request.segments.len() < 2 {
        return Err(syn::Error::new_spanned(
            request,
            "convenience emission needs a `module::Type` request path; use `manual` to opt out",
        ));
    }
    let parents = request.segments.iter().take(request.segments.len() - 1);
    let req_parent = quote!(#(#parents)::*);
    let req_stem =
        crate::names::snake(&request.segments.last().expect("checked").ident.to_string());
    let callback = quote::format_ident!("__armonik_fields_{req_stem}");

    let kind = entry
        .kind
        .facts()
        .sugar
        .expect("a client-streaming rpc carries `manual`, filtered above");
    let project = match &entry.rpc.project {
        Project::Whole => quote!(whole),
        Project::Discard => quote!(discard),
        Project::Field(field) => quote!(field #field),
    };

    Ok(quote! {
        #[cfg(feature = "_gen-client")]
        #module::#req_parent::#callback! { armonik_macros::__emit_convenience! {
            marker { #marker }
            method { #method }
            request { #module::#request }
            response { #module::#response }
            kind { #kind }
            project { #project }
            deprecated { #deprecated }
            docs { #(#docs)* }
        } }
    })
}

fn expand_rpc(def: &ServiceDef, entry: &Resolved<'_>, full_name: &str) -> TokenStream {
    let marker = &def.marker;
    let module = &def.module;
    let method = entry.rpc.method.to_string();
    let ergonomic = &entry.ergonomic;

    let kind = entry.kind.facts().marker;

    let request = &entry.rpc.request;
    let response = &entry.rpc.response;
    let path = format!("/{full_name}/{method}");
    let label = format!("{marker}::{ergonomic}");
    let docs = &entry.meta.docs;
    let input = &entry.meta.input;
    let output = &entry.meta.output;

    // Spanned onto the rpc line's own type paths rather than onto the invocation: a `service!` body
    // holds up to 15 rpc lines, and an assert spanned at the invocation says only that one of them
    // names the wrong type.
    let request_assert = quote_spanned! { request.span() =>
        const _: () = crate::codec::assert_request_message::<#module::#request>(#input);
    };
    let response_assert = quote_spanned! { response.span() =>
        const _: () = crate::codec::assert_response_message::<#module::#response>(#output);
    };

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

        #request_assert
        #response_assert
    }
}

/// The server side of one invocation: the service trait (harvested docs, streaming shapes from the
/// descriptor), the one-line `Ext`, and the routing table the generic `Router` dispatches through.
fn expand_server(
    def: &ServiceDef,
    resolved: &[Resolved<'_>],
    service_docs: &[String],
) -> TokenStream {
    let marker = &def.marker;
    let module = &def.module;
    let trait_ident = quote::format_ident!("{}Service", marker);
    let ext_ident = quote::format_ident!("{}ServiceExt", marker);
    let server_fn = quote::format_ident!("{}_server", crate::names::snake(&marker.to_string()));

    // One signature, with the two positions a `stream` keyword can sit in as the only variables:
    // the parameter type and the `Future`'s output. Spelling all three out in full `::std::`-
    // qualified form meant the shared two thirds -- the receiver, the context parameter, the
    // `Future + Send` return -- were written three times and had to be kept identical by hand.
    let methods = resolved.iter().map(|entry| {
        let ergonomic = &entry.ergonomic;
        let docs = &entry.meta.docs;
        let request = &entry.rpc.request;
        let response = &entry.rpc.response;
        let stream_of = |item: TokenStream| {
            quote! {
                impl ::futures::Stream<
                    Item = ::std::result::Result<#item, ::tonic::Status>,
                > + ::std::marker::Send
            }
        };
        // `stream` sits where the proto puts it, so each side of the signature reads its own flag
        // rather than the three shapes being enumerated together.
        let facts = entry.kind.facts();
        let parameter = if facts.client_streams {
            let stream = stream_of(quote!(#module::#request));
            quote!(#stream + 'static)
        } else {
            quote!(#module::#request)
        };
        let output = if facts.server_streams {
            stream_of(quote!(#module::#response))
        } else {
            quote!(#module::#response)
        };
        quote! {
            #(#[doc = #docs])*
            fn #ergonomic(
                self: ::std::sync::Arc<Self>,
                request: #parameter,
                context: crate::server::RequestContext,
            ) -> impl ::std::future::Future<
                Output = ::std::result::Result<#output, ::tonic::Status>,
            > + ::std::marker::Send;
        }
    });

    let routes = resolved.iter().map(|entry| {
        let ergonomic = &entry.ergonomic;
        let request = &entry.rpc.request;
        let serve = entry.kind.facts().serve;
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
        impl<S, B> crate::server::router::Routes<S, B> for #marker
        where
            S: #trait_ident + ::std::marker::Send + ::std::marker::Sync + 'static,
            B: ::tonic::codegen::Body<Data = ::prost::bytes::Bytes>
                + ::std::marker::Send
                + 'static,
            B::Error: ::std::convert::Into<::tonic::codegen::StdError>
                + ::std::marker::Send
                + 'static,
        {
            const ROUTES: &'static [(&'static str, crate::server::router::RouteFn<S, B>)] = &[
                #(#routes),*
            ];
        }
    }
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

/// What [`validate`] resolved: one entry per `rpc` line, plus what the `unexposed(...)` names
/// resolved to.
///
/// The messages are carried out rather than looked up again. `expand` used to re-find each
/// unexposed method in the descriptor and assert the invariant with `.expect("validated")`, which is
/// the same lookup twice with a panic holding the two halves together.
struct Validated<'a> {
    rpcs: Vec<Resolved<'a>>,
    /// Input and output messages of the unexposed RPCs. No Rust type stands for them, so the
    /// coverage ratchet is told about them from this same declaration.
    unexposed_messages: Vec<String>,
}

fn validate<'a>(def: &'a ServiceDef, service: &'a ServiceMeta) -> syn::Result<Validated<'a>> {
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
                    format!(
                        "service `{}` has no method `{}`",
                        def.name.value(),
                        rpc.method
                    ),
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
                rpc.client_stream
                    .map_or_else(|| rpc.method.span(), |kw| kw.span),
                format!(
                    "`{}` {} client-streaming in the proto",
                    rpc.method,
                    if meta.client_streaming {
                        "is"
                    } else {
                        "is not"
                    },
                ),
            ));
        }
        if meta.server_streaming != rpc.server_stream.is_some() {
            return Err(syn::Error::new(
                rpc.server_stream
                    .map_or_else(|| rpc.method.span(), |kw| kw.span),
                format!(
                    "`{}` {} server-streaming in the proto",
                    rpc.method,
                    if meta.server_streaming {
                        "is"
                    } else {
                        "is not"
                    },
                ),
            ));
        }

        // After the descriptor checks above, not while parsing: `stream` on a method the proto
        // does not stream is one mistake, and asking for `manual` first would answer a question the
        // author never meant to ask.
        //
        // A request stream has no single message to spread into parameters, so the derivation this
        // macro performs -- one parameter per request field -- has nothing to work from. (The
        // *entry point* is no longer the reason: `call` takes all three kinds since the `IntoCall`
        // unification. A `stream in, response out` method would be derivable now, but all three
        // client-streaming RPCs need a hand-written body anyway -- two turn a oneof response into
        // an error, the third builds the request stream -- so deriving it would be an emission path
        // with no consumer.) Spelling `manual` out keeps "no convenience method" readable from the
        // rpc line alone.
        if let (Some(stream), false) = (&rpc.client_stream, rpc.manual) {
            return Err(syn::Error::new(
                stream.span,
                "a client-streaming rpc needs `manual`: its request is a stream, so there is \
                 no message to spread into a convenience method's parameters",
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
                format!("two RPCs would both be named `{ergonomic}`; disambiguate with `as name`"),
            ));
        }

        resolved.push(Resolved {
            rpc,
            meta,
            ergonomic,
            kind,
        });
    }

    let mut unexposed_messages = Vec::new();
    for unexposed in &def.unexposed {
        let Some(meta) = service.methods.iter().find(|meta| unexposed == &meta.name) else {
            return Err(syn::Error::new(
                unexposed.span(),
                format!("service `{}` has no method `{unexposed}`", def.name.value()),
            ));
        };
        if declared.contains(&unexposed.to_string()) {
            return Err(syn::Error::new(
                unexposed.span(),
                format!("`{unexposed}` is declared as an rpc and cannot also be unexposed"),
            ));
        }
        unexposed_messages.push(meta.input.clone());
        unexposed_messages.push(meta.output.clone());
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

    Ok(Validated {
        rpcs: resolved,
        unexposed_messages,
    })
}
