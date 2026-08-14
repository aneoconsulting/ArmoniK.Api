//! Parsing of `#[armonik(...)]` helper attributes.
//!
//! Parsed by hand rather than through `parse_nested_meta`, because `enum` is a Rust keyword and
//! must still be accepted as a key. Also hosts the multi-error accumulator ([`Errors`]) the
//! resolvers fill. The user-facing grammar documentation lives on the two macros in `lib.rs`; keep
//! it in sync.

use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, LitInt, LitStr, Token};

/// A single `key` or `key = value` entry inside `#[armonik(...)]`.
pub(crate) enum AttrItem {
    /// `message = "full.proto.Name"`: proto message backing the type (repeatable for unified types)
    /// or, on an enum with `transparent`, the single-field wrapper messages.
    Message(LitStr),
    /// `enum = "full.proto.Name"`: proto enum backing the type.
    Enum(LitStr),
    /// `oneof = "name"`: the type is the flattened oneof of that name.
    Oneof(LitStr),
    /// `generic`: no descriptor validation; fields carry explicit `tag`s.
    Generic,
    /// `transparent`: single-field wrapper message flattened into its field.
    Transparent,
    /// `rename = "proto_name"`: the proto field or value name differs from the Rust name.
    Rename(LitStr),
    /// `tag = N`: explicit field tag, cross-checked against the descriptor except in `generic`
    /// mode, where it is authoritative.
    Tag(LitInt),
    /// `with = "path::to::Adapter"`: custom codec for a non-standard representation.
    With(LitStr),
    /// `present`: marker oneof variant selected by field presence.
    Present,
    /// `inline`: the struct variant's leftover fields are the member message's own fields, spread
    /// into the variant rather than carried whole by one of them.
    Inline,
    /// `absorbs = "full.proto.Name"`, on a field or variant carrying a `with` adapter: the proto
    /// message the adapter flattens away, which therefore has no Rust type of its own. Harvested so
    /// the build script prunes it and the differential harness counts it as covered.
    Absorbs(LitStr),
    /// `service = "full.proto.Service"`, on a client impl block: the proto service its methods
    /// belong to, which is what `#[armonik_macros::client]` looks their documentation up in.
    Service(LitStr),
    /// `rpc = "MethodName"`, on a client method: the RPC it stands for.
    Rpc(LitStr),
}

pub(crate) struct AttrEntry {
    pub(crate) span: Span,
    pub(crate) item: AttrItem,
}

/// Parse the `= <value>` tail shared by every `key = value` entry; the value type
/// (`LitStr`/`LitInt`) is inferred from the `AttrItem` constructor.
fn eq_then<T: Parse>(input: ParseStream) -> syn::Result<T> {
    input.parse::<Token![=]>()?;
    input.parse()
}

struct AttrList(Vec<AttrEntry>);

impl Parse for AttrList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut entries = Vec::new();
        while !input.is_empty() {
            let (span, key) = if input.peek(Token![enum]) {
                let token: Token![enum] = input.parse()?;
                (token.span, "enum".to_owned())
            } else {
                let ident: syn::Ident = input.parse()?;
                (ident.span(), ident.to_string())
            };

            let item = match key.as_str() {
                "message" => AttrItem::Message(eq_then(input)?),
                "enum" => AttrItem::Enum(eq_then(input)?),
                "oneof" => AttrItem::Oneof(eq_then(input)?),
                "rename" => AttrItem::Rename(eq_then(input)?),
                "tag" => AttrItem::Tag(eq_then(input)?),
                "with" => AttrItem::With(eq_then(input)?),
                "absorbs" => AttrItem::Absorbs(eq_then(input)?),
                "service" => AttrItem::Service(eq_then(input)?),
                "rpc" => AttrItem::Rpc(eq_then(input)?),
                "generic" => AttrItem::Generic,
                "transparent" => AttrItem::Transparent,
                "present" => AttrItem::Present,
                "inline" => AttrItem::Inline,
                other => {
                    return Err(syn::Error::new(
                        span,
                        format!(
                            "unknown armonik attribute key `{other}` (expected one of: \
                             message, enum, oneof, rename, tag, with, absorbs, service, rpc, \
                             generic, transparent, present, inline)"
                        ),
                    ));
                }
            };
            entries.push(AttrEntry { span, item });

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(AttrList(entries))
    }
}

/// Collect every entry of every `#[armonik(...)]` attribute in `attrs`.
pub(crate) fn parse(attrs: &[Attribute]) -> syn::Result<Vec<AttrEntry>> {
    let mut entries = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("armonik") {
            let list: AttrList = attr.parse_args()?;
            entries.extend(list.0);
        }
    }
    Ok(entries)
}

/// Visit the attributes of the type itself and of every field, variant and variant field: the
/// common traversal for whole-input attribute scans.
pub(crate) fn for_each_site(input: &syn::DeriveInput, mut visit: impl FnMut(&[Attribute])) {
    visit(&input.attrs);
    match &input.data {
        syn::Data::Struct(data) => {
            for field in &data.fields {
                visit(&field.attrs);
            }
        }
        syn::Data::Enum(data) => {
            for variant in &data.variants {
                visit(&variant.attrs);
                for field in &variant.fields {
                    visit(&field.attrs);
                }
            }
        }
        syn::Data::Union(_) => {}
    }
}

/// Span of every key token of every `#[armonik(...)]` attribute in `attrs`, for the
/// hover-documentation anchors (see `item::anchors`). Malformed attributes are skipped; the real
/// parse reports them.
pub(crate) fn key_spans(attrs: &[Attribute]) -> Vec<Span> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("armonik"))
        .filter_map(|attr| attr.parse_args::<AttrList>().ok())
        .flat_map(|list| list.0)
        .map(|entry| entry.span)
        .collect()
}

/// Multi-error accumulation so one expansion reports every problem at once.
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

    /// Record one spanned error.
    ///
    /// A diagnostic is a span and a message; spelling out `push(syn::Error::new(` and its closing
    /// parens around each of the 60-odd in `resolve.rs` was five lines of scaffolding for one
    /// string, and made the message the least visible thing at the site.
    pub(crate) fn at(&mut self, span: Span, message: impl std::fmt::Display) {
        self.errors.push(syn::Error::new(span, message));
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
