//! Encoding and decoding of the results messages, with no gRPC around them.
//!
//! `benches/roundtrip.rs` measures whole calls; this one isolates the codec,
//! which is the part of the stack this branch replaces.
//!
//! # This is the post-revamp form
//!
//! The armonik types implement [`prost::Message`] themselves, so encoding is one
//! pass over the value and decoding is one pass into it. Before the results
//! service flipped, this file went through the generated `api::v3` mirrors and
//! the conversions either way; it was rewritten at that commit, keeping the same
//! benchmark ids and the same payloads so the two forms are comparable.
//!
//! Two shapes, picked because they stress different things:
//!
//! * `list_response`: many small fields and a repeated nested message, so the
//!   per-field work dominates;
//! * `download_chunk`: one large `bytes` field, so the payload copy dominates.

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

    let buffer = bytes::Bytes::from(encode_chunk(download_chunk()));
    group.throughput(Throughput::Bytes(CHUNK as u64));
    group.bench_function("download_chunk", |b| {
        b.iter(|| criterion::black_box(decode_chunk(buffer.clone())))
    });

    group.finish();
}

criterion_group!(benches, encode, decode);
criterion_main!(benches);
