//! Runtime support for the wire representation of the API types.
//!
//! The `armonik-macros` expansions build their [`prost::Message`] impls out of the pieces here:
//!
//! - [`ProtoField`], implemented by every type that can appear as a field of a message: scalars,
//!   `String`, `Bytes`, containers, well-known types, and, through the derives, every API message
//!   and enum. The trait picks the wire representation from the Rust type, while tags and expected
//!   kinds come from the descriptor at expansion time.
//! - [`ProtoAdapter`], the escape hatch for fields whose Rust representation differs structurally
//!   from the proto (a repeated pair message exposed as a `HashMap`).
//!
//! [`ProtoField::SHAPE`] lets one derive-emitted `const` assertion per field check the Rust type
//! against the descriptor at compile time (see [`shape_matches`]).
//!
//! An implicit-presence leaf leaves its zero off the wire, the way any proto3 encoder does: a
//! scalar, `String`, `Bytes` or enum field holding the proto zero is not written, and a proto3
//! reader cannot tell that from an absent field. Nothing else skips on the value. Message fields,
//! oneof members, `present` markers and adapter fields are written whatever they hold, because for
//! them presence is information -- a member being there is what selects its variant. The skip
//! lives in [`ProtoField::encode_implicit`] over an [`ProtoField::is_zero`] that is `false` unless
//! a leaf overrides it, so a codec opts into skipping rather than out of it, and the derives only
//! pick which of the two entry points each slot is written through. The decode side keeps its
//! "absent = default" reading (seeded from `Default`, the proto zero for every armonik type).
//!
//! What the Rust *type* decides rather than the value: `Option<T>` omits `None`, a oneof writes its
//! active member and no other, and an empty container writes nothing (there are no elements to
//! write).

use prost::bytes::{Buf, BufMut};
use prost::encoding::{DecodeContext, WireType};
use prost::DecodeError;

pub(crate) mod adapters;
mod containers;
#[cfg(feature = "serde")]
pub(crate) mod enum_serde;
pub(crate) mod enumeration;
mod leaves;

/// Wire-level kind of a protobuf field, checked by derive-emitted const-asserts against the
/// descriptor. Only the kinds with a [`ProtoField`] impl are listed, so every variant is live; a
/// proto field of any other wire kind (`sint*`/`fixed*`/`sfixed*`, which no ArmoniK field uses) is
/// a spanned "unsupported wire kind" compile error from the derive rather than a
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

/// Cardinality of a protobuf field, checked by derive-emitted const-asserts against the descriptor.
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

/// Compile-time shape of a [`ProtoField`] impl. The derive emits one const assert per
/// descriptor-checked field, comparing the field type's `SHAPE` against an [`Expect`] built from
/// the descriptor.
#[derive(Clone, Copy)]
pub(crate) struct Shape {
    pub(crate) kind: FieldKind,
    pub(crate) cardinality: Cardinality,
    /// Full proto type names this Rust type can stand for; empty means unchecked. Containers
    /// propagate the names of their element type.
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

/// What the descriptor expects of one field, tokenized as a const literal by the derive.
pub(crate) struct Expect {
    /// `None` for map fields (their kinds live in `map`).
    pub(crate) kind: Option<FieldKind>,
    /// Acceptable cardinalities (e.g. a singular message field may be either `Singular` or
    /// `Optional` in Rust).
    pub(crate) cardinalities: &'static [Cardinality],
    /// Expected proto type name for message/enum (element) kinds; a `SHAPE` with empty `names` is
    /// unchecked (scalars, adapters, generics).
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
        if !names_contain(shape.names, name) {
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
///
/// `Default` is what decoding seeds a field from (the proto zero value for every armonik type);
/// nothing here ever compares a value *against* that default, so no `PartialEq` is required.
pub(crate) trait ProtoField: Default {
    const SHAPE: Shape;

    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut);
    fn merge_field(
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>;
    fn encoded_len_field(tag: u32, value: &Self) -> usize;

    /// Whether this is the proto zero. `false` unless a leaf overrides it, which is what keeps
    /// messages, wrappers and containers on the wire whatever they hold.
    fn is_zero(value: &Self) -> bool {
        let _ = value;
        false
    }

    /// The implicit-presence pair: a zero is left out, since a proto3 reader cannot tell it from an
    /// absent field. Both read the same predicate, so the length cannot disagree with what is
    /// written. Fields whose presence is the information (a oneof's active member) go through
    /// `encode_field` instead, and the derives pick between the two per slot.
    fn encode_implicit(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if !Self::is_zero(value) {
            Self::encode_field(tag, value, buf);
        }
    }

    fn encoded_len_implicit(tag: u32, value: &Self) -> usize {
        if Self::is_zero(value) {
            0
        } else {
            Self::encoded_len_field(tag, value)
        }
    }

    // Repeated forms, used by `Vec<Self>`. Packable kinds override them with their packed
    // encodings; the defaults implement the unpacked form.

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

/// Marker: this Rust type is the protobuf messages in [`Msg::NAMES`]. The blanket impl below is the
/// single [`ProtoField`] impl covering every message-shaped type (derived messages, transparent
/// wrapper enums, well-known types), so the derives emit a one-line `Msg` impl instead of a full
/// `ProtoField` one.
///
/// A type implements `Msg` xor a concrete `ProtoField`: only message-kind types belong here. Plain
/// proto enums keep concrete impls, since a second blanket would overlap this one (E0119).
pub(crate) trait Msg: prost::Message + Default {
    /// See [`Shape::names`].
    const NAMES: &'static [&'static str];
}

/// Whether `names` contains `name`; const, so the `service!`-emitted asserts can check at compile
/// time that a type implements an RPC's input or output message.
pub(crate) const fn names_contain(names: &'static [&'static str], name: &str) -> bool {
    let mut i = 0;
    while i < names.len() {
        if str_eq(names[i], name) {
            return true;
        }
        i += 1;
    }
    false
}

/// What tokens cannot prove about an rpc line: that the type it names is the one the descriptor's
/// method signature calls for.
///
/// One call per type rather than the `assert!` spelled out at each: those were eight emitted lines
/// apiece, 944 across the twelve `service!` invocations, and identical but for a string literal.
/// Failing here, rustc names the concrete type in a `note: inside assert_request_message::<...>`
/// frame, which the inlined form did not do. Two functions and not one taking both types, so that
/// each call carries the span of the type path it checks.
pub(crate) const fn assert_request_message<T: Msg>(input: &'static str) {
    assert!(
        names_contain(<T as Msg>::NAMES, input),
        "the request type does not implement this RPC's input message",
    );
}

/// The response half of [`assert_request_message`].
pub(crate) const fn assert_response_message<T: Msg>(output: &'static str) {
    assert!(
        names_contain(<T as Msg>::NAMES, output),
        "the response type does not implement this RPC's output message",
    );
}

/// The tag and instantiated shape of every field of a `#[armonik(generic)]` type.
///
/// A generic type names no proto message, so its fields cannot be checked where they are declared.
/// They can be checked where the type is *instantiated*, and this is what carries them there: the
/// associated const is written against the type parameters, so `Sort<tasks::Field>` reports the
/// shape `tasks::Field` actually has.
pub(crate) trait GenericFields {
    /// `(tag, shape)` per field, in ascending tag order.
    const FIELDS: &'static [(u32, Shape)];
}

/// Whether every field of a generic instantiation matches what the descriptor says, tag for tag.
///
/// Length first, so a proto revision that adds or drops a field is caught as such rather than as a
/// mismatch on whichever field shifted.
pub(crate) const fn fields_match(fields: &[(u32, Shape)], expect: &[(u32, Expect)]) -> bool {
    if fields.len() != expect.len() {
        return false;
    }
    let mut i = 0;
    while i < fields.len() {
        let (tag, shape) = &fields[i];
        let (want_tag, want) = &expect[i];
        if *tag != *want_tag || !shape_matches(shape, want) {
            return false;
        }
        i += 1;
    }
    true
}

/// What tokens cannot prove about an `#[armonik_macros::alias]`: that the generic instantiation it
/// names has the fields the proto message it registers under declares.
///
/// `generic` mode skips descriptor validation because a generic type names no message, which left
/// the instantiations checked only by the differential harness. That harness does catch a
/// renumbered field (measured: renumbering `Sort.direction` fails `field_information_ratchet` and
/// `registered_types_roundtrip`, and says which field vanished), so this is not new detection. What
/// it is: the same fact at compile time, at the alias line, in every build rather than only under
/// `cargo test --all-features`, which is the only configuration that compiles the harness at all.
pub(crate) const fn assert_generic_fields<T: GenericFields>(expect: &[(u32, Expect)]) {
    assert!(
        fields_match(<T as GenericFields>::FIELDS, expect),
        "the generic instantiation does not have the fields this proto message declares",
    );
}

/// The oneof a `#[armonik(oneof = "...")]` enum stands for, as `message.oneof` paths.
///
/// The counterpart of [`Msg::NAMES`] for the shape that is not a message: an embedded oneof is a
/// fragment of one, so it implements [`prost::Message`] but not `Msg`, and nothing else records
/// which fragment.
pub(crate) trait Oneof {
    /// One entry per proto oneof this type stands for; several when one Rust type serves unified
    /// messages, as [`Msg::NAMES`] does.
    const ONEOF: &'static [&'static str];
}

/// Whether `declared` covers `path`.
///
/// An empty list is unchecked, exactly as an empty [`Shape::names`] is: it is what `item::salvage`
/// emits for a type whose expansion failed, and that type already has a `compile_error!` next to
/// it. Firing here too would be the cascade the stub exists to prevent.
pub(crate) const fn oneof_matches(declared: &'static [&'static str], path: &str) -> bool {
    declared.is_empty() || names_contain(declared, path)
}

/// What tokens cannot prove about a field carrying a oneof: that its type stands for *that* oneof.
///
/// A tag-compatible substitution is a byte-level bijection, so neither the round-trip harness nor
/// the field-information ratchet can see it: six filter families share one shape, which makes six
/// ways to name the wrong one and still pass every test.
pub(crate) const fn assert_oneof<T: Oneof>(path: &'static str) {
    assert!(
        oneof_matches(<T as Oneof>::ONEOF, path),
        "the field's type does not stand for this oneof",
    );
}

/// What tokens cannot prove about a `#[armonik(transparent)]` struct: that the message it names is
/// the one its delegate stands for.
///
/// A transparent struct is wire-identical to its single field, so its declared message *is* the
/// delegate's, one to one. The delegate validates its own fields; nothing else validates that the
/// two agree, which leaves `submitter::{cancel_tasks, list_tasks, count_tasks, list_sessions}` as
/// four near-identical files differing only in that name.
pub(crate) const fn assert_transparent_message<T: Msg>(name: &'static str) {
    assert!(
        names_contain(<T as Msg>::NAMES, name),
        "the delegate does not implement the message this transparent struct names",
    );
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

    // Repeated forms: the trait's unpacked defaults (messages never pack).
}

/// Custom codec for a field whose Rust representation differs structurally from its proto
/// counterpart (`#[armonik(with = "...")]`). Implementations are zero-sized marker types.
pub(crate) trait ProtoAdapter<T> {
    fn encode_field(tag: u32, value: &T, buf: &mut impl BufMut);
    fn merge_field(
        wire_type: WireType,
        value: &mut T,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>;
    fn encoded_len_field(tag: u32, value: &T) -> usize;

    /// Project the field at `tag` of a dynamic message onto the equivalence classes this adapter's
    /// Rust representation defines (for the differential harness; see
    /// `crate::differential::Normalize`). The default is the identity: adapters that only
    /// restructure the wire representation lose nothing.
    #[cfg(test)]
    fn normalize_dynamic(message: &mut ::prost_reflect::DynamicMessage, tag: u32) {
        let _ = (message, tag);
    }
}

/// An empty length-delimited field: what a `#[armonik(present)]` message marker encodes, and the
/// zero of any other length-delimited kind (an empty string or `bytes`) when no value is held to
/// encode from.
pub(crate) mod empty_body {
    use prost::bytes::{Buf, BufMut};
    use prost::encoding::{self, DecodeContext, WireType};
    use prost::DecodeError;

    pub(crate) fn encode(tag: u32, buf: &mut impl BufMut) {
        encoding::encode_key(tag, WireType::LengthDelimited, buf);
        encoding::encode_varint(0, buf);
    }

    pub(crate) fn encoded_len(tag: u32) -> usize {
        encoding::key_len(tag) + 1
    }

    /// Consume a body whose contents carry nothing, rejecting what prost's message codec rejects:
    /// the wire type, the length framing, and the keys inside. Skipping the field instead would
    /// accept a marker spelled as a varint, which no other implementation does.
    pub(crate) fn merge(
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
        encoding::merge_loop(&mut (), buf, ctx, |_, buf, ctx| {
            let (tag, wire_type) = encoding::decode_key(buf)?;
            encoding::skip_field(wire_type, tag, buf, ctx)
        })
    }
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
