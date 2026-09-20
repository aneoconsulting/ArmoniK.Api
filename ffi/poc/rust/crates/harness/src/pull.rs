//! Arms `core-ffi-pull` and `core-ffi-pull-walk`: ABI v1 section 7.1's **pull** family.
//!
//! The push family (`core-ffi-rust`, everywhere else in this harness) has the codec call
//! the host once per field group. The pull family has the codec deposit the same handovers
//! into the host-owned decode context, make **no reverse call at all**, and return; the
//! host then reads them.
//!
//! **Why this exists, and it is not a Rust question.** The java slice's C ABI decode is
//! 1.22 to 1.62 of protobuf-java on every M2 payload, and it decomposed the gap rather
//! than quoting it: 7.004 upcalls per element at about 80 ns is 560 ns of it. On a host
//! whose reverse call is that dear, a family that makes none is the obvious remedy -- and
//! the core did not implement one, so nobody could measure the alternative. Rust is the
//! cheapest place to build it, and the worst place to be impressed by the result: at a 1.8
//! ns crossing this slice expects pull to LOSE, and a loss here with a size attached is
//! what lets the java slice decide whether 560 ns buys more than the materialisation costs.
//!
//! **Two arms, because the copy is not the family.**
//!
//! | arm | what the host does after the parse | who does it this way |
//! |---|---|---|
//! | `core-ffi-pull` | `ak_bdr_drain` the records into its OWN memory in 32 KB chunks, replay each chunk | a managed host: the JVM under `GetPrimitiveArrayCritical`, .NET over a pinned array |
//! | `core-ffi-pull-walk` | `ak_bdr_ptr` and replay in place | a native host: C++, and Rust |
//!
//! The difference between them is exactly the drain copy, so pull's cost decomposes into
//! "materialise the records" (in both) and "copy them to the host" (in one), and a slice
//! on another runtime can price its own half rather than inheriting this one's.
//!
//! **The replay calls the push family's own host functions.** `replay_*` in the generated
//! binding dispatches each record to the same `apply_*`, `new_*`, `add_*` the push vtable
//! registers. That is the control, not an economy: if the two families deposited through
//! two different bodies of host code, a difference between the arms would be a difference
//! between two bindings rather than between two deliveries of one traversal.

use crate::arms::core_ffi_arm::Ctx;
use crate::generated::binding as b;

/// What a host holds across calls so the arm measures the family and not an allocator: the
/// chunk buffer the drain copies into, and the token map the replay builds.
///
/// `Vec<u64>` and not `Vec<u8>`: a record's payload is an `ak_dfix_*` carrying `i64` and
/// `f64`, so it has to be 8-aligned to be read as one, and `Vec<u8>` guarantees alignment 1.
pub struct PullState {
    pub scratch: Vec<u64>,
    pub toks: Vec<i64>,
}

impl Default for PullState {
    fn default() -> Self {
        Self::new()
    }
}

impl PullState {
    pub fn new() -> Self {
        // One chunk, sized at the ABI's minimum, which is section 7.3's 32 KB arena plus
        // one header. This is what section 7.1 specifies ("drain in 32 KB chunks") and
        // what a host that wants a bounded intermediate would allocate.
        Self::with_bytes(ak_rt::bdr::BDR_MIN_CHUNK)
    }

    /// A chunk of a chosen size, for the sensitivity arm. The drain's forward-crossing
    /// count is `ceil(footprint / chunk)`, so the chunk size is the one knob a host has
    /// on the family's crossing count -- and on a host where a forward call is 100 ns
    /// that is not a detail. It trades crossings against how much of the response the
    /// host holds materialised at once, which is the bound section 7.1 gives the host in
    /// the first place.
    pub fn with_bytes(bytes: usize) -> Self {
        PullState {
            scratch: vec![0u64; bytes.max(ak_rt::bdr::BDR_MIN_CHUNK).div_ceil(8)],
            toks: Vec::new(),
        }
    }

    /// Size the chunk to hold the whole record stream: one drain, one forward crossing,
    /// the whole response materialised twice at the peak.
    pub fn fit(&mut self, bytes: usize) {
        let words = bytes.max(ak_rt::bdr::BDR_MIN_CHUNK).div_ceil(8);
        if self.scratch.len() < words {
            self.scratch.resize(words, 0);
        }
    }
}

/// `ak_bdr_footprint` after the last parse: what the intermediate actually costs in bytes.
/// Reported beside the timings, because "pull materialises the whole response" is a
/// memory claim as well as a time one and the two are priced separately.
pub fn footprint(c: &Ctx) -> usize {
    unsafe { ak_abi::ak_bdr_footprint(c.dec) }
}

macro_rules! pull {
    ($drain:ident, $walk:ident, $opaque:ident, $ty:ty, $pd:ident, $pw:ident, $po:ident) => {
        /// Parse, drain into the host's own memory in chunks, replay each chunk.
        pub fn $drain(c: &Ctx, s: &mut PullState, x: &[u8]) -> $ty {
            b::$pd(c.dec, x, &mut s.scratch, &mut s.toks).expect("pull drain decode")
        }
        /// Parse, then walk the records where the core left them. No copy.
        pub fn $walk(c: &Ctx, s: &mut PullState, x: &[u8]) -> $ty {
            b::$pw(c.dec, x, &mut s.toks).expect("pull walk decode")
        }
        /// The same, with the replay's calls made opaque. R5's second half for this
        /// family: a replay is host code calling host code and can be fused into the
        /// loop; a push callback is reached through a vtable across the shared-library
        /// boundary and cannot. Without this arm, "pull is at parity with push" might be
        /// "pull's deposit was inlined and push's could not be", and the two are not the
        /// same claim.
        pub fn $opaque(c: &Ctx, s: &mut PullState, x: &[u8]) -> $ty {
            b::$po(c.dec, x, &mut s.toks).expect("pull walk-opaque decode")
        }
    };
}

pull!(
    drain_m1, walk_m1, opaque_m1, facade::ListResultsResponse,
    parse_drain_with_list_results_response, parse_walk_with_list_results_response,
    parse_walk_opaque_with_list_results_response
);
pull!(
    drain_m2, walk_m2, opaque_m2, facade::ListTasksDetailedResponse,
    parse_drain_with_list_tasks_detailed_response, parse_walk_with_list_tasks_detailed_response,
    parse_walk_opaque_with_list_tasks_detailed_response
);
pull!(
    drain_m3, walk_m3, opaque_m3, facade::ListProbeResponse,
    parse_drain_with_list_probe_response, parse_walk_with_list_probe_response,
    parse_walk_opaque_with_list_probe_response
);
pull!(
    drain_m4, walk_m4, opaque_m4, facade::ListTaskSummaryResponse,
    parse_drain_with_list_task_summary_response, parse_walk_with_list_task_summary_response,
    parse_walk_opaque_with_list_task_summary_response
);
pull!(
    drain_m5, walk_m5, opaque_m5, facade::UploadResultDataMessage,
    parse_drain_with_upload_result_data_message, parse_walk_with_upload_result_data_message,
    parse_walk_opaque_with_upload_result_data_message
);
pull!(
    drain_m6, walk_m6, opaque_m6, facade::ListMetricsResponse,
    parse_drain_with_list_metrics_response, parse_walk_with_list_metrics_response,
    parse_walk_opaque_with_list_metrics_response
);
pull!(
    drain_m7, walk_m7, opaque_m7, facade::DualResponse,
    parse_drain_with_dual_response, parse_walk_with_dual_response,
    parse_walk_opaque_with_dual_response
);
