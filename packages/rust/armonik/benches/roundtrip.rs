//! Round-trip latency and throughput of the results service, in process.
//!
//! The server is a [`ResultsService`] implementation handed straight to
//! [`Client::with_channel`]: the tonic service *is* the channel, so no socket,
//! DNS or TLS is involved and the numbers are the gRPC codec plus the client and
//! server glue.
//!
//! One RPC per call kind, all on `Results`:
//!
//! * unary, `GetResult`: round-trip latency of a small message;
//! * server streaming, `DownloadResultData`: throughput, swept over payload size;
//! * client streaming, `UploadResultData`: throughput, swept over payload size.
//!
//! Each of them builds one client and clones it into every iteration, so the
//! channel is reused and what is timed is the call rather than the building of a
//! gRPC stack; `client_construction` measures that separately.
//!
//! Everything goes through the public API: `client.call(request)` for the unary
//! and server-streaming kinds, `client.upload(..)` for the client-streaming
//! one.
//!
//! The unbenchmarked RPCs answer with `Default::default()`, which is what lets
//! `rotate` call them without the file naming response fields it does not
//! measure. Only `watch` is left `unimplemented`.

use std::sync::Arc;

use armonik::results;
use armonik::server::{RequestContext, ResultsService, ResultsServiceExt};
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use futures::StreamExt;

/// Chunk size the streaming benchmarks cut their payload into. The value the
/// deployed services advertise through `GetServiceConfiguration`.
const CHUNK: usize = 80 * 1024;

/// Payload sizes swept by the two streaming benchmarks: one chunk, then enough
/// chunks that the per-call overhead stops dominating the per-byte cost.
const SIZES: [usize; 3] = [CHUNK, 8 * CHUNK, 64 * CHUNK];

const SESSION: &str = "bench-session";
const RESULT: &str = "bench-result";

/// The service under benchmark. `payload` is what `download` streams back, cut
/// into [`CHUNK`]-sized pieces; `upload` counts what it receives so the work is
/// not optimised away.
#[derive(Clone, Default)]
struct Bench {
    payload: prost::bytes::Bytes,
}

impl ResultsService for Bench {
    async fn get(
        self: Arc<Self>,
        _request: results::get::Request,
        _context: RequestContext,
    ) -> Result<results::get::Response, tonic::Status> {
        let result = results::Raw {
            session_id: SESSION.to_owned(),
            result_id: RESULT.to_owned(),
            name: RESULT.to_owned(),
            ..Default::default()
        };

        Ok(results::get::Response { result })
    }

    async fn download(
        self: Arc<Self>,
        _request: results::download::Request,
        _context: RequestContext,
    ) -> Result<
        impl futures::Stream<Item = Result<results::download::Response, tonic::Status>> + Send,
        tonic::Status,
    > {
        // `slice`, not a copy: `data_chunk` is `bytes::Bytes`, so each chunk is a view into the
        // one payload buffer and the serving side of the measurement allocates nothing.
        let payload = &self.payload;
        let chunks = (0..payload.len())
            .step_by(CHUNK)
            .map(|offset| {
                let end = (offset + CHUNK).min(payload.len());
                Ok(results::download::Response {
                    data_chunk: payload.slice(offset..end),
                })
            })
            .collect::<Vec<_>>();

        Ok(futures::stream::iter(chunks))
    }

    async fn upload(
        self: Arc<Self>,
        request: impl futures::Stream<Item = Result<results::upload::Request, tonic::Status>>
            + Send
            + 'static,
        _context: RequestContext,
    ) -> Result<results::upload::Response, tonic::Status> {
        // Counted rather than matched: the variants of `upload::Request` are
        // not the measurement, and not naming them keeps this compiling
        // across the oneof rework.
        let mut received = 0usize;
        let mut request = std::pin::pin!(request);
        while let Some(item) = request.next().await {
            item?;
            received += 1;
        }
        criterion::black_box(received);

        Ok(results::upload::Response::default())
    }

    // ---- not benchmarked ----

    async fn list(
        self: Arc<Self>,
        _request: results::list::Request,
        _context: RequestContext,
    ) -> Result<results::list::Response, tonic::Status> {
        Ok(Default::default())
    }

    async fn get_owner_task_id(
        self: Arc<Self>,
        _request: results::get_owner_task_id::Request,
        _context: RequestContext,
    ) -> Result<results::get_owner_task_id::Response, tonic::Status> {
        Ok(Default::default())
    }

    async fn create_metadata(
        self: Arc<Self>,
        _request: results::create_metadata::Request,
        _context: RequestContext,
    ) -> Result<results::create_metadata::Response, tonic::Status> {
        Ok(Default::default())
    }

    async fn create(
        self: Arc<Self>,
        _request: results::create::Request,
        _context: RequestContext,
    ) -> Result<results::create::Response, tonic::Status> {
        Ok(Default::default())
    }

    async fn import(
        self: Arc<Self>,
        _request: results::import::Request,
        _context: RequestContext,
    ) -> Result<results::import::Response, tonic::Status> {
        Ok(Default::default())
    }

    async fn delete_data(
        self: Arc<Self>,
        _request: results::delete_data::Request,
        _context: RequestContext,
    ) -> Result<results::delete_data::Response, tonic::Status> {
        Ok(Default::default())
    }

    async fn get_service_configuration(
        self: Arc<Self>,
        _request: results::get_service_configuration::Request,
        _context: RequestContext,
    ) -> Result<results::get_service_configuration::Response, tonic::Status> {
        Ok(Default::default())
    }

    // Turbofished: a method that returns `impl Stream` still has to name a concrete one, even where
    // it only ever fails, because the error arm alone leaves nothing to infer it from.
    async fn watch(
        self: Arc<Self>,
        _request: impl futures::Stream<Item = Result<results::watch::Request, tonic::Status>>
            + Send
            + 'static,
        _context: RequestContext,
    ) -> Result<
        impl futures::Stream<Item = Result<results::watch::Response, tonic::Status>> + Send,
        tonic::Status,
    > {
        Err::<futures::stream::Empty<_>, _>(unimplemented())
    }
}

/// `watch` is the one RPC the benchmark refuses rather than answers: it is bidirectional, so a
/// `Default` response would still need a stream to carry it.
fn unimplemented() -> tonic::Status {
    tonic::Status::unimplemented("not part of the benchmark")
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime")
}

/// Unary: what one small round trip costs.
fn unary(c: &mut Criterion) {
    let runtime = runtime();
    let channel = Bench::default().results_server();
    let client = armonik::Client::with_channel(channel).into_results();

    c.benchmark_group("results")
        .throughput(Throughput::Elements(1))
        .bench_function("get", |b| {
            b.to_async(&runtime).iter(|| {
                let mut client = client.clone();
                async move {
                    let response = client
                        .call(results::get::Request {
                            id: RESULT.to_owned(),
                        })
                        .await
                        .expect("get");
                    criterion::black_box(response);
                }
            })
        });
}

/// The same round trip spread over several distinct RPCs, which is what a server
/// actually serves.
///
/// One benchmark per RPC count, all on `Results`, all unary, all answering trivially:
/// what varies is how many dispatch and codec paths are hot at once. A design that
/// specialises code per RPC is at its best at one and pays as the count grows; a
/// shared generic path costs the same whatever the count. Reading the unary row alone
/// says nothing about which, so this row exists to keep that comparison honest.
fn rotate(c: &mut Criterion) {
    let runtime = runtime();
    let channel = Bench::default().results_server();
    let client = armonik::Client::with_channel(channel).into_results();

    let mut group = c.benchmark_group("results/rotate");
    group.throughput(Throughput::Elements(1));
    for rpcs in [1usize, 2, 4, 7] {
        group.bench_function(BenchmarkId::from_parameter(rpcs), |b| {
            let mut turn = 0usize;
            b.to_async(&runtime).iter(|| {
                let mut client = client.clone();
                turn = turn.wrapping_add(1);
                let which = turn % rpcs;
                async move {
                    match which {
                        0 => {
                            criterion::black_box(
                                client
                                    .call(results::get::Request {
                                        id: RESULT.to_owned(),
                                    })
                                    .await
                                    .expect("get"),
                            );
                        }
                        1 => {
                            criterion::black_box(
                                client
                                    .call(results::list::Request::default())
                                    .await
                                    .expect("list"),
                            );
                        }
                        2 => {
                            criterion::black_box(
                                client
                                    .call(results::get_owner_task_id::Request::default())
                                    .await
                                    .expect("owner task id"),
                            );
                        }
                        3 => {
                            criterion::black_box(
                                client
                                    .call(results::create_metadata::Request::default())
                                    .await
                                    .expect("create metadata"),
                            );
                        }
                        4 => {
                            criterion::black_box(
                                client
                                    .call(results::create::Request::default())
                                    .await
                                    .expect("create"),
                            );
                        }
                        5 => {
                            criterion::black_box(
                                client
                                    .call(results::delete_data::Request::default())
                                    .await
                                    .expect("delete data"),
                            );
                        }
                        _ => {
                            criterion::black_box(
                                client
                                    .call(results::import::Request::default())
                                    .await
                                    .expect("import"),
                            );
                        }
                    }
                }
            })
        });
    }
    group.finish();
}

/// What building a client costs, so the round-trip numbers can be read without
/// it. Only the in-process part: `with_channel` takes an already-connected
/// channel, so this is the gRPC stack over a ready transport and none of the
/// DNS, TCP or TLS a real endpoint would add.
fn construction(c: &mut Criterion) {
    let channel = Bench::default().results_server();

    c.benchmark_group("results")
        .throughput(Throughput::Elements(1))
        .bench_function("client_construction", |b| {
            b.iter(|| {
                criterion::black_box(armonik::Client::with_channel(channel.clone()).into_results());
            })
        });
}

/// Server streaming: bytes per second out of the server, by payload size.
fn download(c: &mut Criterion) {
    let runtime = runtime();
    let mut group = c.benchmark_group("results/download");

    for size in SIZES {
        let channel = Bench {
            payload: vec![0xa5; size].into(),
        }
        .results_server();

        let client = armonik::Client::with_channel(channel).into_results();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_function(BenchmarkId::from_parameter(size), |b| {
            b.to_async(&runtime).iter(|| {
                let mut client = client.clone();
                async move {
                    let stream = client
                        .call(results::download::Request {
                            session_id: SESSION.to_owned(),
                            result_id: RESULT.to_owned(),
                        })
                        .await
                        .expect("download");

                    let mut stream = std::pin::pin!(stream);
                    let mut received = 0usize;
                    while let Some(chunk) = stream.next().await {
                        received += chunk.expect("chunk").data_chunk.len();
                    }
                    // The row is bytes per second, so it has to be the whole payload: handing out
                    // fewer bytes would read as a faster download rather than as a broken one.
                    assert_eq!(received, size, "the whole payload came back");
                }
            })
        });
    }

    group.finish();
}

/// Client streaming: bytes per second into the server, by payload size.
fn upload(c: &mut Criterion) {
    let runtime = runtime();
    let mut group = c.benchmark_group("results/upload");

    for size in SIZES {
        let channel = Bench::default().results_server();
        // `Vec<u8>` chunks: `upload` takes `Item: Into<bytes::Bytes>`, and a `Vec<u8>` converts
        // by handing over its allocation, so the conversion inside the timed closure copies
        // nothing. The clone `iter_batched` makes per iteration does, which is why it is setup.
        let chunks = vec![vec![0xa5u8; CHUNK]; size.div_ceil(CHUNK)];

        let client = armonik::Client::with_channel(channel).into_results();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_function(BenchmarkId::from_parameter(size), |b| {
            // `iter_batched`, not `iter`: the per-iteration input is a copy of the whole payload,
            // and inside the closure it would be timed as if it were bytes on the wire.
            b.to_async(&runtime).iter_batched(
                || (client.clone(), chunks.clone()),
                |(mut client, chunks)| async move {
                    let raw = client
                        .upload(SESSION, RESULT, futures::stream::iter(chunks))
                        .await
                        .expect("upload");
                    criterion::black_box(raw);
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

criterion_group!(benches, unary, rotate, construction, download, upload);
criterion_main!(benches);
