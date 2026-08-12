//! What each site accepts of `#[armonik(...)]`, and the scan that enforces it.
//!
//! One collector for every field and variant, with the accepted keys passed in as [`Allowed`], so a
//! site cannot quietly start tolerating a key by forgetting to reject it. The unit tests at the
//! bottom are the guard on exactly that: the full expansions only run inside `armonik`, and the
//! differential harness only ever feeds them valid input.

use proc_macro2::Span;
use syn::spanned::Spanned;

use crate::attrs::{self, AttrItem, Errors};
use crate::plan::FieldAccess;

/// Compile-time checks for a plain field, derived from the descriptor.
pub(crate) fn unraw(ident: &syn::Ident) -> String {
    ident.to_string().trim_start_matches("r#").to_owned()
}

/// Parse + scan one field/variant's attributes per the site's [`Allowed`] set; `None` (with the
/// error pushed) when the attribute list itself does not parse.
pub(crate) fn scan_attrs(
    attrs: &[syn::Attribute],
    allowed: Allowed,
    reject: &str,
    errors: &mut Errors,
) -> Option<(FieldAttrs, bool)> {
    match attrs::parse(attrs) {
        Ok(entries) => Some(scan_field_attrs(&entries, allowed, reject, errors)),
        Err(err) => {
            errors.push(err);
            None
        }
    }
}

/// Span and access path of a struct field (named, or by position).
pub(crate) fn field_access(field: &syn::Field, index: usize) -> (Span, FieldAccess) {
    let span = field
        .ident
        .as_ref()
        .map(|ident| ident.span())
        .unwrap_or_else(|| field.ty.span());
    let access = match &field.ident {
        Some(ident) => FieldAccess::Named(ident.clone()),
        None => FieldAccess::Indexed(syn::Index::from(index)),
    };
    (span, access)
}

/// Parse the adapter type in `#[armonik(with = "path::To::Adapter")]`, pushing a spanned error (and
/// returning `None`) when it does not parse.
pub(crate) fn parse_adapter_type(
    lit: &syn::LitStr,
    span: Span,
    errors: &mut Errors,
) -> Option<syn::Type> {
    match syn::parse_str::<syn::Type>(&lit.value()) {
        Ok(ty) => Some(ty),
        Err(err) => {
            errors.at(span, format!("invalid adapter type in with = ...: {err}"));
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
    pub(crate) inline: Option<Span>,
    /// Proto messages a `with` adapter flattens away, so they have no Rust type of their own.
    /// Repeatable.
    pub(crate) absorbs: Vec<String>,
}

/// The `#[armonik(...)]` keys a site accepts. Any key not enabled here is a spanned `reject` error,
/// so each site keeps rejecting exactly what it did before.
#[derive(Clone, Copy, Default)]
pub(crate) struct Allowed {
    pub(crate) rename: bool,
    pub(crate) tag: bool,
    pub(crate) with: bool,
    pub(crate) present: bool,
    pub(crate) inline: bool,
    pub(crate) absorbs: bool,
}

/// Scan one field's or variant's `#[armonik(...)]` entries into a [`FieldAttrs`], pushing `reject`
/// (spanned) for any key outside `allowed` and for a malformed `tag` or `with`. Returns the
/// collected attributes and whether every entry was accepted. Callers that abandon a malformed
/// field gate on the bool; the rest rely on the pushed errors and ignore it.
pub(crate) fn scan_field_attrs(
    entries: &[attrs::AttrEntry],
    allowed: Allowed,
    reject: &str,
    errors: &mut Errors,
) -> (FieldAttrs, bool) {
    let mut collected = FieldAttrs::default();
    let mut ok = true;
    for entry in entries {
        let accepted = match &entry.item {
            AttrItem::Rename(lit) if allowed.rename => {
                collected.rename = Some(lit.value());
                true
            }
            AttrItem::Tag(lit) if allowed.tag => match lit.base10_parse::<u32>() {
                Ok(tag) => {
                    collected.tag = Some((entry.span, tag));
                    true
                }
                Err(err) => {
                    errors.at(entry.span, err);
                    false
                }
            },
            AttrItem::With(lit) if allowed.with => {
                match parse_adapter_type(lit, entry.span, errors) {
                    Some(ty) => {
                        collected.with = Some((entry.span, ty));
                        true
                    }
                    None => false,
                }
            }
            AttrItem::Present if allowed.present => {
                collected.present = true;
                true
            }
            AttrItem::Inline if allowed.inline => {
                collected.inline = Some(entry.span);
                true
            }
            AttrItem::Absorbs(lit) if allowed.absorbs => {
                collected.absorbs.push(lit.value());
                true
            }
            _ => {
                errors.at(entry.span, reject);
                false
            }
        };
        ok &= accepted;
    }
    (collected, ok)
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

    fn entry(item: AttrItem) -> attrs::AttrEntry {
        attrs::AttrEntry {
            span: Span::call_site(),
            item,
        }
    }

    fn lit(value: &str) -> syn::LitStr {
        syn::LitStr::new(value, Span::call_site())
    }

    fn scan(entries: &[attrs::AttrEntry], allowed: Allowed) -> (FieldAttrs, bool, bool) {
        let mut errors = Errors::new();
        let (collected, ok) = scan_field_attrs(entries, allowed, "reject", &mut errors);
        (collected, ok, errors.into_result().is_ok())
    }

    /// `absorbs` is collected where a site opts in and rejected where it does not. It used to be
    /// merely *tolerated* here and harvested by a second walk over the whole item, so the value
    /// read at this site was thrown away; the rule a naive shared collector would drop is the
    /// rejection half, which no other test covers.
    #[test]
    fn absorbs_is_gated_per_site() {
        let (collected, ok, clean) = scan(
            &[entry(AttrItem::Absorbs(lit("some.Msg")))],
            Allowed {
                absorbs: true,
                ..Allowed::default()
            },
        );
        assert!(ok && clean, "absorbs accepted where opted in");
        assert_eq!(collected.absorbs, ["some.Msg"], "and its value collected");

        let (collected, ok, clean) = scan(
            &[entry(AttrItem::Absorbs(lit("some.Msg")))],
            Allowed::default(),
        );
        assert!(!ok && !clean, "absorbs rejected where not opted in");
        assert!(collected.absorbs.is_empty(), "and not collected");
    }

    #[test]
    fn collects_only_enabled_keys() {
        let (collected, ok, clean) = scan(
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
        assert!(ok && clean);
        assert_eq!(collected.rename.as_deref(), Some("proto_name"));
        assert!(collected.present);
    }

    #[test]
    fn disallowed_key_is_rejected_and_not_collected() {
        // `present` at a site that only accepts `rename`.
        let (collected, ok, clean) = scan(
            &[entry(AttrItem::Present)],
            Allowed {
                rename: true,
                ..Allowed::default()
            },
        );
        assert!(!ok && !clean);
        assert!(!collected.present);
    }
}
