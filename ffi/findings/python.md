# Reading the python slice

Phase note: this file records facts only; container timings were removed on 2026-09-24 (design/FIX-PLAN.md WP2). The raw logs remain in logs/python/.

The aggregating session's reading of `poc/python`. The slice's own record is
`poc/python/STATE.md`, which is newer than this file and wins where they differ.
Every finding listed in section 8 is unconfirmed until the slice answers it.

## 1. What was built

- **A generated CPython C extension shim over the shared core** (`poc/codec`,
  reached by path, never copied; `poc/codec/gen/one_core.sh` passes). The binding
  has three layers, as README 9.1 describes: the Rust core, a generated C shim
  that speaks the CPython API, and the Python facade. The shim calls C-API
  primitives on Python objects; the core does not call Python-level code.
- **Every shape in `design/SHAPES.md`**: M1 to M7, all 16 payloads, both
  directions. Nothing in the ABI needed extending for M3 to M7: oneof
  (`<oneof>_case` carrying the active member's tag), explicit presence (a presence
  word), packed scalars (`ak_run_*`) and bulk bytes (section 8's direct argument)
  were binding work.
- **Three facade storages**: a plain class, a `__slots__` class, and a C extension
  type the shim reads as struct members. Plus a Python-level accessor arm (`pyacc`,
  the README 9.1 premise control) and a pure-Python codec generated from the same
  description (`gen/py_codec.py`, the no-boundary control).
- **The binding mechanism for the codec path is a C extension.** ctypes and cffi
  (ABI mode and API mode) were built as microbenchmark arms in work unit 1 and not
  used for the codec, because a ctypes or cffi callback routes through the
  interpreter, which is what README 9.1 avoids. PyO3 was also built as an arm.
  ctypes and cffi remain untested candidates for the RPC layer, where there are
  two crossings per call rather than one per field.
- **The RPC arm, as a three-cell grid** (the former `logs/python/80-rpc-grid.log`, withdrawn in fd5da9475 under R-C9: it ran on uncommitted code; the current grid is the campaign harness, `logs/python/campaign/`, instrumentation) against one
  grpcio server that returns pre-serialised bytes: A upb + grpcio, B upb + the
  core's transport, C the core's codec + the core's transport. Both arms decode at
  the client. Three builds of the core exist in the slice (plain, counting, and
  the `rpc`-feature build with tonic and tokio); `AK_FFI_MODULE` selects which
  before `arms` is imported, because the first one loaded satisfies the others'
  `NEEDED` entry by soname.
- **A concurrency suite** (ABI v1 obligation 12.5, `logs/python/56-concurrency.log`).
- **The first consumer of the conformance corpus** (W8, `logs/python/70-corpus-subset.log`).
- **Levels.** The floor, CPython 3.7, is **not demonstrated**: no python3.7 in the
  container (apt lists 3.7.17), facts in `logs/python/01-environment.log`. The
  target is now **CPython 3.12, as shipped by Ubuntu 24.04 LTS** (FIX-PLAN owner
  decision D1). The slice's timed runs used 3.11. `STATE.md` records that every arm
  builds and passes on 3.10, 3.11, 3.12 and 3.13 (`logs/python/53-conformance-all-shapes.log`,
  `54-build-all-shapes.log`). The generated shim uses `Py_NewRef` (3.10+) and
  `PyObject_CallNoArgs` (3.9+), so it cannot build on 3.7 as it stands (review
  finding R-D4, unconfirmed; both calls are present in `gen/out/binding.c`).
  Free-threaded CPython (3.13t) is not available or installable in the container.
- **Configuration**: `protobuf` 7.36.2 on the upb C extension, confirmed at run
  time; the incumbent path (`SerializeToString` / `FromString`) is derived from the
  real `Protos/V1/results_service.proto` stub (`logs/python/52-r14-baseline.log`).

## 2. Correctness

- Encode and decode are byte-identical to `ffi/schema/generated/manifest.json` on
  all 16 payloads, every arm, on 3.10 to 3.13 (`53-conformance-all-shapes.log`).
  The absent paths (P1.3, P2.5) are in the gate.
- P7.1 has the allowance `design/SHAPES.md` grants: checked as a permutation of the
  same (tag, wire type, body) triples, and as parsing to the same message under the
  incumbent's parser.
- Decode is checked twice: re-encoding to the same bytes, and field by field
  against the incumbent, driven from the incumbent's descriptor. Oneof through
  `WhichOneof` against `<oneof>_case`, explicit presence through `HasField` against
  `is None`.
- ABI version and group layout facts are checked at import; the boundary is proved
  from the built artifact (R5).
- Corpus: **126 of 336 rows** (every root the slice's walker reaches, six roots),
  all three arms pass C1 parse 126/126, C2 project 123/123, C3 re-encode 126/126,
  C4 refuse 2/2. The other 210 rows, including every `Surrogate` and `WireZoo`
  vector, are not run.
- Concurrency: correctness passes on every row. Two shapes (P1.2, P2.2), threads in
  sequence and together at 2 and 4, per-thread facades and one facade shared
  across threads, every encode asserted against that arm's own single-thread
  output. Zero wrong bytes. The reference is each arm's own output rather than the
  manifest because `SerializeToString` does not sort map entries.
- The RPC arm is **not gated**: failures could be timed as successes (R-D3).

## 3. Defects found

- **Group-skip defect in the shared core** (D7). `ak_rt`'s unknown-field skip had
  no case for the deprecated GROUP form, so the core rejected `U-root-group`,
  `U-nested-group` and `U-oneof-group`, which upb accepts. Fixed in the core
  (`Dec::skip` takes the tag and recurses to a matching `END_GROUP`, bounded at 100
  nests); re-gated from this slice from 123/126 to 126/126 on C1 and C3 for both
  `core-ffi` arms. Four slices had gated clean on the same core: byte identity
  against a schema-generated manifest cannot find this class, because proto3
  cannot express a group.
- **Incumbent defect U1**: `protobuf` 7.36.2 on upb drops a whole map entry that
  carries an unknown field; the map comes back empty. The same version's
  pure-Python backend keeps the entry, as do the core and the pure-Python control.
  Isolated on a two-field message. The corpus marked that row (`U-map-entry`)
  disputed and withdrew its projection.
- **Slice defects D1 to D6, D8 to D10**, all fixed, each found by something
  running: the forward arm above 2^63; a floor arm that copied nothing (`bytes(ref)`
  on a `bytes` object returns the same object); the repository `.gitignore`
  excluding a nested `gen` directory (the fourth time on the branch); an absolute
  path baked into the generated tree, found by cloning the pushed branch to another
  path and building it; a nested length bounds-checked against the whole buffer; an
  unknown GROUP field that raised; decode discarding an inlined child a run had
  made; a reader walking nested messages with `dir()`; the pure-Python codec
  writing an empty map entry's key (found by corpus vector `E-map-entry-empty`).
- `corpus.py` had silently stopped running after a rename; it is now a step of
  `run.sh`.

## 4. Crossing counts

Counted by the counting build, both halves: the core counts its own crossings, the
shim counts its calls into CPython (`logs/python/53-conformance-all-shapes.log`,
identical on 3.10 to 3.13). Per element:

| payload | shim to CPython, C ext type | shim to CPython, plain / `__slots__` / `pyacc` | core fwd / rev |
|---|---|---|---|
| P1.2 (M1, 1,000 elements) encode | 7.00 | 29.00 | 0.01 / 0.00 |
| P1.2 decode | 7.00 | 24.00 | 0.00 / 0.01 |
| P1.3 (M1, absent path) encode | 7.00 | 21.02 | 0.01 / 0.00 |
| P2.2 (M2, 500) encode | 51.67 | 146.68 | 5.02 / 5.00 |
| P3.1 (M3) encode / decode | 5.40 / 3.15 | | 0.01 / 0.01 |
| P4.1 (M4) encode | 23.00 | | 1.01 / 1.00 |
| P5.4 (M5, 4 MB) encode | 3.00 | | 1.00 / 0.00 |
| P6.1 (M6, packed) encode / decode | 302.00 / 152.00 | | 5.01 / 5.00, 0.01 / 7.00 |
| P7.1 (M7) encode | 2.00 | | 0.50 / 0.33 |

What the counts say, as properties of the interface:

- The core's own boundary is 0.01 to 7 crossings per element; the
  shim-to-CPython edge is 2 to about 640 (P2.4). The batched element run turns
  1,000 elements into about ten core entries.
- The storages differ in whether the shim can stop making a crossing: only the
  struct member read moves (29 to 7 on P1.2 encode). Through `PyObject_GetAttr`,
  plain, `__slots__` and `pyacc` have the same count. On the absent path a getattr
  shim still makes its reads on elements that encode to nothing.
- A packed run crosses the ABI once per field and the shim reads each Python int
  individually (P6.1).
- Bulk bytes cross a constant 3 times from 36 B to 4 MB; a `bytes` object's buffer
  is already a stable address while the facade holds it, so there is nothing to
  pin.
- A oneof reads the discriminant and one member (P3.1).

## 5. Behaviour facts

- **Unknown fields: the facade drops them.** The core carries `ak_unk_f` slots and
  the shim passes NULL for every one, so the drop is the binding's choice, not a
  limit of the ABI. Retention is not built in this slice (owner position 6 asks for
  both behaviours).
- The facade stores a oneof's active member as its tag, not its name.
- A C extension type is not an idiomatic Python class; what that does to the public
  surface is not assessed.
- `PyList_SET_ITEM` is absent from the limited API, so an abi3 shim uses
  `PyList_SetItem`; the rest of what the shim needs is in the limited API from
  3.10. protobuf ships `cp3x-abi3` wheels. The composed shim is built full-API only.
- CPython's specialised `LOAD_ATTR` has an inline cache and the C API has no
  equivalent entry point, so `PyObject_GetAttr` from C does not get that cache.
- Reading a `str`'s UTF-8 is a cached pointer only for ASCII or when the object
  already has a UTF-8 cache; a string that came off the wire has none.
- The composed arm holds the GIL for every element, because every reverse step
  reads a Python object; `SerializeToString` also holds it.
- RPC deliveries: the queue's drainer is a Python thread that drops the GIL while
  waiting in `ak_queue_next`, so no core-owned thread touches a `PyObject`; the
  callback arrives on a tokio worker that must `PyGILState_Ensure` first.
- grpcio flow control, read from the core's own tracing: the static default stream
  window is 65,535 and the 4 MiB seen by default comes from BDP auto-tuning;
  `grpc.http2.lookahead_bytes` acts as a floor while probing is on; setting a window
  does not turn BDP probing off; there is no channel argument for the connection
  window. `packages/rust/armonik-transport`'s `ClientConfig` has no window setting.
- The gRPC C core sets `TCP_NODELAY` itself and this grpcio build has no
  `grpc.tcp_nodelay` argument.
- `packages/python` reads no transport environment configuration today.

## 6. Harness facts (for design/CAMPAIGN.md)

- **upb's `FromString` is lazy**: it parses into an arena and materialises Python
  objects only when read, and caches nothing, so every read re-materialises. A bare
  decode call is not the same work as an eager facade decode; a comparison states
  which one it uses. The RPC grid's C - B does not (R-C2).
- **Cold allocator state moved an encode figure** above about 128 KiB: glibc
  trimming the heap between calls, lifted by `M_TOP_PAD` (`poc/python/JOURNAL.md`
  J26, `logs/python/55-allocator.log`). Work unit 2's encode rows (logs 60, 61) were
  taken cold and are superseded; the bench now pins the allocator.
- **GC on or off matters**: the collector changes the facade's decode figures and
  not upb's (`logs/python/57-gc-bias.log`). The records disagree on which setting
  `bench.py` uses (R-F2).
- A floor arm must be checked to do its work (D2).
- A log whose script no longer runs is not evidence (`corpus.py`).
- Clone the pushed branch to another path and build it before closing a work unit
  (D4).
- The RPC grid is in-process, one run per cell, not interleaved (R-C3, R-C4).

## 7. Not measured or not established

- The floor (3.7) and free-threaded CPython.
- Decode under threads; more than 4 threads.
- The server side of an RPC, an encode-side RPC arm, streaming, TLS, a real network,
  failure injection.
- Allocation per operation.
- The corpus beyond the slice's six roots (210 of 336 rows).
- Unknown-field retention; abi3 on the composed shim; decision 13's borrowed-span
  facade; the pull decode family (ABI v1 7.1); a name-valued oneof discriminator.
- ctypes and cffi in the RPC layer.
- Every timing. The slice's container figures are instrumentation.

## 8. Open review findings (design/FIX-PLAN.md section 7, all unconfirmed)

- R-A7: Python floor 3.7 undemonstrated.
- R-B3: an encode ratio quoted from a log the slice superseded (J26); removed here.
- R-B4: pure-Python control figures misattributed and from superseded log 40; removed here.
- R-C2: RPC grid C - B compares lazy `FromString` with eager facade decode.
- R-C3: RPC grid B - A sign depends on configuration; one run per cell; not interleaved; A and B hit differently configured servers.
- R-C4: the Python RPC grid is in-process.
- R-C9: the Python RPC grid was run on uncommitted code.
- R-C13: the Python grid uses the pinned rather than the shipped transport.
- R-D3: the Python RPC arm is ungated; failures timed as successes.
- R-D4: the Python shim cannot compile on 3.7.
- R-E5: `py_codec` decode gaps (wire type, sign, tag 0); corpus roots unreachable.
- R-F1: Python `STATE.md` contradicts itself on what exists.
- R-F2: `STATE.md` says concurrency is unanswered though the suite exists; `57-gc-bias.log` closing line contradicts `bench.py`.
