//! One module per shape a type can take, each holding both halves: how it is resolved against the
//! descriptor, and how it is emitted.
//!
//! Shape-major, because that is the unit anyone reads: understanding what `#[armonik(transparent)]`
//! does used to mean jumping between `resolve.rs` and `codegen.rs` across a three-thousand-line
//! boundary. The plan/emit discipline is kept *within* each module, and the modules around it are
//! what make that checkable: `plan` names no `TokenStream`, and `emit` reads the descriptor's
//! vocabulary (`FieldKind`, `Cardinality`) without ever reaching for a `DescriptorIndex` to walk.

pub(crate) mod enumeration;
pub(crate) mod generic;
pub(crate) mod oneof;
pub(crate) mod plain;
pub(crate) mod transparent;
