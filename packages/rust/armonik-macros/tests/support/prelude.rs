// Crate-root surface for the compile-fail cases, included at the top of each one.
//
// The derives emit paths rooted at `crate::`, because they are only ever used from inside
// `armonik`. A `trybuild` case is its own crate, so it has to provide those roots itself. This is a
// stand-in, not a copy: only the shapes the *failing* expansions name are here, and no logic is
// duplicated, so there is nothing that can silently disagree with `armonik::codec`. A signature
// change there breaks these cases loudly, which is the right signal.
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
    /// Reached through the blanket `ProtoField` impl by any message-shaped type.
    pub trait Msg: ::prost::Message + Default {
        const NAMES: &'static [&'static str];
    }

    /// What a field's type is asked for. Only `SHAPE` and the three singular methods are named by
    /// the stubs a failed expansion emits.
    pub trait ProtoField: Sized {
        const SHAPE: Shape;

        fn encode_field(tag: u32, value: &Self, buf: &mut impl ::prost::bytes::BufMut);
        fn merge_field(
            wire_type: ::prost::encoding::WireType,
            value: &mut Self,
            buf: &mut impl ::prost::bytes::Buf,
            ctx: ::prost::encoding::DecodeContext,
        ) -> Result<(), ::prost::DecodeError>;
        fn encoded_len_field(tag: u32, value: &Self) -> usize;
    }

    pub struct Shape {
        names: &'static [&'static str],
    }

    impl Shape {
        pub const fn enumeration(names: &'static [&'static str]) -> Self {
            Shape { names }
        }
    }

    /// Enough of a leaf impl for a case whose message has a scalar field. The codec keys on the
    /// Rust type, so `i32` is what every 32-bit integer kind resolves to.
    impl ProtoField for i32 {
        const SHAPE: Shape = Shape::enumeration(&[]);

        fn encode_field(_tag: u32, _value: &Self, _buf: &mut impl ::prost::bytes::BufMut) {
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
        fn encoded_len_field(_tag: u32, _value: &Self) -> usize {
            unimplemented!()
        }
    }

    /// An embedded oneof, carried by a field of the message that owns it.
    pub trait ProtoOneof: Sized {
        fn encode_oneof(value: &Self, buf: &mut impl ::prost::bytes::BufMut);
        fn merge_oneof(
            tag: u32,
            wire_type: ::prost::encoding::WireType,
            value: &mut Self,
            buf: &mut impl ::prost::bytes::Buf,
            ctx: ::prost::encoding::DecodeContext,
        ) -> Result<(), ::prost::DecodeError>;
        fn encoded_len_oneof(value: &Self) -> usize;
    }
}
