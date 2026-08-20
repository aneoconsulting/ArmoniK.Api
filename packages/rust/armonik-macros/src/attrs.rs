//! The `#[armonik(...)]` attribute grammar: parsing, and the per-site scan that decides what
//! each field or variant accepts.
//!
//! Parsed by hand rather than through `parse_nested_meta`, because `enum` is a Rust keyword and
//! must still be accepted as a key. One collector for every field and variant, with the accepted
//! keys passed in as [`Allowed`], so a site cannot quietly start tolerating a key by forgetting to
//! reject it. A scan that fails records into the [`Generator`] and answers `None`, like every other
//! step: what the site does with that is the site's own. The user-facing grammar documentation lives
//! on the two macros in `lib.rs`; keep it in sync.

use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, LitInt, LitStr, Token};

use crate::generator::Generator;

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
    /// `inlined`: a proto message layer gets no Rust type of its own; what it contains lives
    /// directly at the site. On a struct variant, the member message's fields spread into the
    /// variant; on a field or tuple variant, the wrapper or key/value pair layer unwrapped from
    /// the descriptor (the inner value, or a map).
    Inlined,
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
                "service" => AttrItem::Service(eq_then(input)?),
                "rpc" => AttrItem::Rpc(eq_then(input)?),
                "generic" => AttrItem::Generic,
                "transparent" => AttrItem::Transparent,
                "present" => AttrItem::Present,
                "inlined" => AttrItem::Inlined,
                other => {
                    return Err(syn::Error::new(
                        span,
                        format!(
                            "unknown armonik attribute key `{other}` (expected one of: \
                             message, enum, oneof, rename, tag, with, service, rpc, \
                             generic, transparent, present, inlined)"
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

/// The name an identifier matches proto names by: the ident, minus any raw prefix.
pub(crate) fn unraw(ident: &syn::Ident) -> String {
    ident.to_string().trim_start_matches("r#").to_owned()
}

/// Parse + scan one field/variant's attributes per the site's [`Allowed`] set; `None` (with the
/// error pushed) when the attribute list itself does not parse.
pub(crate) fn scan_attrs(
    attrs: &[syn::Attribute],
    allowed: Allowed,
    reject: &str,
    generator: &mut Generator,
) -> Option<FieldAttrs> {
    match parse(attrs) {
        Ok(entries) => Some(scan_field_attrs(&entries, allowed, reject, generator)),
        Err(err) => {
            generator.record(err);
            None
        }
    }
}

/// Parse the adapter type in `#[armonik(with = "path::To::Adapter")]`, pushing a spanned error (and
/// returning `None`) when it does not parse.
fn parse_adapter_type(
    lit: &syn::LitStr,
    span: Span,
    generator: &mut Generator,
) -> Option<syn::Type> {
    match syn::parse_str::<syn::Type>(&lit.value()) {
        Ok(ty) => Some(ty),
        Err(err) => {
            generator.error(span, format!("invalid adapter type in with = ...: {err}"));
            None
        }
    }
}

/// The field/variant-level `#[armonik(...)]` keys collected by [`scan_field_attrs`]. Each site
/// reads only the keys it opted into through [`Allowed`]; the rest stay at their defaults.
#[derive(Default)]
pub(crate) struct FieldAttrs {
    pub(crate) rename: Option<String>,
    pub(crate) tag: Option<(Span, u32)>,
    pub(crate) with: Option<(Span, syn::Type)>,
    pub(crate) present: bool,
    pub(crate) inlined: Option<Span>,
}

/// The `#[armonik(...)]` keys a site accepts. Any key not enabled here is a spanned `reject` error,
/// so each site keeps rejecting exactly what it did before.
#[derive(Clone, Copy, Default)]
pub(crate) struct Allowed {
    pub(crate) rename: bool,
    pub(crate) tag: bool,
    pub(crate) with: bool,
    pub(crate) present: bool,
    pub(crate) inlined: bool,
}

/// Scan one field's or variant's `#[armonik(...)]` entries into a [`FieldAttrs`], pushing `reject`
/// (spanned) for any key outside `allowed` and for a malformed `tag` or `with`.
///
/// A rejected entry is reported and skipped, so the collected attributes are whatever was valid;
/// callers act on the pushed errors, not on a return value.
fn scan_field_attrs(
    entries: &[AttrEntry],
    allowed: Allowed,
    reject: &str,
    generator: &mut Generator,
) -> FieldAttrs {
    let mut collected = FieldAttrs::default();
    for entry in entries {
        match &entry.item {
            AttrItem::Rename(lit) if allowed.rename => collected.rename = Some(lit.value()),
            AttrItem::Tag(lit) if allowed.tag => match lit.base10_parse::<u32>() {
                Ok(tag) => collected.tag = Some((entry.span, tag)),
                Err(err) => generator.error(entry.span, err),
            },
            AttrItem::With(lit) if allowed.with => {
                if let Some(ty) = parse_adapter_type(lit, entry.span, generator) {
                    collected.with = Some((entry.span, ty));
                }
            }
            AttrItem::Present if allowed.present => collected.present = true,
            AttrItem::Inlined if allowed.inlined => collected.inlined = Some(entry.span),
            _ => generator.error(entry.span, reject),
        }
    }
    collected
}

#[cfg(test)]
mod tests {
    //! Guards for [`scan_field_attrs`], the one place deciding which `#[armonik(...)]` keys each
    //! field or variant site accepts. The full expansions only run inside the `armonik` crate (they
    //! read the build-script descriptor), and the differential harness only fuzzes valid input, so
    //! the per-site rejection rules, which the shared collector could silently weaken, are pinned
    //! here instead.

    use proc_macro2::Span;

    use super::*;

    fn entry(item: AttrItem) -> AttrEntry {
        AttrEntry {
            span: Span::call_site(),
            item,
        }
    }

    fn lit(value: &str) -> syn::LitStr {
        syn::LitStr::new(value, Span::call_site())
    }

    fn scan(entries: &[AttrEntry], allowed: Allowed) -> (FieldAttrs, bool) {
        let mut generator = Generator::new();
        let collected = scan_field_attrs(entries, allowed, "reject", &mut generator);
        (collected, !generator.poisoned())
    }

    #[test]
    fn collects_only_enabled_keys() {
        let (collected, clean) = scan(
            &[
                entry(AttrItem::Rename(lit("proto_name"))),
                entry(AttrItem::Present),
            ],
            Allowed {
                rename: true,
                present: true,
                ..Allowed::default()
            },
        );
        assert!(clean);
        assert_eq!(collected.rename.as_deref(), Some("proto_name"));
        assert!(collected.present);
    }

    #[test]
    fn disallowed_key_is_rejected_and_not_collected() {
        // `present` at a site that only accepts `rename`.
        let (collected, clean) = scan(
            &[entry(AttrItem::Present)],
            Allowed {
                rename: true,
                ..Allowed::default()
            },
        );
        assert!(!clean);
        assert!(!collected.present);
    }
}
