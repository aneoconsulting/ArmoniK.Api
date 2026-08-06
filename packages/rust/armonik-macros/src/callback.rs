//! The field-reflection handshake, as seen from the consuming side.
//!
//! `#[armonik_macros::message]` emits, next to each struct, a
//! `__armonik_fields_*` callback macro carrying that struct's fields CPS-style:
//! invoked as `callback! { cont::path! { ctx } }`, it re-invokes the
//! continuation with a `fields { [name class]* }` block appended (see
//! `reflection` in `lib.rs` for the emitting side). Two proc macros continue it:
//! `__emit_convenience` (client methods) and `__emit_reflect` (reflection for a
//! type alias of a message).
//!
//! This module holds what both parse: the braced blocks of the protocol, the
//! sugar class of a field, and the `__armonik_ty_*` suffixes each class stands
//! for.

use syn::parse::ParseStream;
use syn::Ident;

/// The sugar class of a field, as the callback forwards it: how a convenience
/// method widens the parameter, and which type aliases the derive emitted for
/// the field.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Class {
    Plain,
    Into,
    Iter,
    Pairs,
    Filters,
}

impl Class {
    /// The `__armonik_ty_*` suffixes the derive emitted for a field of this
    /// class: the field's own type, plus the element or key/value types the
    /// sugar widens over.
    pub(crate) fn suffixes(&self, name: &Ident) -> Vec<String> {
        let name = name.to_string();
        match self {
            Class::Plain | Class::Into => vec![name],
            Class::Iter | Class::Filters => vec![format!("{name}_elem"), name],
            Class::Pairs => vec![format!("{name}_key"), format!("{name}_value"), name],
        }
    }
}

/// The `fields { [name class]* }` block the callback appends.
pub(crate) fn fields(input: ParseStream) -> syn::Result<Vec<(Ident, Class)>> {
    let mut fields = Vec::new();
    while !input.is_empty() {
        let unit;
        syn::bracketed!(unit in input);
        let name: Ident = unit.parse()?;
        let class: Ident = unit.parse()?;
        let class = match class.to_string().as_str() {
            "plain" => Class::Plain,
            "into" => Class::Into,
            "iter" => Class::Iter,
            "pairs" => Class::Pairs,
            "filters" => Class::Filters,
            other => {
                return Err(syn::Error::new(
                    class.span(),
                    format!("unknown sugar class `{other}`"),
                ))
            }
        };
        fields.push((name, class));
    }
    Ok(fields)
}

/// One braced block of a continuation's context, parsed by its contents.
pub(crate) fn braced<T>(
    input: ParseStream,
    parse: impl FnOnce(ParseStream) -> syn::Result<T>,
) -> syn::Result<T> {
    let content;
    syn::braced!(content in input);
    let value = parse(&content)?;
    if !content.is_empty() {
        return Err(content.error("unexpected trailing tokens"));
    }
    Ok(value)
}
