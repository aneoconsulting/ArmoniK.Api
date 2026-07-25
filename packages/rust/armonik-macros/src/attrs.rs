//! Parsing of `#[armonik(...)]` helper attributes.
//!
//! The grammar is parsed by hand (rather than through `parse_nested_meta`)
//! because `enum` is a Rust keyword and must still be accepted as a key.

// Parts of the grammar are not consumed yet; the allow goes away once the
// wire-implementation codegen lands.
#![allow(dead_code)]

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
    /// `tag`/`kind` attributes.
    Generic,
    /// `transparent` — single-field wrapper message flattened into its field.
    Transparent,
    /// `rename = "proto_name"` — proto field/value name differs from the
    /// Rust name.
    Rename(LitStr),
    /// `tag = N` — explicit field tag, cross-checked against the descriptor
    /// unless in `generic` mode where it is authoritative.
    Tag(LitInt),
    /// `kind = "..."` — explicit wire kind, only in `generic` mode.
    Kind(LitStr),
    /// `with = "path::to::Adapter"` — custom codec for a non-standard
    /// representation.
    With(LitStr),
    /// `present` — marker oneof variant selected by field presence.
    Present,
}

pub(crate) struct AttrEntry {
    pub(crate) span: Span,
    pub(crate) item: AttrItem,
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
                "message" => {
                    input.parse::<Token![=]>()?;
                    AttrItem::Message(input.parse()?)
                }
                "enum" => {
                    input.parse::<Token![=]>()?;
                    AttrItem::Enum(input.parse()?)
                }
                "oneof" => {
                    input.parse::<Token![=]>()?;
                    AttrItem::Oneof(input.parse()?)
                }
                "rename" => {
                    input.parse::<Token![=]>()?;
                    AttrItem::Rename(input.parse()?)
                }
                "tag" => {
                    input.parse::<Token![=]>()?;
                    AttrItem::Tag(input.parse()?)
                }
                "kind" => {
                    input.parse::<Token![=]>()?;
                    AttrItem::Kind(input.parse()?)
                }
                "with" => {
                    input.parse::<Token![=]>()?;
                    AttrItem::With(input.parse()?)
                }
                "generic" => AttrItem::Generic,
                "transparent" => AttrItem::Transparent,
                "present" => AttrItem::Present,
                other => {
                    return Err(syn::Error::new(
                        span,
                        format!(
                            "unknown armonik attribute key `{other}` (expected one of: \
                             message, enum, oneof, rename, tag, kind, with, generic, \
                             transparent, present)"
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
