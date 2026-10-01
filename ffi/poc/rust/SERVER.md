# The campaign's RPC server: the interface every slice codes against

FIX-PLAN WP10, CAMPAIGN req 13 as amended at 9f6d579fa (the owner): the Rust slice's tonic
`rpc_server` is THE RPC server of every slice, so the server's work is the same whatever
the client language and no JIT runs in its warm-up. This file is its interface, kept stable;
`poc/rust/serve.sh` starts, warms and stops it. Source: `crates/campaign/src/server.rs`
(the service) and `src/bin/rpc_server.rs` (the process).

## Running it

```
poc/rust/serve.sh build            # once: rpc_server + rpc_warm, release, poc/rust/target-server/
poc/rust/serve.sh start --out DIR  # prints:  shipped /tmp/aksrv.XXXXXX/shipped.sock
                                   #          pinned  /tmp/aksrv.XXXXXX/pinned.sock
                                   #          pid     N
                                   #          tcp     127.0.0.1:PORT   (only with AK_SERVER_TCP)
poc/rust/serve.sh warm N           # N checked calls per direction, tonic + core clients, both sockets
poc/rust/serve.sh stop
```

- ONE process per launch, pinned to `AK_CPU_SERVER` (taskset; unpinned if unset), serving
  both configurations on two Unix domain sockets in a short `mktemp -d /tmp/aksrv.XXXXXX`
  directory (a socket path must fit `sun_path`, 107 bytes). Dial `unix:<path>`.
- Log: `DIR/rpc-server.log` (one line per socket: configuration, path, P2.2 size, workers,
  receive limit, pid). State: `$AK_SERVE_STATE` (default `${TMPDIR:-/tmp}/ak-rpc-server.state`).
- `AK_SERVER_THREADS`: the tokio multi-thread runtime's worker count, shared by both sockets
  (default 4, the campaign's SERVER set size). Stated in the log.
- `serve.sh warm N`: on each socket, N checked Fetch (a), Push (b) and Upload (c) calls and
  ceil(N/4) UploadStream (d, 4 MiB) calls, from a tonic client with prost and from the core's
  client with prost. Exit 3 on the first failed check.

## Optional TCP listener (loopback)

`AK_SERVER_TCP=PORT serve.sh start --out DIR` (PORT 0 = any free port) also serves the same
service on `127.0.0.1:PORT`, from the same process, runtime and workers, always in the **pinned**
configuration (4 MiB stream and connection windows, adaptive window off, the same 8 MiB receive
limit), with `TCP_NODELAY` set on every accepted socket. `start` prints and the state file holds
one more line, `tcp 127.0.0.1:PORT`; dial `http://127.0.0.1:PORT`. `serve.sh warm N` then also
warms it (pinned clients). Unset, the server is Unix sockets only, as before.

## Server configurations

| Socket | Configuration |
|---|---|
| `shipped` | tonic's server defaults (what an ArmoniK peer would see) |
| `pinned` | initial stream window 4 MiB, initial connection window 4 MiB, adaptive window off |

Nagle does not apply to a Unix socket. Compression is off on every path. Both answer the
same methods.

## Service and methods

Service `armonik.ffi.campaign.v1.Grid`, raw gRPC over HTTP/2 (5-byte length-prefixed
frames, no compression). Messages are the shapes schema's (`ffi/schema/shapes.json`), in
protobuf wire form; the server decodes requests with prost.

| Direction | Method path | Kind | Request | Response |
|---|---|---|---|---|
| a | `/armonik.ffi.campaign.v1.Grid/Fetch` | unary | ignored (send `Empty`, 0 bytes) | P2.2: a `ListTasksDetailedResponse`, **pre-serialised once at start-up** with prost and checked against the payload manifest's SHA-256; 540,422 bytes, identical on every call |
| a, planted | `/armonik.ffi.campaign.v1.Grid/FetchShort` | unary | ignored | P2.2 **one byte short** (540,421 bytes): the planted wrong response, below |
| b | `/armonik.ffi.campaign.v1.Grid/Push` | unary | a `ListTasksDetailedResponse` (P2.2), decoded with prost; must have at least one task | empty (0 bytes); INVALID_ARGUMENT if it does not decode or has no task |
| c | `/armonik.ffi.campaign.v1.Grid/Upload` | unary | an `UploadResultDataMessage` (M5): P5.3 (1,048,660 B) or P5.4 (4,194,390 B), decoded with prost; `upload.data_chunk` must be non-empty | empty; INVALID_ARGUMENT otherwise |
| d | `/armonik.ffi.campaign.v1.Grid/UploadStream` | client streaming | a stream of `UploadResultDataMessage`, one per 2 MiB chunk (4 MiB = 2 messages, 16 MiB = 8), each decoded with prost | 8 bytes: the total `data_chunk` byte count, u64 little-endian |
| d, check | `/armonik.ffi.campaign.v1.Grid/UploadStreamCheck` | client streaming | the same stream | 40 bytes: the byte count (u64 LE), then the SHA-256 of every message's bytes **as received**, in order (a correctness check, never timed) |

Messages (shapes.json): `ListTasksDetailedResponse { repeated TaskDetailed tasks = 1; int32
page = 2; int32 total = 3; }`; `UploadResultDataMessage { UploadResultData upload = 1; }`;
`UploadResultData { string session_id = 1; string result_id = 2; bytes data_chunk = 3; }`.

**The ids-on-first-message rule (d):** the first message of the stream must carry non-empty
`session_id` and `result_id`, or the call fails with INVALID_ARGUMENT; later messages carry
empty ids (ArmoniK's UploadResultData shape) and only `data_chunk` (2 MiB of data each).
Every message must carry `upload`. The Rust slice's stream: ids `session-u2` / `result-u2`
on message 0, 2,097,152 data bytes per message (a wire message of 2 MiB + 57 bytes).

## Limits

| Limit | Value |
|---|---|
| receive (server decoding) | 8 MiB (8,388,608 B) per message, every path: covers P5.4 (4,194,390 B) and a stream message (2 MiB + 57 B); a 16 MiB stream is 8 messages, and a stream has no total limit |
| send (server encoding) | tonic's default, unlimited; the largest response is P2.2, 540,422 B |

A client's own receive limit must admit 540,422 B (tonic's default 4 MiB does); its send
limit must admit 4,194,390 B for direction c (tonic's default is unlimited).

## Requirement 18: a planted wrong response

For a slice's control that a wrong response aborts the run with no sample, either:

- call `/armonik.ffi.campaign.v1.Grid/FetchShort` in place of Fetch: the server answers P2.2
  one byte short, so a client checking the length (540,422) must fail; or
- keep the path and expect a wrong length on the client (what the Rust slice does:
  `AK_RPC_PLANT`, expected length + 1).

No environment variable changes the server's answers: the server is shared, so a plant must
be selected per call.

## Test paths (never timed)

`StatusU<n>` / `StatusS<n>` (answer gRPC status n, unary / client streaming), `SleepU` /
`SleepS` (answer after 3 s), `EchoS` (echo the request metadata) under the same service,
used by the Rust slice's `bin/rpc_semantics`. Any other method path under the service is
answered like Fetch.

## The core's two h2 variants (2026-10-01)

The core has two h2 variants. **stock** (crates.io h2 0.4.19) is every ordinary build.
**h2-batch** is h2 PR 903 ported to 0.4.19 and combined with p4, with AK_H2_COALESCE
defaulting to 16. It is opt-in and never in a default build. Build either one with
`poc/codec/h2-batch/build.sh stock|h2-batch TARGET_DIR [FEATURES]`. `poc/codec/h2-batch/README.md`
holds the patch's provenance and sha256, and says how each slice builds and loads a variant.
The server (`bin/rpc_server`) is always built stock.

Timed runs use TCP from 2026-10-01 (owner). `serve.sh` with `AK_SERVER_TCP=0` also listens on
127.0.0.1 (TCP_NODELAY on accept) and writes `tcp 127.0.0.1:PORT` in its state file. Clients
target `http://127.0.0.1:PORT`. `gen/inproc.sh` does this by default (`AK_IP_TRANSPORT=tcp`;
`uds` is the explicit option).
