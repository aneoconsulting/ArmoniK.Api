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

    /// The same decode without the unwrap, for the malformed-input case: prost validates
    /// UTF-8 on decode, so it is the reference for what a conformant parser does.
    pub fn decode_res(b: &[u8]) -> Result<p::ListResultsResponse, prost::DecodeError> {
        p::ListResultsResponse::decode(b)
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
        core_native::encode_list_results_response(v)
    }

    pub fn decode(b: &[u8]) -> ListResultsResponse {
        core_native::decode_list_results_response(b).expect("core-native decode")
    }
}

/// Arm `core-ffi-rust`: the same core, reached through the C ABI from a Rust host.
/// **The interface cost with the runtime tax removed** (README section 4.1).
pub mod core_ffi_arm {
    use crate::generated::binding;
    use ak_abi::{ak_enc_ctx, ak_enc_ctx_free,
                 ak_enc_ctx_new};
    use facade::ListResultsResponse;

    /// The contexts a host holds across calls. Creating one per call would put an
    /// allocation and a fresh (unlearned) length-width table inside every measurement.
    pub struct Ctx {
        pub enc: *mut ak_enc_ctx,
        pub dec: binding::DecCtxs,
        pub tcs: binding::Tcs,
    }

    impl Ctx {
        pub fn new() -> Self {
            // The host links the codec, so a missing symbol is a load failure rather than
            // a wrong value in a field (ABI v1 section 10).
            assert_eq!(crate::abi_version(), ak_abi::AK_ABI_VERSION);
            // ABI v1 section 3: every entry point requires `ak_init`, the codec included.
            // Called here rather than in each `main` because that is the shape a host
            // LIBRARY has -- it initialises defensively on its own first use and must not
            // fail because something else got there first, which is exactly what
            // `AK_ALREADY_INITIALIZED` being a success is for. Idempotent under identical
            // options, so every context after the first takes that path.
            //
            // With `init-guard` off this changes nothing the codec does; with it on it is
            // what makes the codec usable at all, and the difference between the two
            // builds is what section 3's rule costs.
            // R-G7: the call is RENDERED into the binding from `plan.lifecycle`, so it is
            // the same call in every binding the generator emits.
            let rc = binding::ak_init_once();
            assert!(rc >= 0, "ak_init failed: rc {rc}");
            unsafe {
                Ctx { enc: ak_enc_ctx_new(), dec: binding::DecCtxs::new(), tcs: binding::Tcs::trusted() }
            }
        }
        pub fn validating() -> Self {
            let mut c = Self::new();
            c.tcs = binding::Tcs::validating();
            c
        }
        pub fn validating_simd() -> Self {
            let mut c = Self::new();
            c.tcs = binding::Tcs::validating_simd();
            c
        }
    }

    impl Drop for Ctx {
        fn drop(&mut self) {
            unsafe {
                ak_enc_ctx_free(self.enc);
                self.dec.free();
            }
        }
    }

    pub fn value(pid: &str) -> ListResultsResponse {
        super::armonik_arm::value(pid)
    }

    /// Returns the bytes the context holds, which stay valid until the next encode.
    pub fn encode_into<'a>(c: &'a Ctx, v: &ListResultsResponse) -> &'a [u8] {
        binding::encode_into_list_results_response(c.enc, v, &c.tcs).expect("core-ffi encode");
        unsafe { binding::encoded(c.enc) }
    }

    pub fn encode(c: &Ctx, v: &ListResultsResponse) -> Vec<u8> {
        encode_into(c, v).to_vec()
    }

    pub fn decode(c: &Ctx, b: &[u8]) -> ListResultsResponse {
        binding::decode_with_list_results_response(c.dec, b).expect("core-ffi decode")
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

/// Two no-boundary variants of `core-native`, added to audit a claim rather than to make a
/// new one.
///
/// **The claim being audited.** The P1.3 inversion (`core-ffi-rust` above `core-native` on
/// the absent path) was attributed to the by-value group, with a per-element interface cost
/// obtained by subtracting `core-native` from `core-ffi-rust`. That subtraction bundles two
/// things: "crossed a boundary and materialised a 200-byte group" and "was not inlined".
/// `core-native` is compiled into the harness and the FFI arm cannot be, so the second term
/// was being charged to the first.
///
/// **It is not a hypothetical.** In the built `bench` binary there are ZERO call sites to
/// `facade::generated::core_native::encode_into_list_results_response` and zero to
/// `decode_list_results_response`: both bodies are fused into the benchmark closure. The
/// exported symbols exist only because the functions are `pub`. `gen/inline_check.sh` shows
/// this from the artifact rather than asserting it.
///
/// So two variants, at the granularity the FFI call sits at -- the per-message entry point,
/// not the inner per-element traversal:
///
/// | arm | what it stops | what it still allows |
/// |---|---|---|
/// | `core-native-noinline` | inlining into the loop | rustc still reasons about the body: same crate, argument attributes, no aliasing surprises. A **lower bound** on the penalty |
/// | `core-native-opaque` | inlining, devirtualisation and constant propagation across the call | nothing. The pointer has been through `black_box`. The closer model of a dynamic call into a cdylib, **minus the group** |
///
/// Neither changes the codec: both call the same generated traversal, so any byte
/// difference from `core-native` would be a defect in this module and nothing else.
pub mod core_native_noinline {
    use ak_rt::Enc;
    use facade::generated::core_native;
    use facade::ListResultsResponse;

    #[inline(never)]
    pub fn encode_into(o: &ListResultsResponse, e: &mut Enc) {
        core_native::encode_into_list_results_response(o, e)
    }

    #[inline(never)]
    pub fn decode_res(b: &[u8]) -> Result<ListResultsResponse, i32> {
        core_native::decode_list_results_response(b)
    }

    pub fn encode(o: &ListResultsResponse) -> Vec<u8> {
        let mut e = Enc::new(core_native::SITES);
        encode_into(o, &mut e);
        e.buf.clone()
    }

    pub fn decode(b: &[u8]) -> ListResultsResponse {
        decode_res(b).expect("core-native-noinline decode")
    }
}

pub mod core_native_opaque {
    use ak_rt::Enc;
    use facade::generated::core_native;
    use facade::ListResultsResponse;

    pub type EncFn = fn(&ListResultsResponse, &mut Enc);
    pub type DecFn = fn(&[u8]) -> Result<ListResultsResponse, i32>;

    /// The entry point as a value the optimiser cannot see through. `black_box` is an empty
    /// inline-asm block that consumes and returns the pointer, so LLVM must treat the result
    /// as unknown: it cannot inline through it, cannot devirtualise it back to the known
    /// callee, and cannot propagate anything about the arguments across it. Taken ONCE,
    /// outside every timed region.
    pub fn enc_fn() -> EncFn {
        std::hint::black_box(core_native::encode_into_list_results_response as EncFn)
    }

    pub fn dec_fn() -> DecFn {
        std::hint::black_box(core_native::decode_list_results_response as DecFn)
    }

    pub fn encode(o: &ListResultsResponse) -> Vec<u8> {
        let mut e = Enc::new(core_native::SITES);
        (enc_fn())(o, &mut e);
        e.buf.clone()
    }

    pub fn decode(b: &[u8]) -> ListResultsResponse {
        (dec_fn())(b).expect("core-native-opaque decode")
    }
}

/// ABI v1 open decision 9 candidate: the ZEROED-GROUP variant, built as an arm.
///
/// The element-group array is zeroed once per chunk with a memset and the host then
/// assigns only the fields that are not at their default, instead of the total fill
/// section 6 requires. **This reverses a trade the ABI already made and priced**: the total
/// fill was bought so the codec never resets the group between elements, worth 5.4 ns per
/// `ResultRaw` and 24.4 per `TaskDetailed`. So it pays that back on every element of every
/// payload to save the scattered stores on the empty ones, and the question is where the
/// cross-over sits relative to real traffic.
///
/// The generator emits it ALONGSIDE the default and nothing the default path uses changed.
/// Only the TOP-LEVEL element group is built this way; a nested loop inside an element
/// keeps the total fill.
pub mod core_ffi_zeroed {
    use super::core_ffi_arm::Ctx;
    use crate::generated::binding;
    use facade::ListResultsResponse;

    pub fn encode_into<'a>(c: &'a Ctx, v: &ListResultsResponse) -> &'a [u8] {
        binding::encode_into_list_results_response_zeroed(c.enc, v, &c.tcs)
            .expect("core-ffi-zeroed encode");
        unsafe { binding::encoded(c.enc) }
    }

    pub fn encode(c: &Ctx, v: &ListResultsResponse) -> Vec<u8> {
        encode_into(c, v).to_vec()
    }
}

/// ABI v1 open decision 11 candidate: the unknown-field bag, as an arm.
///
/// Decision 11 says the core retains nothing while `Google.Protobuf`, protobuf-java,
/// protobuf C++ and upb all retain, so adopting the core removes a protobuf guarantee from
/// four of the five languages. This prices the fix.
///
/// **The bag is ONE opaque `bytes` blob per message, not a list, and that is the whole
/// design decision.** `gen/unknown_predicate.py` runs ABI v1 7.2's batching predicate --
/// the one the generator uses, imported, not re-derived -- over the schema with a bag added
/// under each modelling: a REPEATED bag takes the schema from 9 leaf messages to 0 and the
/// batched run fails everywhere, including `ResultRaw`, which is what turns 1000 rows into
/// 9 crossings. One blob changes nothing.
///
/// On encode the host hands the blob back as one `ak_str` in the group and the codec
/// appends it verbatim; empty is `tc == NULL`, the existing absent convention. On decode the
/// unknown runs are captured as spans into the buffer the host handed in and delivered as a
/// side run keyed by token, so the core copies nothing, allocates nothing, and the element
/// group is not touched at all.
#[cfg(feature = "unknown-fields")]
pub mod core_ffi_unk {
    use super::core_ffi_arm::Ctx;
    use crate::generated::binding;
    use facade::ListResultsResponse;

    pub fn encode_into<'a>(c: &'a Ctx, v: &ListResultsResponse) -> &'a [u8] {
        binding::encode_into_list_results_response_unk(c.enc, v, &c.tcs).expect("unk encode");
        unsafe { binding::encoded(c.enc) }
    }
    pub fn encode(c: &Ctx, v: &ListResultsResponse) -> Vec<u8> {
        encode_into(c, v).to_vec()
    }
    /// Decisions 9 and 11 together: the zeroed fill over the group that carries the bag.
    pub fn encode_into_zeroed<'a>(c: &'a Ctx, v: &ListResultsResponse) -> &'a [u8] {
        binding::encode_into_list_results_response_unk_zeroed(c.enc, v, &c.tcs).expect("unk/zeroed encode");
        unsafe { binding::encoded(c.enc) }
    }
    pub fn decode(c: &Ctx, b: &[u8]) -> ListResultsResponse {
        binding::decode_with_list_results_response_unk(c.dec, b).expect("unk decode")
    }
}
