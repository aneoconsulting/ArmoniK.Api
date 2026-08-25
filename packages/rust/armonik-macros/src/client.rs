//! `#[armonik_macros::client]`: the link between a hand-written client method and the RPC it stands
//! for.
//!
//! The client methods are written by hand, in `client/*.rs`. This attribute does the two things a
//! hand-written method cannot do for itself: it prepends the RPC's documentation, harvested from the
//! proto, and it registers the method so the coverage test can prove every RPC has one.
//!
//! It deliberately does *not* touch the signature. That is the point of writing the methods out: a
//! signature that is spelled cannot move when a field is added to the proto message behind it.
//!
//! # Resilience
//!
//! Every failure path re-emits the impl block, so the worst case is a block with no injected docs
//! and no registration, never a block that vanished. rust-analyzer only sees an attributed item
//! through the macro's output: answering a malformed input with `compile_error!` alone takes every
//! method in the block out of the IDE, on every keystroke that leaves it unparseable.
//! `item::rewrite` applies the same rule to a derived type.

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote;

use crate::attrs::{self, ClientAttrs, MethodAttrs};
use crate::descriptor::ServiceMeta;
use crate::matcher::unknown_name;

pub(crate) fn expand(input: TokenStream) -> TokenStream {
    // The item is parsed twice on the failure path, which costs nothing and buys the guarantee
    // above: whatever happens below, something shaped like the input comes back out.
    match syn::parse2::<syn::ItemImpl>(input.clone()) {
        Ok(item) => rewrite(item),
        Err(error) => {
            let error = error.into_compile_error();
            quote! { #input #error }
        }
    }
}

fn rewrite(mut item: syn::ItemImpl) -> TokenStream {
    let mut errors = Vec::new();

    // Taken off the block before anything else can fail, so the helper never survives into the
    // output to become a second, misleading "cannot find attribute" error.
    let service_name = take_service(&mut item.attrs, &mut errors);

    let index = crate::descriptor::index().ok();
    let service = match (&service_name, &index) {
        (Some(name), Some(index)) => match index.services.get(&name.value()) {
            Some(service) => Some(service),
            None => {
                let mut known = index.services.keys().cloned().collect::<Vec<_>>();
                known.sort();
                errors.push(syn::Error::new(
                    name.span(),
                    format!(
                        "no service `{}` in the descriptor; known services: {}",
                        name.value(),
                        known.join(", ")
                    ),
                ));
                None
            }
        },
        _ => None,
    };

    let mut registrations = Vec::new();
    for member in &mut item.items {
        let claimed = match member {
            syn::ImplItem::Fn(method) => claim_of_fn(method, &mut errors),
            syn::ImplItem::Macro(invocation) => claim_of_macro(invocation),
            _ => None,
        };
        let Some(claim) = claimed else { continue };

        let Some(service) = service else { continue };
        let Some(docs) = method_docs(service, &claim, &service_name, &mut errors) else {
            continue;
        };

        match member {
            syn::ImplItem::Fn(method) => {
                for line in docs.iter().rev() {
                    method.attrs.insert(0, syn::parse_quote!(#[doc = #line]));
                }
            }
            // The invocation leads with the RPC name and this prepends an `@docs { ... }` block in
            // front of it, which is the whole contract between the two macros: nothing here parses
            // what comes after the name, and nothing there knows this attribute exists. The `@`
            // keeps the block distinguishable from the RPC name, which is also an ident.
            syn::ImplItem::Macro(invocation) => {
                let tokens = &invocation.mac.tokens;
                invocation.mac.tokens = quote! { @docs { #(#[doc = #docs])* } #tokens };
            }
            _ => {}
        }

        let service_literal = service_name.as_ref().expect("checked above").value();
        let method_literal = claim.method;
        registrations.push(quote! {
            crate::register!(client_method: #service_literal, #method_literal);
        });
    }

    // The block itself, not just its methods: the coverage check needs to know which services this
    // build compiled a client for, and reading that off the methods would let a service that lost
    // all of them leave the check quietly.
    if let Some(service) = &service_name {
        let service_literal = service.value();
        registrations.push(quote! {
            crate::register!(client_service: #service_literal);
        });
    }

    let errors = errors.into_iter().map(syn::Error::into_compile_error);
    quote! {
        #item
        #(#registrations)*
        #(#errors)*
    }
}

/// What one item claims: the RPC name, and where to point an error about it.
struct Claim {
    method: String,
    span: Span,
}

/// Remove the block-level `#[armonik(service = "...")]`, reporting anything else found there.
fn take_service(
    attrs: &mut Vec<syn::Attribute>,
    errors: &mut Vec<syn::Error>,
) -> Option<syn::LitStr> {
    let service = attrs::read::<ClientAttrs>(attrs)
        .map_err(|error| errors.push(error))
        .ok()
        .and_then(|scanned| scanned.service);
    attrs::strip(attrs);

    if service.is_none() {
        errors.push(syn::Error::new(
            Span::call_site(),
            "#[armonik_macros::client] needs #[armonik(service = \"full.proto.Service\")] \
             naming the service its methods belong to",
        ));
    }
    service
}

/// The `#[armonik(rpc = "...")]` on a hand-written method, removed from it.
fn claim_of_fn(method: &mut syn::ImplItemFn, errors: &mut Vec<syn::Error>) -> Option<Claim> {
    let claim = attrs::read::<MethodAttrs>(&method.attrs)
        .map_err(|error| errors.push(error))
        .ok()
        .and_then(|scanned| scanned.rpc)
        .map(|rpc| Claim {
            method: rpc.value(),
            span: rpc.span(),
        });
    attrs::strip(&mut method.attrs);

    if claim.is_none() {
        errors.push(syn::Error::new(
            method.sig.ident.span(),
            "every method of a client impl block needs #[armonik(rpc = \"MethodName\")] \
             naming the RPC it stands for, so the coverage check can see it",
        ));
    }
    claim
}

/// The RPC a `client_method!` invocation leads with. Anything else is left alone: an impl block may
/// hold helper macros that have nothing to do with an RPC.
fn claim_of_macro(invocation: &syn::ImplItemMacro) -> Option<Claim> {
    let last = invocation.mac.path.segments.last()?;
    if last.ident != "client_method" {
        return None;
    }
    // The first ident, past any attributes the invocation carries: `Submitter`'s methods lead with
    // `#[deprecated]`, and the RPC name is what follows it.
    let mut tokens = invocation.mac.tokens.clone().into_iter().peekable();
    while let Some(token) = tokens.peek() {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == '#' => {
                tokens.next();
                if matches!(tokens.peek(), Some(TokenTree::Group(_))) {
                    tokens.next();
                }
            }
            _ => break,
        }
    }
    match tokens.next() {
        Some(TokenTree::Ident(ident)) => Some(Claim {
            method: ident.to_string(),
            span: ident.span(),
        }),
        _ => None,
    }
}

/// The RPC's harvested documentation, or `None` once the error is reported.
fn method_docs(
    service: &ServiceMeta,
    claim: &Claim,
    service_name: &Option<syn::LitStr>,
    errors: &mut Vec<syn::Error>,
) -> Option<Vec<String>> {
    match service
        .methods
        .iter()
        .find(|meta| meta.name == claim.method)
    {
        Some(meta) => Some(meta.docs.clone()),
        None => {
            let available = service
                .methods
                .iter()
                .map(|meta| meta.name.clone())
                .collect();
            errors.push(unknown_name(
                claim.span,
                "method",
                &claim.method,
                &format!(
                    "service `{}`",
                    service_name
                        .as_ref()
                        .map(syn::LitStr::value)
                        .unwrap_or_default()
                ),
                available,
                "check the RPC name against the proto service",
            ));
            None
        }
    }
}
