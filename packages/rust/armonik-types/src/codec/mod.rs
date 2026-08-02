//! Runtime support for the wire representation of the API types.
//!
//! The `armonik-macros` derives emit implementations of [`prost::Message`]
//! built from the building blocks in this module:
//!
//! - [`ProtoField`] is implemented by every type that can appear as a field
//!   of a message (scalars, `String`, `Bytes`, containers, well-known types,
//!   and — through the derives — every API message and enum). The wire
//!   representation is chosen by the type system through this trait, while
//!   tags and expected kinds come from the protobuf descriptor at expansion
//!   time.
//! - [`ProtoOneof`] is implemented by flattened-oneof enums; the containing
//!   message routes the oneof's tag set to it.
//! - [`ProtoAdapter`] is the escape hatch for fields whose Rust
//!   representation differs structurally from the proto (e.g. a repeated
//!   pair message exposed as a `HashMap`).
//!
//! The associated consts ([`ProtoField::KIND`], [`ProtoField::CARDINALITY`],
//! [`ProtoField::NAMES`], …) exist so that derive-emitted `const` assertions
//! can check the Rust type against the descriptor at compile time.

use prost::bytes::{Buf, BufMut};
use prost::encoding::{DecodeContext, WireType};
use prost::DecodeError;

pub(crate) mod adapters;
mod bytes;
mod containers;
pub(crate) mod enumeration;
mod scalars;
mod well_known;
pub(crate) mod wrapper_enum;

/// Wire-level kind of a protobuf field, checked by derive-emitted
/// const-asserts against the descriptor. Only the kinds with a [`ProtoField`]
/// impl are listed, so every variant is live; a proto field of any other wire
/// kind (`sint*`/`fixed*`/`sfixed*`, which no ArmoniK field uses) is a spanned
/// "unsupported wire kind" compile error from the derive rather than a
/// silently-unmatchable pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FieldKind {
    Double,
    Float,
    Int32,
    Int64,
    UInt32,
    UInt64,
    Bool,
    String,
    Bytes,
    Message,
    Enum,
}

/// Cardinality of a protobuf field, checked by derive-emitted const-asserts
/// against the descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cardinality {
    /// Singular proto3 field: implicit presence.
    Singular,
    /// `optional` proto3 field: explicit presence.
    Optional,
    /// Repeated field.
    Repeated,
    /// Map field.
    Map,
}

/// A type that can be encoded and decoded as a single protobuf field.
pub(crate) trait ProtoField: Default + PartialEq {
    const KIND: FieldKind;
    const CARDINALITY: Cardinality = Cardinality::Singular;
    /// Full proto type names this Rust type can stand for; empty means
    /// unchecked. Containers propagate the names of their element type.
    const NAMES: &'static [&'static str] = &[];
    /// Key/value kinds; only meaningful when `CARDINALITY` is `Map`.
    const MAP_KEY_KIND: FieldKind = FieldKind::Message;
    const MAP_VALUE_KIND: FieldKind = FieldKind::Message;

    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut);
    fn merge_field(
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>;
    fn encoded_len_field(tag: u32, value: &Self) -> usize;

    /// proto3 implicit presence: when `true`, a singular field is skipped on
    /// encode. The default is "equal to the type's default value", which is
    /// right for scalars, enums, strings and containers. Message-kind types
    /// override it with [`message_is_default`] ("encodes to zero bytes", which
    /// differs once a nested field is always emitted), and wrapper enums
    /// override it to `false` (always emitted).
    fn is_default(value: &Self) -> bool {
        value == &Self::default()
    }

    // Repeated forms, used by `Vec<Self>`. Packable kinds override them with
    // their packed encodings; the defaults implement the unpacked form.

    fn encode_repeated(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        for value in values {
            Self::encode_field(tag, value, buf);
        }
    }

    fn encoded_len_repeated(tag: u32, values: &[Self]) -> usize {
        values
            .iter()
            .map(|value| Self::encoded_len_field(tag, value))
            .sum()
    }

    fn merge_repeated(
        wire_type: WireType,
        values: &mut Vec<Self>,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        let mut value = Self::default();
        Self::merge_field(wire_type, &mut value, buf, ctx)?;
        values.push(value);
        Ok(())
    }
}

/// Marker: this Rust type IS the protobuf message(s) in [`Msg::NAMES`]. The
/// blanket impl below is the single [`ProtoField`] impl for every
/// message-shaped type — derived messages, transparent wrapper enums,
/// well-known types — so the derives emit a one-line `Msg` impl instead of a
/// full `ProtoField` one.
///
/// A type implements `Msg` XOR a concrete `ProtoField`: only message-kind
/// types belong here. Plain proto enums keep concrete impls — a second
/// blanket would overlap this one (E0119).
pub(crate) trait Msg: prost::Message + Default + PartialEq {
    /// See [`ProtoField::NAMES`].
    const NAMES: &'static [&'static str];
    /// Transparent wrapper enums encode their zero as a non-empty wrapper, so
    /// they are always emitted as a field (presence-significant).
    const ALWAYS_PRESENT: bool = false;
}

impl<T: Msg> ProtoField for T {
    const KIND: FieldKind = FieldKind::Message;
    const NAMES: &'static [&'static str] = <T as Msg>::NAMES;

    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        prost::encoding::message::encode(tag, value, buf);
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        prost::encoding::message::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_field(tag: u32, value: &Self) -> usize {
        prost::encoding::message::encoded_len(tag, value)
    }

    fn is_default(value: &Self) -> bool {
        !Self::ALWAYS_PRESENT && message_is_default(value)
    }

    // Repeated forms: the trait's unpacked defaults (messages never pack).
}

/// A flattened-oneof enum: the value encodes its own variant tag, and the
/// containing message routes the oneof's whole tag set to `merge_oneof`.
pub(crate) trait ProtoOneof: Sized {
    fn encode_oneof(value: &Self, buf: &mut impl BufMut);
    fn merge_oneof(
        tag: u32,
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>;
    fn encoded_len_oneof(value: &Self) -> usize;
}

/// Custom codec for a field whose Rust representation differs structurally
/// from its proto counterpart (`#[armonik(with = "...")]`). Implementations
/// are zero-sized marker types.
pub(crate) trait ProtoAdapter<T> {
    fn encode_field(tag: u32, value: &T, buf: &mut impl BufMut);
    fn merge_field(
        wire_type: WireType,
        value: &mut T,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>;
    fn encoded_len_field(tag: u32, value: &T) -> usize;
    fn is_default(value: &T) -> bool;

    /// Project the field at `tag` of a dynamic message onto the equivalence
    /// classes this adapter's Rust representation defines (for the
    /// differential harness; see `crate::differential::Normalize`). The
    /// default is the identity: adapters that only restructure the wire
    /// representation lose nothing.
    #[cfg(feature = "_differential")]
    fn normalize_dynamic(
        message: &mut crate::differential::prost_reflect::DynamicMessage,
        tag: u32,
    ) {
        let _ = (message, tag);
    }
}

/// Read a length-delimited sub-buffer: decode the length varint, guard
/// against a truncated buffer, and return a `Take` limited to the body.
pub(crate) fn read_delimited<B: Buf + ?Sized>(
    buf: &mut B,
) -> Result<prost::bytes::buf::Take<&mut B>, DecodeError> {
    let len = prost::encoding::decode_varint(&mut &mut *buf)? as usize;
    if buf.remaining() < len {
        // prost offers no other public constructor; pinned to prost 0.14.
        #[allow(deprecated)]
        return Err(DecodeError::new("buffer underflow"));
    }
    Ok(buf.take(len))
}

/// A message-kind field is absent (proto3 default) exactly when it encodes to
/// zero bytes. This is deliberately *not* `value == M::default()`: a message
/// can hold only default sub-values yet still encode to a non-empty buffer when
/// one of them is always emitted — a wrapper enum encodes its zero as an empty
/// wrapper — and such a field must survive the round-trip. This is why
/// message-kind types override the [`ProtoField::is_default`] trait default.
pub(crate) fn message_is_default<M: prost::Message>(value: &M) -> bool {
    value.encoded_len() == 0
}

/// Const string equality, for derive-emitted assertions.
pub(crate) const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Whether `names` covers the proto type `expected`. An empty list means the
/// type is unchecked (scalars, adapters, generic instantiations).
pub(crate) const fn names_match(names: &'static [&'static str], expected: &str) -> bool {
    if names.is_empty() {
        return true;
    }
    let mut i = 0;
    while i < names.len() {
        if str_eq(names[i], expected) {
            return true;
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests;
