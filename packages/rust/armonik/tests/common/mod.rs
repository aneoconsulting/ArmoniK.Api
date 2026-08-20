//! [`rpc_tests!`]: everything one RPC needs, declared once.
//!
//! Each of the twelve per-service suites is a single `rpc_tests!` block. From one case per RPC it
//! emits the fake server, and two pairs of tests:
//!
//! * `<rpc>::mock::{call, convenience}` drives the RPC against `ArmoniK.Api.Mock`, which CI starts
//!   (see `scripts/mock_test.sh`) and points `GrpcClient__Endpoint` at, once per TLS configuration.
//!   The mock counts calls per service and method and serves the tally at `/calls.json`, so this
//!   pair checks the call reached the RPC it was aimed at, over a real connection. Nothing is
//!   asserted about the response: the mock answers with stub data.
//! * `<rpc>::in_process::{call, convenience}` drives it against the generated fake over an
//!   in-memory channel, and asserts what comes back.
//!
//! In both pairs, `call` goes through `ServiceClient::call` with a hand-built request (a message,
//! or a stream of them under the streaming-request kinds) and `convenience` goes through the client
//! method in `client/<svc>.rs`. The `convenience:` clauses pin those signatures: they name the
//! method, its arguments and their order, so a signature cannot move without editing the case. What
//! pins the *mapping* -- which argument reaches which field, the one thing the types cannot say when
//! two parameters share one -- is the unary fake asserting that the request it received equals the
//! case's `request:` clause. Both halves send that same request, so the two clauses have to agree.
//!
//! These suites compile against `armonik` from the outside, so they also stand as the proof that
//! the public API is usable: everything they touch has to be `pub`, which no in-crate test could
//! establish.

// Each test binary uses a different subset of these helpers, so every one of them is unused
// somewhere.
#![allow(unused)]

use std::collections::HashMap;

/// Honour the `wait`/`failure` knobs the fake carries. Sleeping first is what the cancellation
/// tests hang their timeouts on.
///
/// Separate from [`stub`] because the streaming fakes apply the knobs per item, from inside a
/// `try_stream!` block that has no response to hand back.
pub(crate) async fn knobs(
    duration: Option<tokio::time::Duration>,
    failure: Option<tonic::Status>,
) -> Result<(), tonic::Status> {
    if let Some(duration) = duration {
        tokio::time::sleep(duration).await;
    }
    match failure {
        Some(failure) => Err(failure),
        None => Ok(()),
    }
}

/// [`knobs`], then the response: what a unary fake does.
pub(crate) async fn stub<Response>(
    duration: Option<tokio::time::Duration>,
    failure: Option<tonic::Status>,
    response: impl FnOnce() -> Result<Response, tonic::Status>,
) -> Result<Response, tonic::Status> {
    knobs(duration, failure).await?;
    response()
}

/// Apply a `respond:` clause. A closure that is called directly gets no expected signature, so its
/// parameter type cannot be inferred; going through a generic `FnOnce` bound is what lets the
/// clauses be written with no type annotation on them. Same for [`project`], [`check`] and
/// [`check_async`].
pub(crate) fn respond<Request, Response>(
    request: Request,
    respond: impl FnOnce(Request) -> Response,
) -> Response {
    respond(request)
}

/// Apply a `project:` clause.
pub(crate) fn project<Value, Projected>(
    value: Value,
    projection: impl FnOnce(Value) -> Projected,
) -> Projected {
    projection(value)
}

/// Run a `check:` clause.
pub(crate) fn check<Value>(value: Value, check: impl FnOnce(Value)) {
    check(value)
}

/// [`check`], for the `async` closure a `server_stream` RPC needs.
pub(crate) async fn check_async<Value, Fut: std::future::Future<Output = ()>>(
    value: Value,
    check: impl FnOnce(Value) -> Fut,
) {
    check(value).await
}

/// Drive a server-streaming response to its end, so the call completes instead of being dropped
/// half-read.
pub(crate) async fn drain<S, T>(
    outcome: Result<S, armonik::client::RequestError>,
) -> Result<(), armonik::client::RequestError>
where
    S: futures::Stream<Item = Result<T, armonik::client::RequestError>>,
{
    futures::TryStreamExt::try_collect::<Vec<_>>(outcome?).await?;
    Ok(())
}

/// The `mock_error:` default: no failure is acceptable.
pub(crate) fn no_error(_: &tonic::Status) -> bool {
    false
}

/// The counterpart of [`drain`] for a response that is already a value, so that the mock pair can
/// settle either kind through one call.
pub(crate) async fn keep<T>(
    outcome: Result<T, armonik::client::RequestError>,
) -> Result<T, armonik::client::RequestError> {
    outcome
}

/// Accept the outcome of one call against the mock. A few RPCs answer its stub data with a gRPC
/// failure, which still counts as the call having landed; `accepted` says which failure that RPC
/// may give, and defaults to none.
pub(crate) fn accept<T>(
    outcome: Result<T, armonik::client::RequestError>,
    accepted: fn(&tonic::Status) -> bool,
) {
    match outcome {
        Ok(_) => {}
        Err(armonik::client::RequestError::Grpc { source, .. }) => {
            assert!(accepted(&source), "unexpected failure: {source:?}");
        }
        Err(err) => panic!("unexpected failure: {err:?}"),
    }
}

/// The method name `/calls.json` files an RPC under, taken from the `Rpc` identity `service!`
/// validates against the descriptor, so a test cannot end up watching the wrong counter. It reads
/// the name off a request *value* rather than a type parameter, which is what lets a case spell the
/// request out once.
pub(crate) fn method_of<R: armonik::rpc::Rpc>(_: &R) -> &'static str {
    R::METHOD
}

/// [`method_of`], for the item type of a client-streaming request.
pub(crate) fn method_of_stream<S>(_: &S) -> &'static str
where
    S: futures::Stream,
    S::Item: armonik::rpc::Rpc,
{
    <S::Item as armonik::rpc::Rpc>::METHOD
}

/// Whether there is a mock server to talk to.
///
/// `test.yml` exports `GrpcClient__Endpoint` in each of its six TLS steps, and it is the one
/// variable every armonik client and [`nb_requests`] reads, so its absence is how a bare
/// `cargo test` says the cross-implementation half has nothing to drive. Skipping beats `#[ignore]`,
/// whose `--include-ignored` cargo forwards to rustdoc's harness too, where it compiles the
/// `ignore`-marked examples that cannot compile by construction.
///
/// A skip prints `ok`, so nothing distinguishes it from a run. Fine locally; under CI it would let
/// the one check that is not this crate reading its own descriptor evaporate on a renamed step, so
/// there the absence is a failure.
pub(crate) fn mock_is_live() -> bool {
    if matches!(std::env::var("GrpcClient__Endpoint"), Ok(endpoint) if !endpoint.is_empty()) {
        return true;
    }
    assert!(
        std::env::var_os("CI").is_none(),
        "GrpcClient__Endpoint is unset under CI, so the mock half of the suite would report `ok` \
         without running. See .github/workflows/test.yml and scripts/mock_test.sh."
    );
    false
}

/// The mock's tally of calls to one RPC. Driven through the same connector `armonik-transport`
/// builds for the gRPC channel, so it follows the endpoint and TLS configuration the suite is
/// running under.
async fn nb_requests(service: &str, method: &str) -> usize {
    let mut config = armonik::ClientConfig::from_env().expect("client configuration");

    // The mock serves `/calls.json` over plain HTTP on its own port in some configurations, and
    // alongside gRPC in others.
    match std::env::var("Http__Endpoint") {
        Ok(value) if !value.is_empty() => {
            config.endpoint = hyper::Uri::try_from(value).expect("HTTP endpoint");
        }
        Ok(_) | Err(std::env::VarError::NotPresent) => {}
        Err(std::env::VarError::NotUnicode(value)) => {
            panic!("{value:?} is not a valid unicode string")
        }
    }

    let request = hyper::Request::get(format!("{}calls.json", config.endpoint))
        .body(http_body_util::Empty::<&[u8]>::new())
        .expect("Request");

    let https = armonik::transport::https_connector(config)
        .await
        .expect("connection information");
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(https);

    let response = client.request(request).await.expect("/calls.json");
    let body = http_body_util::BodyExt::collect(response)
        .await
        .expect("Response")
        .to_bytes();

    serde_json::from_slice::<HashMap<String, HashMap<String, usize>>>(body.as_ref())
        .expect("Invalid JSON response")[service][method]
}

/// The `/calls.json` tally for one RPC, read before the call so that [`Counter::assert_one_call`]
/// can check the delta afterwards.
pub(crate) struct Counter {
    service: &'static str,
    method: &'static str,
    before: usize,
}

impl Counter {
    pub(crate) async fn read(service: &'static str, method: &'static str) -> Self {
        let before = nb_requests(service, method).await;
        Self {
            service,
            method,
            before,
        }
    }

    pub(crate) async fn assert_one_call(self) {
        let after = nb_requests(self.service, self.method).await;
        // Saturating: a mock restarted mid-suite counts from zero again, and the assertion below is
        // a better report of that than a subtraction overflow.
        assert_eq!(
            after.saturating_sub(self.before),
            1,
            "expected exactly one call to {}.{}",
            self.service,
            self.method,
        );
    }
}

/// One block per RPC. See [the module docs](self).
///
/// ```ignore
/// rpc_tests! {
///     client: into_partitions;
///     server: PartitionsService, partitions_server;
///     mock: "Partitions";
///
///     rpc unary get {
///         request: armonik::partitions::get::Request {
///             partition_id: String::from("rpc-get-input"),
///         },
///         respond: |request| armonik::partitions::get::Response {
///             partition: armonik::partitions::Raw {
///                 partition_id: request.partition_id,
///                 parent_partition_ids: vec![String::from("rpc-get-output")],
///                 ..Default::default()
///             },
///         },
///         convenience: get("rpc-get-input"),
///         project: |response| response.partition,
///         check: |partition| {
///             assert_eq!(partition.partition_id, "rpc-get-input");
///             assert_eq!(partition.parent_partition_ids[0], "rpc-get-output");
///         },
///     }
/// }
/// ```
///
/// The header says how to reach the service: the `Client` accessor, the server
/// trait with its `Router` builder, and the name the mock files its counters
/// under. That last is the name of the C# class implementing it, so not always
/// the proto service name (`Authentication`, `HealthChecks`), and `mock: none;`
/// drops the mock pair for a service the mock does not implement. The `Client`
/// accessor doubles as the `serial_test` group, since the two halves of a pair
/// share one counter and would race.
///
/// Then one `rpc` line per RPC, the keyword after it being the call kind
/// (`unary`, `client_stream` for a stream of request messages, `server_stream`
/// for a stream of responses, `bidi_stream` for both). Clauses, in this order:
///
/// * `request:` is driven through `call`: a request message, or a stream of them
///   under the two streaming-request kinds. Written as a struct literal, which
///   is also where the macro reads the request type from to emit the handler
///   signature.
/// * `respond:` is the fake's answer, and generates the handler. Omit it when the
///   handler cannot be a plain function of one request, and hand-write that
///   method in the `manual { .. }` block instead; every streaming kind does.
/// * `convenience:` is the client method in `client/<svc>.rs` and the arguments
///   to call it with.
/// * `project:` is what that method pulls out of the response. Omit it when the
///   method hands back the whole response, which is always the case under
///   `bidi_stream`. Under `server_stream` it projects one item.
/// * `check:` asserts on the projected value, and so is shared by both halves of
///   the in-process pair. Under the two response-streaming kinds it takes the
///   stream and reads `|stream| async move { .. }`.
/// * `mock_error:` is optional and only concerns the mock pair; see
///   [`accept`].
///
/// A `fake { .. }` clause adds fields to the generated `Service`, for the knobs
/// the hand-written tests drive it with; `failure` and `wait` are always there.
macro_rules! rpc_tests {
    (
        client: $into:ident;
        server: $trait:ident, $server:ident;
        mock: $mock:tt;
        $( fake { $($field:ident: $field_ty:ty),* $(,)? } )?
        $( rpc $kind:ident $name:ident { $($case:tt)* } )*
        $( manual { $($manual:item)* } )?
    ) => {
        /// The fake this suite's in-process pairs run against, generated from the `respond:`
        /// clauses below.
        #[derive(Debug, Clone, Default)]
        struct Service {
            #[allow(dead_code)]
            failure: Option<tonic::Status>,
            #[allow(dead_code)]
            wait: Option<tokio::time::Duration>,
            $($(
                #[allow(dead_code)]
                $field: $field_ty,
            )*)?
        }

        impl armonik::server::$trait for Service {
            $( rpc_tests!(@handler $kind $name { $($case)* }); )*
            $($( $manual )*)?
        }

        $(
            mod $name {
                #[allow(unused_imports)]
                use super::*;

                rpc_tests!(@pairs $kind $name, $into, $server, $mock, { $($case)* });
            }
        )*
    };

    // ---- the fake's handler, for a `respond:` that is a plain function of the ---- request.
    // Anything else is hand-written in `manual { .. }`.

    (@handler unary $name:ident {
        request: $($request_ty:ident)::+ { $($request_fields:tt)* },
        respond: $respond:expr,
        $($rest:tt)*
    }) => {
        async fn $name(
            self: std::sync::Arc<Self>,
            request: $($request_ty)::+,
            _context: armonik::server::RequestContext,
        ) -> Result<<$($request_ty)::+ as armonik::rpc::Rpc>::Response, tonic::Status> {
            // What ties a convenience method's parameters to the fields they have to reach. The
            // `convenience:` clause pins arity and types; only this pins the mapping, so a swapped
            // pair of same-typed arguments -- the one mistake the type system cannot see -- builds
            // a different request and fails here. Both halves of the pair send this same request by
            // construction, so there is nothing else for the fake to compare against.
            assert_eq!(
                request,
                $($request_ty)::+ { $($request_fields)* },
                "the convenience method built a different request"
            );
            crate::common::stub(self.wait, self.failure.clone(), || {
                Ok(crate::common::respond(request, $respond))
            })
            .await
        }
    };

    (@handler $kind:ident $name:ident { $($case:tt)* }) => {};

    // ---- both pairs, per call kind ----
    //
    // Each arm is grammar: it names the clauses its kind accepts, then hands the two halves off.
    // What the halves do differs along one axis only, whether the response is a value or a stream,
    // so the bodies live in the two `@in_process` arms below rather than once per kind.

    (@pairs unary $name:ident, $into:ident, $server:ident, $mock:tt, {
        request: $($request_ty:ident)::+ { $($request_fields:tt)* },
        $( respond: $respond:expr, )?
        convenience: $method:ident ( $($arg:expr),* $(,)? ),
        $( project: $project:expr, )?
        check: $check:expr,
        $( mock_error: $mock_error:expr, )?
    }) => {
        rpc_tests!(@mock $mock, $into, method_of,
            request = ($($request_ty)::+ { $($request_fields)* }),
            call = (call),
            convenience = ($method($($arg),*)),
            settle = (keep),
            mock_error = ($($mock_error)?),
        );
        rpc_tests!(@in_process value $into, $server,
            request = ($($request_ty)::+ { $($request_fields)* }),
            convenience = ($method($($arg),*)),
            project = ($($project)?),
            check = ($check),
        );
    };

    (@pairs server_stream $name:ident, $into:ident, $server:ident, $mock:tt, {
        request: $($request_ty:ident)::+ { $($request_fields:tt)* },
        convenience: $method:ident ( $($arg:expr),* $(,)? ),
        $( project: $project:expr, )?
        check: $check:expr,
        $( mock_error: $mock_error:expr, )?
    }) => {
        rpc_tests!(@mock $mock, $into, method_of,
            request = ($($request_ty)::+ { $($request_fields)* }),
            call = (call),
            convenience = ($method($($arg),*)),
            settle = (drain),
            mock_error = ($($mock_error)?),
        );
        rpc_tests!(@in_process stream $into, $server,
            request = ($($request_ty)::+ { $($request_fields)* }),
            convenience = ($method($($arg),*)),
            project = ($($project)?),
            check = ($check),
        );
    };

    (@pairs client_stream $name:ident, $into:ident, $server:ident, $mock:tt, {
        request: $request:expr,
        convenience: $method:ident ( $($arg:expr),* $(,)? ),
        $( project: $project:expr, )?
        check: $check:expr,
        $( mock_error: $mock_error:expr, )?
    }) => {
        rpc_tests!(@mock $mock, $into, method_of_stream,
            request = ($request),
            call = (call),
            convenience = ($method($($arg),*)),
            settle = (keep),
            mock_error = ($($mock_error)?),
        );
        rpc_tests!(@in_process value $into, $server,
            request = ($request),
            convenience = ($method($($arg),*)),
            project = ($($project)?),
            check = ($check),
        );
    };

    // Both halves at once: the request is a stream, like `client_stream`, and the response is one,
    // like `server_stream`. No `@handler` arm goes with it, for the same reason the other two
    // streaming kinds have none: a fake that answers a stream with a stream is not a function of
    // one request, so it goes in `manual { .. }`.
    (@pairs bidi_stream $name:ident, $into:ident, $server:ident, $mock:tt, {
        request: $request:expr,
        convenience: $method:ident ( $($arg:expr),* $(,)? ),
        $( project: $project:expr, )?
        check: $check:expr,
        $( mock_error: $mock_error:expr, )?
    }) => {
        rpc_tests!(@mock $mock, $into, method_of_stream,
            request = ($request),
            call = (call),
            convenience = ($method($($arg),*)),
            settle = (drain),
            mock_error = ($($mock_error)?),
        );
        rpc_tests!(@in_process stream $into, $server,
            request = ($request),
            convenience = ($method($($arg),*)),
            project = ($($project)?),
            check = ($check),
        );
    };

    // ---- the in-process pair, by response shape ----

    (@in_process value $into:ident, $server:ident,
        request = ($request:expr),
        convenience = ($method:ident($($arg:expr),*)),
        project = ($($project:expr)?),
        check = ($check:expr),
    ) => {
        mod in_process {
            #[allow(unused_imports)]
            use super::*;

            #[tokio::test]
            async fn call() {
                let mut client = rpc_tests!(@client $into, $server);
                let value = client.call($request).await.unwrap();
                $( let value = crate::common::project(value, $project); )?
                crate::common::check(value, $check);
            }

            #[tokio::test]
            async fn convenience() {
                let mut client = rpc_tests!(@client $into, $server);
                let value = client.$method($($arg),*).await.unwrap();
                crate::common::check(value, $check);
            }
        }
    };

    (@in_process stream $into:ident, $server:ident,
        request = ($request:expr),
        convenience = ($method:ident($($arg:expr),*)),
        project = ($($project:expr)?),
        check = ($check:expr),
    ) => {
        mod in_process {
            #[allow(unused_imports)]
            use super::*;

            #[tokio::test]
            async fn call() {
                let mut client = rpc_tests!(@client $into, $server);
                let stream = client.call($request).await.unwrap();
                // Boxed on both halves, so the shared check sees one type: `call` projects through
                // an adaptor, and a hand-written convenience method returns `impl Stream`.
                let stream = futures::StreamExt::boxed(futures::StreamExt::map(stream, |item| {
                    Result::map(item, |response| {
                        $( let response = crate::common::project(response, $project); )?
                        response
                    })
                }));
                crate::common::check_async(stream, $check).await;
            }

            #[tokio::test]
            async fn convenience() {
                let mut client = rpc_tests!(@client $into, $server);
                let stream = client.$method($($arg),*).await.unwrap();
                crate::common::check_async(futures::StreamExt::boxed(stream), $check).await;
            }
        }
    };

    (@pairs $kind:ident $($rest:tt)*) => {
        ::core::compile_error!(::core::concat!(
            "unknown rpc kind `",
            ::core::stringify!($kind),
            "`: expected `unary`, `client_stream`, `server_stream` or `bidi_stream`",
        ));
    };

    // ---- the mock pair, dropped entirely for a service the mock lacks ----

    (@mock none, $($rest:tt)*) => {};

    (@mock $mock:literal, $into:ident, $method_of:ident,
        request = ($request:expr),
        call = ($call:ident),
        convenience = ($method:ident($($arg:expr),*)),
        settle = ($settle:ident),
        mock_error = ($($mock_error:expr)?),
    ) => {
        mod mock {
            #[allow(unused_imports)]
            use super::*;

            /// The failure this RPC may answer the mock's stub data with.
            fn accepted() -> fn(&tonic::Status) -> bool {
                rpc_tests!(@accepted $($mock_error)?)
            }

            // Both cases below need the C# mock server, which `scripts/mock_test.sh` starts and
            // points `GrpcClient__Endpoint` at. Without it each would fail with `InvalidUri(Empty)`,
            // which makes a plain `cargo test` a poor signal: half of every service suite failing
            // for a reason that has nothing to do with the code. So they skip on the variable the
            // script exports (see `mock_is_live`), and CI needs no flag to run them.
            //
            // The two differ only in how the RPC is reached, so both name a method of the client.
            rpc_tests!(@mock_test call, $into, $mock, $method_of, ($request), $settle,
                       $call($request));
            rpc_tests!(@mock_test convenience, $into, $mock, $method_of, ($request), $settle,
                       $method($($arg),*));
        }
    };

    (@mock_test $test:ident, $into:ident, $mock:tt, $method_of:ident, ($request:expr),
        $settle:ident, $invoke:ident($($arg:expr),*)
    ) => {
        #[tokio::test]
        #[serial_test::serial($into)]
        async fn $test() {
            if !crate::common::mock_is_live() {
                return;
            }
            let counter =
                crate::common::Counter::read($mock, crate::common::$method_of(&$request)).await;
            let mut client = armonik::Client::new().await.unwrap().$into();
            let outcome = crate::common::$settle(client.$invoke($($arg),*).await).await;
            crate::common::accept(outcome, accepted());
            counter.assert_one_call().await;
        }
    };

    // ---- shared pieces ----

    (@accepted) => { crate::common::no_error };
    (@accepted $mock_error:expr) => { $mock_error };

    (@client $into:ident, $server:ident) => {
        armonik::Client::with_channel(Service::default().$server()).$into()
    };
}
