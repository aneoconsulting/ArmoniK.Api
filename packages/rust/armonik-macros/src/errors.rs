//! Multi-error accumulation so one expansion reports every problem at once.

pub(crate) struct Errors {
    errors: Vec<syn::Error>,
}

impl Errors {
    pub(crate) fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub(crate) fn push(&mut self, error: syn::Error) {
        self.errors.push(error);
    }

    /// `Ok(())` when no error was recorded, the combined error otherwise.
    pub(crate) fn into_result(self) -> Result<(), Errors> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self)
        }
    }

    pub(crate) fn into_syn_error(self) -> syn::Error {
        let mut errors = self.errors.into_iter();
        let mut combined = errors
            .next()
            .unwrap_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "derive failed"));
        for error in errors {
            combined.combine(error);
        }
        combined
    }
}

impl From<syn::Error> for Errors {
    fn from(error: syn::Error) -> Self {
        Self {
            errors: vec![error],
        }
    }
}
