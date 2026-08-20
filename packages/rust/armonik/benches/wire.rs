//! Encoding and decoding of the results messages, with no gRPC around them.
//!
//! `benches/roundtrip.rs` measures whole calls; this one isolates the codec, which is
//! the part of the stack this branch replaces. Three shapes, picked because they
//! stress different things:
//!
//! * `list_response`: many small fields and a repeated nested message, so the
//!   per-field work dominates;
//! * `download_chunk`: one large `bytes` field, so the payload copy dominates;
//! * `get_response`: the smallest realistic unary response, which is what
//!   decomposes the unary round trip.
//!
//! # Running it
//!
//! Pin to one physical core, both its SMT siblings, and to two cores for
//! `roundtrip.rs`, whose tokio runtime has two workers:
//!
//! ```sh
//! taskset -c 4,14      cargo bench -p armonik --bench wire
//! taskset -c 4,14,5,15 cargo bench -p armonik --bench roundtrip
//! ```
//!
//! Over 8 alternating runs that cut the run-to-run deviation from 7.0% to 2.4% on
//! `encode/download_chunk` and from 1.2% to 0.5% on `encode/get_response`, moving the
//! medians by 0.1%. Pinning to a single logical CPU is worse than not pinning at all,
//! since it leaves the sibling free for other work. Compare configurations interleaved
//! run by run and never in blocks: this machine drifts by more than most of the
//! differences below, and blocks put that drift inside the comparison.
//!
//! # Measurements
//!
//! Three points of this branch in one pinned campaign, 4 interleaved rounds on a
//! stable desktop (i9-7900X, no virtualisation), medians of the per-round medians,
//! per-row deviation 0.3% to 4.4%. `base` is the branch point, where the ergonomic
//! types are not `prost::Message`: its copy of this file converts to the generated
//! `api::v3` type first, which is what a call did there, and its copy of
//! `roundtrip.rs` drops the `watch` handler, that RPC being younger than the base.
//!
//! | | base | before the leaf skip | now | now vs base |
//! |---|---|---|---|---|
//! | `results/get` | 2.672 us | 2.903 us | 2.658 us | -0.5% |
//! | `results/rotate/1` | 2.599 us | 2.925 us | 2.660 us | +2.4% |
//! | `results/rotate/2` | 2.633 us | 2.915 us | 2.616 us | -0.6% |
//! | `results/rotate/4` | 2.734 us | 2.942 us | 2.616 us | -4.3% |
//! | `results/rotate/7` | 3.104 us | 3.316 us | 3.055 us | -1.6% |
//! | `wire/encode/get_response` | 144 ns | 146 ns | 133 ns | -7.8% |
//! | `wire/decode/get_response` | 222 ns | 268 ns | 231 ns | +4.4% |
//! | `wire/encode/list_response` | 4.36 us | 4.49 us | 4.02 us | -8.0% |
//! | `wire/decode/list_response` | 9.70 us | 10.81 us | 9.71 us | +0.1% |
//! | `wire/encode/download_chunk` | 1.70 us | 1.71 us | 1.72 us | +1.3% |
//! | `wire/decode/download_chunk` | 1.82 us | 31 ns | 33 ns | -98% |
//!
//! So the whole call is at parity with the generated types it replaced, and every
//! codec row is at parity or better except the unary decode, +4.4%, which is one
//! branch misprediction (below). The `before the leaf skip` to `now` step is what
//! leaving a zero off the wire bought: -8% to -14% on the eight rows built from small
//! fields, -7.9% on `rotate/7`, and nothing on the two `download_chunk` rows (+0.6% and
//! +6.5%), whose one large `bytes` field has no zero to leave off and whose absolute
//! numbers, 31 and 33 ns for the decode, are where run-to-run noise lives. Half of the
//! rest is simply fewer bytes: 53 against 59 for the get response and 1766 against 1960
//! for the list, 9.9% off the list, and 53 is what the generated types wrote too.
//! Re-measure both figures with the fixtures below when either moves.
//!
//! ## Why the call rows need `rotate`
//!
//! The generated design emits a specialised, fully inlined server stub per RPC: 11,475
//! instructions of machine code for `GetResult` alone, all of tonic's header, codec and
//! trailer work folded in with its constants. This crate reaches the same tonic code
//! generically, once per call shape.
//!
//! That makes a single-RPC benchmark the generated design's best case, and it flatters
//! it enough to invert the conclusion. With only `get` implemented and the other
//! handlers answering `unimplemented`, `base` measured 2.478 us against this crate's
//! 2.668, a +7.7% regression. Implementing the other six handlers, without calling
//! them, cost `base` 7.8% (2.478 to 2.672) and cost this crate nothing (2.668 to
//! 2.658): seven stubs in one `call()` are worse than one, whatever runs. Rotating over
//! several RPCs, which is what a server does, holds that parity from two upwards.
//!
//! Generating specialised handlers here was measured and dropped. Direct dispatch (a
//! generated `if` chain instead of the routing table's `fn` pointer) plus
//! `#[inline(always)]` on the four `serve_*` bodies removes 1510 instructions a call,
//! 6%, and buys 29 cycles of 10,611: this path is not instruction-bound, and the
//! variant measured 1.3% slower. Removing the per-RPC tracing span (-528 instructions)
//! and `#[inline(always)]` alone were likewise flat.
//!
//! # Where the remaining gap sits
//!
//! Per iteration, `perf stat` over pinned `--profile-time` runs.
//!
//! `wire/decode/get_response` is +46 cycles against base while running 81 instructions
//! *fewer*, 2880 against 2961. It pays 2.03 branch mispredictions a message where the
//! generated types pay 0.09, and 94% of them land on one instruction, the indirect
//! `jmp` of the tag jump table in `Raw::merge_field`; at ~18 cycles a miss that is 35
//! of the 46. Both codecs lower the tag match to a jump table, so what differs is only
//! how well the target sequence predicts.
//!
//! Two levers were measured there, neither taken. A comparison chain instead of a
//! `match` changes nothing: LLVM re-forms the jump tables. Forcing the lowering with
//! `-Cllvm-args=-min-jump-table-entries=64` does fix the codec (decode 207 ns, the list
//! 9.02 us, both under base) and costs the whole call +6.5%, the http/2 and tonic
//! switches it also rewrites being hotter than ours. A local fix would be an unrolled
//! in-order fast path in the generated merge, trading one-shape-per-field simplicity
//! for ~15 ns a message.
//!
//! `download_chunk` is the row to distrust: the copy is a single `rep movsb`, so the row
//! reports memory-system state, and two campaigns half an hour apart landed on opposite
//! sides of parity. The work settles it: 165 instructions an iteration here against 132
//! at base, the +33 being `Bytes::append_to`'s clone and drop, ~40 cycles of ~6400. Its
//! decode is the `Vec<u8>`-to-`Bytes` move, 33 ns against 1.82 us: `replace_with` slices
//! the payload out of the input buffer with `copy_to_bytes` instead of copying, which
//! needs the source to be a `Bytes` and not the last reference to one. No encode path can
//! consume the payload either way, since `Message::encode` takes `&self`.

use armonik::results;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use prost::Message;

/// Number of `Raw` entries in the `list_response` fixture.
const RESULTS: usize = 32;

/// Size of the `download_chunk` payload.
const CHUNK: usize = 80 * 1024;

/// A `Raw` with its stable fields filled in.
///
/// Field-by-field rather than a struct literal: `Raw` gains and loses fields
/// along this branch and only these are stable throughout.
#[allow(clippy::field_reassign_with_default)]
fn raw(index: usize) -> results::Raw {
    let mut raw = results::Raw::default();
    raw.session_id = "bench-session".to_owned();
    raw.result_id = format!("result-{index}");
    raw.name = format!("name-{index}");
    raw.owner_task_id = format!("task-{index}");
    raw.created_by = "bench".to_owned();
    raw.size = 4096;
    raw
}

fn list_response() -> results::list::Response {
    results::list::Response {
        results: (0..RESULTS).map(raw).collect(),
        page: 0,
        page_size: RESULTS as i32,
        total: RESULTS as i32,
    }
}

/// The `GetResult` response: one nested `Raw`, the shape of a small unary reply.
fn get_response() -> results::get::Response {
    results::get::Response { result: raw(0) }
}

fn download_chunk() -> results::download::Response {
    results::download::Response {
        data_chunk: vec![0xa5; CHUNK].into(),
    }
}

// ---- the codec path under measurement ----

fn encode_list(value: results::list::Response) -> Vec<u8> {
    value.encode_to_vec()
}

fn decode_list(buffer: bytes::Bytes) -> results::list::Response {
    results::list::Response::decode(buffer).expect("decode a list response")
}

fn encode_get(value: results::get::Response) -> Vec<u8> {
    value.encode_to_vec()
}

fn decode_get(buffer: bytes::Bytes) -> results::get::Response {
    results::get::Response::decode(buffer).expect("decode a get response")
}

fn encode_chunk(value: results::download::Response) -> Vec<u8> {
    value.encode_to_vec()
}

fn decode_chunk(buffer: bytes::Bytes) -> results::download::Response {
    results::download::Response::decode(buffer).expect("decode a data chunk")
}

// ---- benchmarks ----

fn encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("wire/encode");

    let value = list_response();
    group.throughput(Throughput::Bytes(encode_list(value.clone()).len() as u64));
    group.bench_function("list_response", |b| {
        // The value is taken by value here only so the timed region matches the
        // pre-revamp form, whose conversion consumes it; the clone is setup and
        // is not timed.
        b.iter_batched(
            || value.clone(),
            |value| criterion::black_box(encode_list(value)),
            BatchSize::SmallInput,
        )
    });

    let value = get_response();
    group.throughput(Throughput::Bytes(encode_get(value.clone()).len() as u64));
    group.bench_function("get_response", |b| {
        b.iter_batched(
            || value.clone(),
            |value| criterion::black_box(encode_get(value)),
            BatchSize::SmallInput,
        )
    });

    // Into a buffer the iterations share, which is what tonic's encoder does and the
    // only way this row measures the codec: allocating 80 KiB inside the timed region
    // measures the state of the heap instead, and swings 5x with it.
    let wire = download_chunk();
    group.throughput(Throughput::Bytes(CHUNK as u64));
    group.bench_function("download_chunk", |b| {
        let mut buf = Vec::with_capacity(wire.encoded_len());
        b.iter(|| {
            buf.clear();
            wire.encode_raw(&mut buf);
            criterion::black_box(buf.len())
        })
    });

    group.finish();
}

fn decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("wire/decode");

    // Decoded from `Bytes` rather than a slice: that is what lets a `bytes`
    // field share the buffer instead of copying once the migration lands, so
    // the pre and post measurements differ by the codec and not by the input.
    let buffer = bytes::Bytes::from(encode_list(list_response()));
    group.throughput(Throughput::Bytes(buffer.len() as u64));
    group.bench_function("list_response", |b| {
        b.iter(|| criterion::black_box(decode_list(buffer.clone())))
    });

    let buffer = bytes::Bytes::from(encode_get(get_response()));
    group.throughput(Throughput::Bytes(buffer.len() as u64));
    group.bench_function("get_response", |b| {
        b.iter(|| criterion::black_box(decode_get(buffer.clone())))
    });

    let buffer = bytes::Bytes::from(encode_chunk(download_chunk()));
    group.throughput(Throughput::Bytes(CHUNK as u64));
    group.bench_function("download_chunk", |b| {
        b.iter(|| criterion::black_box(decode_chunk(buffer.clone())))
    });

    group.finish();
}

criterion_group!(benches, encode, decode);
criterion_main!(benches);
