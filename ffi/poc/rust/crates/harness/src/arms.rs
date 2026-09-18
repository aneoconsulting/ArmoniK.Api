//! One entry per arm, so the conformance run and the benchmarks drive the same code.
//!
//! What each arm's byte identity is being checked against, stated because two of them are
//! NOT independent (README R2, and the coordinator's stage 2 note):
//!
//! | arm | objects it encodes | codec | independent of |
//! |---|---|---|---|
//! | `prost` | prost-build structs, built by the hand-written builder in `stage1-validate` | prost 0.14.4 | everything else here |
//! | `armonik` | facade types, built by the GENERATED builder | generated `prost::Message` impls | the prost arm's objects and its impls |
//! | `core-native` | the same facade values as `armonik` | the generated core traversal, no boundary | prost entirely |
//! | `core-ffi-rust` | the same facade values | **the same core traversal**, reached through the C ABI | NOT independent of `core-native`: they share the codec. What differs is only where the values come from |
//!
//! So `prost` against `armonik` is two independent encoders over two independently built
//! object graphs, and both are checked against the manifest, which is itself validated
//! against a third (prost-reflect). `core-native` against `core-ffi-rust` is one encoder
//! reached two ways, and it proves the BINDING, not the codec.

use prost::Message;

pub const P1_1: &str = "P1.1";
pub const P1_2: &str = "P1.2";
pub const P1_3: &str = "P1.3";

/// Arm `prost`: prost-build's structs and prost's codec. Today's floor.
pub mod prost_arm {
    use super::*;
    use shapes_prost::shapes as p;

    pub fn value(pid: &str) -> p::ListResultsResponse {
        match pid {
            super::P1_1 => build_list(4, false),
            super::P1_2 => build_list(1000, false),
            super::P1_3 => build_list(300, true),
            _ => unimplemented!("{pid}"),
        }
    }

    pub fn encode(v: &p::ListResultsResponse) -> Vec<u8> {
        v.encode_to_vec()
    }

    pub fn decode(b: &[u8]) -> p::ListResultsResponse {
        p::ListResultsResponse::decode(b).unwrap()
    }

    // The hand-written builder of stage 1, kept here so arm `prost` constructs its objects
    // by a different route from the facade arms.
    fn build_list(count: i64, absent: bool) -> p::ListResultsResponse {
        p::ListResultsResponse {
            results: (0..count).map(|j| result_raw(j, absent)).collect(),
            page: 1,
            total: count as i32,
        }
    }

    fn result_raw(idx: i64, absent: bool) -> p::ResultRaw {
        use shapes_values as v;
        if absent {
            return p::ResultRaw::default();
        }
        let (s, n) = v::timestamp(idx);
        p::ResultRaw {
            session_id: v::guid("ResultRaw.session_id", idx),
            name: v::word("ResultRaw.name", idx),
            owner_task_id: v::guid("ResultRaw.owner_task_id", idx),
            status: v::enum_value(&v::RESULT_STATUS, idx),
            created_at: Some(p::Timestamp { seconds: s, nanos: n }),
            completed_at: Some(p::Timestamp { seconds: s, nanos: n }),
            result_id: v::guid("ResultRaw.result_id", idx),
            size: v::scalar_i64("ResultRaw.size", idx),
            created_by: v::guid("ResultRaw.created_by", idx),
            opaque_id: v::blob("ResultRaw.opaque_id", idx, 16),
            manual_deletion: v::scalar_bool("ResultRaw.manual_deletion", idx),
        }
    }
}

/// Arm `core-native`: the no-boundary control (README R3). The same generated traversal
/// the core behind the C ABI runs, emitted into the host, over the same facade values.
pub mod core_native_arm {
    use facade::generated::core_native;
    use facade::ListResultsResponse;

    pub fn value(pid: &str) -> ListResultsResponse {
        super::armonik_arm::value(pid)
    }

    pub fn encode(v: &ListResultsResponse) -> Vec<u8> {
        core_native::encode(v)
    }

    pub fn decode(b: &[u8]) -> ListResultsResponse {
        core_native::decode(b).expect("core-native decode")
    }
}

/// Arm `core-ffi-rust`: the same core, reached through the C ABI from a Rust host.
/// **The interface cost with the runtime tax removed** (README section 4.1).
pub mod core_ffi_arm {
    use crate::generated::binding;
    use ak_abi::{ak_dec_ctx, ak_dec_ctx_free, ak_dec_ctx_new, ak_enc_ctx, ak_enc_ctx_free,
                 ak_enc_ctx_new};
    use facade::ListResultsResponse;

    /// The contexts a host holds across calls. Creating one per call would put an
    /// allocation and a fresh (unlearned) length-width table inside every measurement.
    pub struct Ctx {
        pub enc: *mut ak_enc_ctx,
        pub dec: *mut ak_dec_ctx,
        pub tcs: binding::Tcs,
    }

    impl Ctx {
        pub fn new() -> Self {
            // The host links the codec, so a missing symbol is a load failure rather than
            // a wrong value in a field (ABI v1 section 10).
            assert_eq!(crate::abi_version(), ak_abi::AK_ABI_VERSION);
            unsafe {
                Ctx { enc: ak_enc_ctx_new(), dec: ak_dec_ctx_new(), tcs: binding::Tcs::trusted() }
            }
        }
        pub fn validating() -> Self {
            let mut c = Self::new();
            c.tcs = binding::Tcs::validating();
            c
        }
    }

    impl Drop for Ctx {
        fn drop(&mut self) {
            unsafe {
                ak_enc_ctx_free(self.enc);
                ak_dec_ctx_free(self.dec);
            }
        }
    }

    pub fn value(pid: &str) -> ListResultsResponse {
        super::armonik_arm::value(pid)
    }

    /// Returns the bytes the context holds, which stay valid until the next encode.
    pub fn encode_into<'a>(c: &'a Ctx, v: &ListResultsResponse) -> &'a [u8] {
        binding::encode_into(c.enc, v, &c.tcs).expect("core-ffi encode");
        unsafe { binding::encoded(c.enc) }
    }

    pub fn encode(c: &Ctx, v: &ListResultsResponse) -> Vec<u8> {
        encode_into(c, v).to_vec()
    }

    pub fn decode(c: &Ctx, b: &[u8]) -> ListResultsResponse {
        binding::decode_with(c.dec, b).expect("core-ffi decode")
    }
}

/// Arm `armonik`: the facade types, with generated `prost::Message` impls and no
/// conversion layer. The bet packages/rust already made.
pub mod armonik_arm {
    use super::*;
    use facade::ListResultsResponse;

    pub fn value(pid: &str) -> ListResultsResponse {
        match pid {
            super::P1_1 => facade::build::payload_p1_1(),
            super::P1_2 => facade::build::payload_p1_2(),
            super::P1_3 => facade::build::payload_p1_3(),
            _ => unimplemented!("{pid}"),
        }
    }

    pub fn encode(v: &ListResultsResponse) -> Vec<u8> {
        v.encode_to_vec()
    }

    pub fn decode(b: &[u8]) -> ListResultsResponse {
        ListResultsResponse::decode(b).unwrap()
    }
}
