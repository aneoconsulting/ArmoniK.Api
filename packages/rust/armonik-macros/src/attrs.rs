//! Parsing of `#[armonik(...)]` helper attributes.
//!
//! The grammar is parsed by hand (rather than through `parse_nested_meta`)
//! because `enum` is a Rust keyword and must still be accepted as a key.
//! Also hosts the multi-error accumulator ([`Errors`]) the resolvers fill.
//! The user-facing documentation of the grammar lives on the two derive
//! macros in `lib.rs` — keep it in sync.

use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, LitInt, LitStr, Token};

/// A single `key` or `key = value` entry inside `#[armonik(...)]`.
pub(crate) enum AttrItem {
    /// `message = "full.proto.Name"` — proto message backing the type
    /// (repeatable for unified types) or, on an enum with `transparent`,
    /// the single-field wrapper message(s).
    Message(LitStr),
    /// `enum = "full.proto.Name"` — proto enum backing the type.
    Enum(LitStr),
    /// `oneof = "name"` — the type is the flattened oneof of that name.
    Oneof(LitStr),
    /// `generic` — no descriptor validation; fields carry explicit
    /// `tag` attributes.
    Generic,
    /// `transparent` — single-field wrapper message flattened into its field.
    Transparent,
    /// `rename = "proto_name"` — proto field/value name differs from the
    /// Rust name.
    Rename(LitStr),
    /// `tag = N` — explicit field tag, cross-checked against the descriptor
    /// unless in `generic` mode where it is authoritative.
    Tag(LitInt),
    /// `with = "path::to::Adapter"` — custom codec for a non-standard
    /// representation.
    With(LitStr),
    /// `present` — marker oneof variant selected by field presence.
    Present,
    /// `absorbs = "full.proto.Name"` — on a field/variant carrying a `with`
    /// adapter: the proto message the adapter flattens away, which therefore
    /// has no Rust type of its own. Harvested so the build script prunes it
    /// and the differential harness counts it as covered.
    Absorbs(LitStr),
    /// `replace(target = "...", service = "...", method = "...", input|output)`
    /// — the type stands in for its `message` at one RPC site; the build
    /// script rewrites that slot to the synthetic `target` message.
    Replace(ReplaceSpec),
}

/// Which slot of an RPC a `replace(...)` type occupies.
#[derive(Clone, Copy)]
pub(crate) enum Direction {
    Input,
    Output,
}

/// Parsed body of `replace( ... )`.
#[derive(Clone)]
pub(crate) struct ReplaceSpec {
    /// Synthetic proto message name to give the RPC slot in the stubs.
    pub(crate) target: LitStr,
    /// Proto service name.
    pub(crate) service: LitStr,
    /// Proto method name.
    pub(crate) method: LitStr,
    /// Which slot (`input`/`output`) the type occupies.
    pub(crate) direction: Direction,
}

impl Parse for ReplaceSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let head = input.span();
        let mut target = None;
        let mut service = None;
        let mut method = None;
        let mut direction = None;
        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            match ident.to_string().as_str() {
                "target" => {
                    input.parse::<Token![=]>()?;
                    target = Some(input.parse()?);
                }
                "service" => {
                    input.parse::<Token![=]>()?;
                    service = Some(input.parse()?);
                }
                "method" => {
                    input.parse::<Token![=]>()?;
                    method = Some(input.parse()?);
                }
                "input" => direction = Some(Direction::Input),
                "output" => direction = Some(Direction::Output),
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown replace(...) key `{other}` (expected one of: \
                             target, service, method, input, output)"
                        ),
                    ))
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        let missing = |what: &str| syn::Error::new(head, format!("replace(...) requires `{what}`"));
        Ok(ReplaceSpec {
            target: target.ok_or_else(|| missing("target = \"...\""))?,
            service: service.ok_or_else(|| missing("service = \"...\""))?,
            method: method.ok_or_else(|| missing("method = \"...\""))?,
            direction: direction.ok_or_else(|| missing("`input` or `output`"))?,
        })
    }
}

pub(crate) struct AttrEntry {
    pub(crate) span: Span,
    pub(crate) item: AttrItem,
}

/// Parse the `= <value>` tail shared by every `key = value` entry; the value
/// type (`LitStr`/`LitInt`) is inferred from the `AttrItem` constructor.
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
                "generic" => AttrItem::Generic,
                "transparent" => AttrItem::Transparent,
                "present" => AttrItem::Present,
                "replace" => {
                    let content;
                    syn::parenthesized!(content in input);
                    AttrItem::Replace(content.parse()?)
                }
                other => {
                    return Err(syn::Error::new(
                        span,
                        format!(
                            "unknown armonik attribute key `{other}` (expected one of: \
                             message, enum, oneof, rename, tag, with, absorbs, generic, \
                             transparent, present, replace)"
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

/// Span of every key token of every `#[armonik(...)]` attribute in `attrs`,
/// for the hover-documentation anchors (see `doc_anchors` in lib.rs).
/// Malformed attributes are skipped — the real parse reports them.
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
