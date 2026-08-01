//! Wire-level classification of protobuf fields, mirrored from the
//! descriptor. The `armonik::codec` module keeps an equivalent runtime
//! classification that emitted const-asserts are checked against.

/// Scalar/wire kind of a protobuf field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FieldKind {
    Double,
    Float,
    Int32,
    Int64,
    UInt32,
    UInt64,
    SInt32,
    SInt64,
    Fixed32,
    Fixed64,
    SFixed32,
    SFixed64,
    Bool,
    String,
    Bytes,
    /// Full name of the message type, without leading dot.
    Message(String),
    /// Full name of the enum type, without leading dot.
    Enum(String),
}

impl FieldKind {
    /// Whether repeated fields of this kind are packable (proto3 packs them
    /// by default).
    pub(crate) fn packable(&self) -> bool {
        !matches!(
            self,
            FieldKind::String | FieldKind::Bytes | FieldKind::Message(_)
        )
    }
}

/// Cardinality of a protobuf field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Cardinality {
    /// Singular proto3 field: implicit presence.
    Singular,
    /// `optional` proto3 field: explicit presence.
    Optional,
    /// Repeated field; `packed` is the encoding used by conforming writers.
    Repeated { packed: bool },
    /// Map field, folded from its synthetic `*Entry` message.
    Map { key: FieldKind, value: FieldKind },
}
