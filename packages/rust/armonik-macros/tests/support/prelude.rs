// Crate-root surface for the compile-fail cases, included at the top of each one.
//
// The derives emit paths rooted at `crate::`, because they are only ever used from inside
// `armonik`. A `trybuild` case is its own crate, so it has to provide those roots itself. This is a
// stand-in, not a copy: a failed expansion still emits the real impls for whatever resolved (with
// placeholder bodies at the poisoned scopes), so everything those impls name is here as an inert
// mock (every assert passes, every body is unimplemented), and no logic is duplicated, so there is
// nothing that can silently disagree with `armonik::codec`. A signature change there breaks these
// cases loudly, which is the right signal.
//
// The const-assert classes (a field whose Rust type has the wrong shape, a stale descriptor
// fingerprint, an rpc line naming the wrong request or response type) are deliberately absent:
// their messages live in `armonik::codec` and fire at const-eval against the real trait impls,
// which a stand-in cannot host without copying the codec wholesale. They are a different mechanism
// from the expansion-time diagnostics this suite pins.

// `include!` splices this in as items, so the `allow`s ride on the items themselves rather than on
// the crate.
fn main() {}

/// The descriptor fingerprint the derives' staleness tripwire compares against, written by
/// `build.rs` next to the fixture descriptor set it belongs to.
#[allow(dead_code)]
pub mod __schema {
    include!(concat!(env!("OUT_DIR"), "/fixture_fingerprint.rs"));
}

/// The registry entry every successful expansion emits. `armonik` collects these into a
/// `linkme` slice for its differential harness; here there is nothing to collect.
#[allow(unused_macros)]
macro_rules! register {
    ($($ignored:tt)*) => {};
}
#[allow(unused_imports)]
pub(crate) use register;

#[allow(dead_code)]
pub mod codec {
    /// Reached through the blanket `ProtoField` impl by any message-shaped type. No bounds, unlike
    /// the real one (`Msg: prost::Message + Default`): a failing case's item rarely derives
    /// `Default`, and dropping the bound keeps each snapshot about the diagnostic it was written
    /// for. The blind spot that buys, on purpose: in `armonik` a message with no `Default` is an
    /// error of its own, which these cases cannot show.
    pub trait Msg {
        const NAMES: &'static [&'static str];
    }

    /// The identity of an embedded oneof, asserted by the struct carrying it.
    pub trait Oneof {
        const ONEOF: &'static [&'static str];
    }

    /// The per-instantiation field table of a generic type.
    pub trait GenericFields {
        const FIELDS: &'static [(u32, Shape)];
    }

    /// What a field's type is asked for. Everything is defaulted so a leaf impl is one line: the
    /// emitted code only has to *resolve*, never to run.
    pub trait ProtoField: Sized {
        const SHAPE: Shape;

        fn encode_field(_tag: u32, _value: &Self, _buf: &mut impl ::prost::bytes::BufMut) {
            unimplemented!()
        }
        fn encode_implicit(_tag: u32, _value: &Self, _buf: &mut impl ::prost::bytes::BufMut) {
            unimplemented!()
        }
        fn encoded_len_field(_tag: u32, _value: &Self) -> usize {
            unimplemented!()
        }
        fn encoded_len_implicit(_tag: u32, _value: &Self) -> usize {
            unimplemented!()
        }
        fn merge_field(
            _wire_type: ::prost::encoding::WireType,
            _value: &mut Self,
            _buf: &mut impl ::prost::bytes::Buf,
            _ctx: ::prost::encoding::DecodeContext,
        ) -> Result<(), ::prost::DecodeError> {
            unimplemented!()
        }
        fn is_zero(_value: &Self) -> bool {
            unimplemented!()
        }
        fn encode_repeated(_tag: u32, _values: &[Self], _buf: &mut impl ::prost::bytes::BufMut) {
            unimplemented!()
        }
        fn encoded_len_repeated(_tag: u32, _values: &[Self]) -> usize {
            unimplemented!()
        }
        fn merge_repeated(
            _wire_type: ::prost::encoding::WireType,
            _values: &mut ::std::vec::Vec<Self>,
            _buf: &mut impl ::prost::bytes::Buf,
            _ctx: ::prost::encoding::DecodeContext,
        ) -> Result<(), ::prost::DecodeError> {
            unimplemented!()
        }
    }

    /// The leaf types the fixture messages use.
    impl ProtoField for i32 {
        const SHAPE: Shape = Shape::enumeration(&[]);
    }
    impl ProtoField for String {
        const SHAPE: Shape = Shape::enumeration(&[]);
    }
    impl ProtoField for bool {
        const SHAPE: Shape = Shape::enumeration(&[]);
    }

    pub struct Shape {
        names: &'static [&'static str],
    }

    impl Shape {
        pub const fn enumeration(names: &'static [&'static str]) -> Self {
            Shape { names }
        }
    }

    /// The shape assert's vocabulary. The mock assert passes whatever it is handed: the ui suite
    /// pins expansion-time diagnostics, and the const-assert classes live with the real codec.
    pub struct Expect {
        pub kind: Option<FieldKind>,
        pub cardinalities: &'static [Cardinality],
        pub name: Option<&'static str>,
        pub map: Option<(FieldKind, FieldKind)>,
    }

    pub enum FieldKind {
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

    pub enum Cardinality {
        Singular,
        Optional,
        Repeated,
        Map,
    }

    pub const fn shape_matches(_shape: &Shape, _expect: &Expect) -> bool {
        true
    }

    pub const fn assert_oneof<T>(_path: &str) {}

    pub const fn assert_transparent_message<T>(_name: &str) {}

    /// The wire functions a plain enumeration's `ProtoField` impl delegates to.
    pub mod enumeration {
        use ::prost::bytes::{Buf, BufMut};
        use ::prost::encoding::{DecodeContext, WireType};
        use ::prost::DecodeError;

        pub fn encode<T>(_tag: u32, _value: &T, _buf: &mut impl BufMut) {
            unimplemented!()
        }
        pub fn merge<T>(
            _wire_type: WireType,
            _value: &mut T,
            _buf: &mut impl Buf,
            _ctx: DecodeContext,
        ) -> Result<(), DecodeError> {
            unimplemented!()
        }
        pub fn encoded_len<T>(_tag: u32, _value: &T) -> usize {
            unimplemented!()
        }
        pub fn encode_repeated<T>(_tag: u32, _values: &[T], _buf: &mut impl BufMut) {
            unimplemented!()
        }
        pub fn encoded_len_repeated<T>(_tag: u32, _values: &[T]) -> usize {
            unimplemented!()
        }
        pub fn merge_repeated<T>(
            _wire_type: WireType,
            _values: &mut Vec<T>,
            _buf: &mut impl Buf,
            _ctx: DecodeContext,
        ) -> Result<(), DecodeError> {
            unimplemented!()
        }
    }

    /// The codec substitutions a `present` marker picks.
    pub mod adapters {
        pub trait ProtoAdapter<T> {
            fn encode_field(_tag: u32, _value: &T, _buf: &mut impl ::prost::bytes::BufMut) {
                unimplemented!()
            }
            fn encoded_len_field(_tag: u32, _value: &T) -> usize {
                unimplemented!()
            }
            fn merge_field(
                _wire_type: ::prost::encoding::WireType,
                _value: &mut T,
                _buf: &mut impl ::prost::bytes::Buf,
                _ctx: ::prost::encoding::DecodeContext,
            ) -> Result<(), ::prost::DecodeError> {
                unimplemented!()
            }
        }

        pub struct BoolPresence;
        impl ProtoAdapter<()> for BoolPresence {}

        pub struct EmptyPresence;
        impl ProtoAdapter<()> for EmptyPresence {}
    }

    // The emitter spells the adapter dispatch `crate::codec::ProtoAdapter<T>`.
    pub use adapters::ProtoAdapter;
}
