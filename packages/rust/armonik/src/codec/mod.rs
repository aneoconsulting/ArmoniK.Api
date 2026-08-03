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
//! [`ProtoField::SHAPE`] exists so that one derive-emitted `const` assertion
//! per field can check the Rust type against the descriptor at compile time
//! (see [`shape_matches`]).

use prost::bytes::{Buf, BufMut};
use prost::encoding::{DecodeContext, WireType};
use prost::DecodeError;

pub(crate) mod adapters;
mod containers;
pub(crate) mod enumeration;
mod leaves;
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

impl FieldKind {
    pub(crate) const fn same(self, other: Self) -> bool {
        self as u8 == other as u8
    }
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

impl Cardinality {
    pub(crate) const fn same(self, other: Self) -> bool {
        self as u8 == other as u8
    }
}

/// Compile-time shape of a [`ProtoField`] impl. The derive emits one const
/// assert per descriptor-checked field, comparing the field type's `SHAPE`
/// against an [`Expect`] built from the descriptor.
#[derive(Clone, Copy)]
pub(crate) struct Shape {
    pub(crate) kind: FieldKind,
    pub(crate) cardinality: Cardinality,
    /// Full proto type names this Rust type can stand for; empty means
    /// unchecked. Containers propagate the names of their element type.
    pub(crate) names: &'static [&'static str],
    /// Key/value kinds when `cardinality` is [`Cardinality::Map`].
    pub(crate) map: Option<(FieldKind, FieldKind)>,
}

impl Shape {
    pub(crate) const fn scalar(kind: FieldKind) -> Self {
        Shape {
            kind,
            cardinality: Cardinality::Singular,
            names: &[],
            map: None,
        }
    }

    pub(crate) const fn enumeration(names: &'static [&'static str]) -> Self {
        Shape {
            names,
            ..Shape::scalar(FieldKind::Enum)
        }
    }
}

/// What the descriptor expects of one field, tokenized as a const literal by
/// the derive.
pub(crate) struct Expect {
    /// `None` for map fields (their kinds live in `map`).
    pub(crate) kind: Option<FieldKind>,
    /// Acceptable cardinalities (e.g. a singular message field may be either
    /// `Singular` or `Optional` in Rust).
    pub(crate) cardinalities: &'static [Cardinality],
    /// Expected proto type name for message/enum (element) kinds; a `SHAPE`
    /// with empty `names` is unchecked (scalars, adapters, generics).
    pub(crate) name: Option<&'static str>,
    pub(crate) map: Option<(FieldKind, FieldKind)>,
}

/// Whether a field type's [`Shape`] satisfies the descriptor's [`Expect`].
pub(crate) const fn shape_matches(shape: &Shape, expect: &Expect) -> bool {
    if let Some(kind) = expect.kind {
        if !shape.kind.same(kind) {
            return false;
        }
    }
    let mut card_ok = false;
    let mut i = 0;
    while i < expect.cardinalities.len() {
        card_ok |= shape.cardinality.same(expect.cardinalities[i]);
        i += 1;
    }
    if !card_ok {
        return false;
    }
    if let (Some(name), false) = (expect.name, shape.names.is_empty()) {
        let mut found = false;
        let mut i = 0;
        while i < shape.names.len() {
            found |= str_eq(shape.names[i], name);
            i += 1;
        }
        if !found {
            return false;
        }
    }
    if let Some((key, value)) = expect.map {
        let Some((shape_key, shape_value)) = shape.map else {
            return false;
        };
        if !(shape_key.same(key) && shape_value.same(value)) {
            return false;
        }
    }
    true
}

/// A type that can be encoded and decoded as a single protobuf field.
pub(crate) trait ProtoField: Default + PartialEq {
    const SHAPE: Shape;

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

    // The proto3 singular-field forms the derives emit: skip the default.

    fn encode_nondefault(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if !Self::is_default(value) {
            Self::encode_field(tag, value, buf);
        }
    }

    fn encoded_len_nondefault(tag: u32, value: &Self) -> usize {
        if Self::is_default(value) {
            0
        } else {
            Self::encoded_len_field(tag, value)
        }
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
    /// See [`Shape::names`].
    const NAMES: &'static [&'static str];
    /// Transparent wrapper enums encode their zero as a non-empty wrapper, so
    /// they are always emitted as a field (presence-significant).
    const ALWAYS_PRESENT: bool = false;
}

/// Whether `names` contains `name`; const, so the `service!`-emitted asserts
/// can check at compile time that a type implements an RPC's input or output
/// message.
pub(crate) const fn names_contain(names: &'static [&'static str], name: &str) -> bool {
    let name = name.as_bytes();
    let mut i = 0;
    while i < names.len() {
        let candidate = names[i].as_bytes();
        if candidate.len() == name.len() {
            let mut j = 0;
            while j < name.len() && candidate[j] == name[j] {
                j += 1;
            }
            if j == name.len() {
                return true;
            }
        }
        i += 1;
    }
    false
}

impl<T: Msg> ProtoField for T {
    const SHAPE: Shape = Shape {
        kind: FieldKind::Message,
        cardinality: Cardinality::Singular,
        names: <T as Msg>::NAMES,
        map: None,
    };

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

    fn encode_nondefault(tag: u32, value: &T, buf: &mut impl BufMut) {
        if !Self::is_default(value) {
            Self::encode_field(tag, value, buf);
        }
    }

    fn encoded_len_nondefault(tag: u32, value: &T) -> usize {
        if Self::is_default(value) {
            0
        } else {
            Self::encoded_len_field(tag, value)
        }
    }

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

/// An empty length-delimited body: what a `#[armonik(present)]` message
/// marker encodes.
pub(crate) mod empty_body {
    use prost::bytes::BufMut;
    use prost::encoding::{self, WireType};

    pub(crate) fn encode(tag: u32, buf: &mut impl BufMut) {
        encoding::encode_key(tag, WireType::LengthDelimited, buf);
        encoding::encode_varint(0, buf);
    }

    pub(crate) fn encoded_len(tag: u32) -> usize {
        encoding::key_len(tag) + 1
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

#[cfg(test)]
mod tests;
