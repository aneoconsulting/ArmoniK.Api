//! Field-and-oneof lookup with coverage over one proto message.
//!
//! Resolving any shape is the same walk: take a Rust name, find what it maps to, record that it was
//! consumed, and at the end turn whatever was not consumed into "this proto field is not covered"
//! errors. Structs, whole-message enums and inlined members all drive this.

use proc_macro2::Span;

use crate::descriptor::{FieldMeta, MessageMeta};
use crate::generator::Generator;

/// Field-or-oneof lookup with coverage over one proto message: resolves names, records what was
/// consumed, reports misses with the sorted "available:" list, and turns leftovers into
/// completeness errors. One per message in [`crate::resolve`]'s plain-struct walk; also drives inlined struct
/// variants.
pub(crate) struct Matcher<'a> {
    message_name: &'a str,
    meta: &'a MessageMeta,
    consumed: Vec<bool>,
    consumed_oneofs: Vec<bool>,
}

pub(crate) enum Found<'a> {
    Field(&'a FieldMeta),
    Oneof { tags: Vec<u32> },
}

impl<'a> Matcher<'a> {
    pub(crate) fn new(message_name: &'a str, meta: &'a MessageMeta) -> Self {
        Self {
            message_name,
            meta,
            consumed: vec![false; meta.fields.len()],
            consumed_oneofs: vec![false; meta.oneofs.len()],
        }
    }

    /// Look `proto_name` up among the message's fields and oneofs, marking it consumed. `None`
    /// (with a spanned error) when nothing matches or the field can only be mapped through its
    /// oneof.
    pub(crate) fn find(
        &mut self,
        proto_name: &str,
        span: Span,
        generator: &mut Generator,
    ) -> Option<Found<'a>> {
        if let Some(position) = self
            .meta
            .fields
            .iter()
            .position(|field| field.name == proto_name)
        {
            self.consumed[position] = true;
            let field = &self.meta.fields[position];
            if field.oneof.is_some() {
                generator.error(
                    span,
                    format!(
                        "proto field `{}.{proto_name}` belongs to a oneof; \
                         map the whole oneof to one field named after it",
                        self.message_name
                    ),
                );
                return None;
            }
            return Some(Found::Field(field));
        }
        if let Some((index, oneof)) = self.meta.oneof(proto_name) {
            self.consumed_oneofs[index] = true;
            let tags = oneof
                .fields
                .iter()
                .map(|&field| self.meta.fields[field].tag)
                .collect();
            return Some(Found::Oneof { tags });
        }
        let available = self
            .meta
            .fields
            .iter()
            .map(|field| field.name.clone())
            .chain(self.meta.oneofs.iter().map(|oneof| oneof.name.clone()))
            .collect();
        generator.record(unknown_name(
            span,
            "field or oneof",
            proto_name,
            &format!("proto message `{}`", self.message_name),
            available,
            "use #[armonik(rename = \"...\")] if the names differ",
        ));
        None
    }

    /// Completeness: every uncovered proto field and oneof is an error at `at`. A field is covered
    /// through its oneof when the oneof was mapped whole; a oneof is covered when every member was
    /// mapped individually. Callers skip this
    /// when a Rust field failed to resolve, since an unconsumed proto field then already has its
    /// probable explanation on screen: one mistake reads as one error.
    pub(crate) fn check_complete(&self, at: Span, generator: &mut Generator) {
        for (position, field) in self.meta.fields.iter().enumerate() {
            let in_oneof_group = field.oneof.is_some_and(|oneof| self.consumed_oneofs[oneof]);
            if !self.consumed[position] && !in_oneof_group {
                generator.error(
                    at,
                    format!(
                        "proto field `{}.{}` (tag {}) is not covered by any Rust field",
                        self.message_name, field.name, field.tag
                    ),
                );
            }
        }
        for (index, oneof) in self.meta.oneofs.iter().enumerate() {
            let members_covered = oneof.fields.iter().all(|&field| self.consumed[field]);
            if !self.consumed_oneofs[index] && !members_covered {
                generator.error(
                    at,
                    format!(
                        "proto oneof `{}.{}` is not covered by any Rust field",
                        self.message_name, oneof.name
                    ),
                );
            }
        }
    }
}

/// "proto message/enum `name` not found in the compiled descriptor set".
pub(crate) fn not_found(span: Span, what: &str, name: &str) -> syn::Error {
    syn::Error::new(
        span,
        format!("proto {what} `{name}` not found in the compiled descriptor set"),
    )
}

/// The shared shape of every name-lookup miss:
/// `no {what} named {name} in {container} (available: ...); {hint}`.
pub(crate) fn unknown_name(
    span: Span,
    what: &str,
    name: &str,
    container: &str,
    mut available: Vec<String>,
    hint: &str,
) -> syn::Error {
    available.sort_unstable();
    syn::Error::new(
        span,
        format!(
            "no {what} named `{name}` in {container} (available: {}); {hint}",
            available.join(", ")
        ),
    )
}
