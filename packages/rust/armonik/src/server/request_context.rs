use crate::reexports::http::{Extensions, HeaderMap};

/// Context of the request
#[derive(Debug, Default)]
pub struct RequestContext {
    /// Headers of the request
    headers: HeaderMap,
    /// Extensions of the request
    extensions: Extensions,
}

impl RequestContext {
    /// Create a new context from headers and extensions
    pub fn new(headers: HeaderMap, extensions: Extensions) -> Self {
        Self {
            headers,
            extensions,
        }
    }

    /// Get headers of the request
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }
    /// Get mutable headers of the request
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    /// Get extensions of the request
    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }
    /// Get mutable extensions of the request
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}
