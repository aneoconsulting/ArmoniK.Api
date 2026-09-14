//! The output being built: the emitted items and the diagnostics, in one place.
//!
//! Every expansion step writes here and nothing returns `Result`: an error is recorded, the step
//! degrades to what it can still say (a poisoned slot, a placeholder body, a skipped name), and
//! generation carries on. The recorded errors become `compile_error!` invocations at the end, so a
//! degraded expansion can never build; the placeholders exist so that it *resolves*, keeping one
//! mistake reading as one error instead of a cascade, in the IDE as on the command line.

use proc_macro2::{Span, TokenStream};

pub(crate) struct Generator {
    /// Top-level items, in emission order.
    stream: TokenStream,
    /// Everything recorded so far, combined; `None` is a clean expansion.
    error: Option<syn::Error>,
}

impl Generator {
    pub(crate) fn new() -> Self {
        Self {
            stream: TokenStream::new(),
            error: None,
        }
    }

    /// Append whole items to the output.
    pub(crate) fn emit(&mut self, tokens: TokenStream) {
        self.stream.extend(tokens);
    }

    /// Record one spanned error. A diagnostic is a span and a message, and the resolvers raise
    /// dozens: this keeps the message the most visible thing at the site.
    pub(crate) fn error(&mut self, span: Span, message: impl std::fmt::Display) {
        self.record(syn::Error::new(span, message));
    }

    pub(crate) fn record(&mut self, error: syn::Error) {
        match &mut self.error {
            Some(combined) => combined.combine(error),
            None => self.error = Some(error),
        }
    }

    /// Whether anything was recorded: the emitter reads this to withhold the registry entry and to
    /// pick placeholder bodies where real ones would misstate a failed expansion.
    pub(crate) fn poisoned(&self) -> bool {
        self.error.is_some()
    }

    /// The final expansion: the (re-emitted) item first, then the emitted items, then the errors
    /// as `compile_error!`s, so a poisoned expansion can never build.
    pub(crate) fn finish(self, item: TokenStream) -> TokenStream {
        let error = self.error.map(|error| error.into_compile_error());
        let stream = self.stream;
        quote::quote! { #item #stream #error }
    }

    /// The emitted items so far, for the token-level tests.
    #[cfg(test)]
    pub(crate) fn stream(&self) -> &TokenStream {
        &self.stream
    }

    /// The combined error, for the tests that assert on diagnostics.
    #[cfg(test)]
    pub(crate) fn into_error(self) -> Option<syn::Error> {
        self.error
    }
}
