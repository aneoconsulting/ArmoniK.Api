# Fix plan after the 2026-09-24 adversarial review

Status: **proposed, not started.** Written for whoever implements it, which is
assumed to be neither the author nor anyone with the review session's context.
Everything needed is in this file or in the paths it names.

## 0. Why this plan exists

On 2026-09-24 seven read-only `ffi-review` agents reviewed the plan
(`README.md`, `CLAUDE.md`, `REPORT.md`, `design/*`) and all five slices, one
angle each: the plan itself, measurement validity (native, then managed),
implementation correctness (native, then managed), the generators, and
cross-slice comparability. They produced about 90 findings, which dedupe to the
register in section 7. **Every finding is unconfirmed until the owning slice
agent answers it** (`CLAUDE.md`, roles), except the handful marked *verified*,
which the aggregating session checked against the source or log directly.

The review also exposed a mismatch between what the documents claim to be and
what the project owner wants them to be. The owner's position, which this plan
turns into rules:

1. **The decision is the owner's.** The branch records facts. No document, no
   agent and no role writes a recommendation, a verdict or a ranking of options.
2. **The branch is in its setup and design phase.** Performance is measured
   later, once, on **one physical machine**, **one slice after another**, with the
   client and the server **pinned to distinct CPU sets**. Every figure taken in a
   container before that is instrumentation: it shows a harness runs, it does not
   show a result.
3. **The core's transport may stay minimal.** `packages/rust`'s
   `armonik-transport` is known not to be at parity and will be brought to parity
   before any binding is implemented for real. A PoC transport with even fewer
   features (no TLS, retry, metadata, deadlines, numeric status, streaming) is
   acceptable because those features change neither feasibility nor the cost of
   the binding. Findings that attack the transport for missing features are
   closed by this, not fixed.
4. **The Python floor stays 3.7.** The Python *target* for the campaign is an
   open decision (section 6, D1).

## 1. Rules the implementer follows

- **Roles stay as `CLAUDE.md` defines them.** The aggregating session writes
  `README.md`, `CLAUDE.md`, `design/**`, `findings/**`, `REPORT.md`. Slice agents
  (`ffi-slice`, one per language) write `poc/<lang>/**` and `logs/<lang>/**`. The
  corpus agent writes `corpus/**`. Review agents write nothing. A change to
  existing behaviour of `poc/codec/` goes through the aggregating session.
- **Nobody re-measures performance in a container to settle a finding.** A
  performance finding is settled by fixing the harness so the campaign cannot
  repeat the defect, and by deleting the figure it affected. The only timing a
  container run may still produce is a smoke run proving a harness executes.
- **Correctness findings are settled by running something.** Build, run the
  gate, commit the log. A finding closed by argument alone stays open.
- **Remove, do not correct.** A container figure that is wrong is deleted, not
  replaced by a better container figure, because the replacement is equally not
  a result.
- **One work package per commit**, prefixes as `CLAUDE.md` says (`docs(ffi):`,
  `poc(<lang>):`). Slice agents do not push; the aggregating session pushes to
  the working branch.
- **Every closed finding gets one line** in section 7: `fixed <commit>`,
  `refuted <log>`, `closed by owner position N`, or `removed with the figure`.

## 2. Work packages, in order

WP1 and WP2 are documents and can start immediately. WP3 needs WP1 (so the new
rules are in force). WP4 and WP5 can run in parallel with WP3. WP6 closes.

### WP1. Rewrite the plan to the owner's position (aggregating session)

Files: `README.md`, `CLAUDE.md`, `REPORT.md`, `.claude/agents/ffi-slice.md`,
`.claude/agents/ffi-review.md`, `.claude/skills/ffi-review/SKILL.md`.

1. **Add a "Phase" section near the top of `README.md`** stating owner
   positions 2 and 3 verbatim in substance: setup and design phase; container
   figures are instrumentation; one campaign on one physical machine, slices run
   sequentially, client and server on distinct pinned CPU sets; minimal
   transport accepted and why.
2. **Replace the decision machinery.**
   - `README.md` section 13: the three "outcomes" become **options**, described
     neutrally, with no sentence of the form "X is ruled out", "worst
     combination", "non-starter", "collapses into". Add the option the review
     found missing: **the incumbent protoc codecs on the core's RPC layer** (it is
     grid cell B, already built in every host).
   - Delete "Requires no language to regress materially in both directions" (it
     is a decision rule, and the decision is the owner's).
   - `README.md` section 13.1: keep the grid's *definition* (cells A, B, C, D and
     what each difference isolates). Delete the results table and every result
     sentence under it.
   - `REPORT.md`: replace "which outcome the evidence supports and what rules out
     the other two" with "the facts established per option, and what is not
     established". Question 2 (interface cost vs runtime tax) becomes a campaign
     output, because it subtracts absolutes and can only be formed on one machine.
     Add Python to question 3 (floors). Add a question for W12 (below).
   - W9's done-when: "`REPORT.md` records the evidence per option and what it does
     not establish" instead of "states a recommendation".
3. **Extend the roles table in `CLAUDE.md`**: no role writes a recommendation,
   the aggregating session included. Add to `ffi-slice.md`: a `STATE.md` states
   what exists and what was checked, never what a binding "should" choose.
4. **Reclassify facts.** Add to `CLAUDE.md` invariants a list of what counts as
   a result in this phase: byte identity; crossing *counts*; floor builds and
   corpus passes; feasibility (it builds, it links, it round-trips); defects
   found. Timings are "harness validated" or "harness defect found" and nothing
   else.
5. **New work items in `README.md` section 7:**
   - **W11. Campaign specification**: `design/CAMPAIGN.md`, the contract every
     slice harness meets before the campaign. Content in WP3.
   - **W12. Divergence inventory**: a factual table of where the five packages
     under `packages/` behave differently today (which status codes are retried,
     what `AllowUnsafeConnection` disables, `send_result` write-then-notify
     ordering, chunk sizes, default windows, Nagle, keepalive, and whatever else
     is found). Each row cites file and line in each package. No cost estimate,
     no judgement. This is the only work item on the maintenance side of the
     question, which today has none (review P1).
   - **W13. Campaign run**: executes W11 on the physical machine. Owner-driven.
6. **Close the section 15 open questions that are answered**: 1 (moot, nothing to
   import), 3 (C++11 and C++14 both viable per `findings/cpp.md`); record 4 as
   "floor 3.7, target open (D1)".
7. **Correct `README.md` section 7 cells that contradict the slices**: W5 says the
   C# core-ffi arm is not built (it is, stage 14); W7 says Python M3 to M7, RPC and
   concurrency are not built (they are).

Done when: a reader of `README.md`, `CLAUDE.md` and `REPORT.md` finds no
recommendation, no container figure presented as a result, the phase statement,
the options list including cell B, and W11 to W13.

### WP2. Purge the findings and the README of container results (aggregating session)

Files: `README.md` sections 2, 3, 4.1, 9, 13, 13.1; `findings/*.md`;
`design/ABI-v1.md`.

1. **Delete every container absolute and ratio** from `README.md` sections 2, 3,
   4.1, 9, 13 and 13.1, and from `findings/*.md`. This removes, among others, the
   figures the review found wrong: R-B1 to R-B6 in section 7. Do not re-quote
   corrected values.
2. **Keep, and state as facts**: crossing counts per payload (these are
   properties of the interface), byte identity results, floor builds that were
   demonstrated (C++11: `logs/cpp/conformance.log`; Java 8:
   `logs/java/floor.log`; netstandard2.0 on Mono 6.8, *not* .NET Framework),
   feasibility results, and defects found.
3. **Keep the harness lessons, reworded as harness facts**, in a new
   `README.md` subsection "Known harness hazards" that W11 cites. The review and
   the slices together found these; each becomes a W11 requirement:
   in-process servers can flip a delta's sign (Java, `logs/java/rpc.log`);
   upb's `FromString` is lazy, so a bare decode call is not the same work as an
   eager decode (Python); `Process.TotalProcessorTime` has 10 ms steps on Linux
   (C#); a per-call fresh allocation in a marshaller dominated a cell (Java cell
   D); protobuf-java's wide-string encode has two JIT states 2x apart
   (`logs/java/deopt.log`); a cold allocator moved a Python encode ratio by 0.5
   (`poc/python/JOURNAL.md` J26); the core's CPU counter in one process includes
   the server.
4. **`design/ABI-v1.md`**: keep decisions and their *mechanisms*. Remove the
   container figures used as motivation, or mark each "instrumentation, not a
   result". Decision 1's "answered by the C++ slice" rests on a table the review
   could not find in any log (R-C1); reopen it as "to be measured in the
   campaign". Decision 11 ("Blocks: nothing") gets the note that it changes
   behaviour for four languages and needs the product question "does ArmoniK
   re-encode anything it decoded" answered (R-A6).
5. **Fix the two document inconsistencies**: corpus vector count (328 at
   `README.md` section 10.2 against 336 in W8), and section 3's stale bullets
   (Python has no PoC; no corpus exists; C# has no managed decode control; C++
   never measured on the amended ABI).

Done when: `grep -nE '[0-9]\.[0-9]+ ?(ns|us|µs|ms)|[0-9]\.[0-9]{2,3} of '
README.md findings/*.md` returns only lines inside "Known harness hazards" or
lines explicitly labelled instrumentation.

### WP3. Write `design/CAMPAIGN.md` (aggregating session), then conform each harness (slice agents)

`design/CAMPAIGN.md` is the contract. Each requirement closes one or more
review findings (in brackets). A slice harness is campaign-ready when it meets
all of them and a smoke run on its container shows it executes.

**Machine and process**
1. One physical machine, bare metal, no other tenant. Slices run **one after
   another**, never concurrently.
2. Record in every log header: CPU model, core count, SMT on/off, governor,
   turbo on/off, kernel, `isolcpus` or cgroup setup, and the commit hash. A log
   whose commit is "UNCOMMITTED" is invalid. [R-C9]
3. **Client and server in separate processes, pinned to disjoint CPU sets** (for
   example `taskset`/`cpuset`), with the sets named in the log header. This
   applies to every host, including Rust and C++. [R-C3, R-C4, R-C5]
4. A codec micro-benchmark runs in one process with no server; its CPU set is
   also named.

**Cells and runs**
5. The RPC grid is defined once, identically for every host: cells A (host codec,
   host transport), B (host codec, core transport), C (core codec, core
   transport), D (core codec, host transport). Same delivery mode across hosts
   for B and C (pick one: blocking), same decode family for C (pick one, and the
   other is a labelled extra row). [R-C5, R-C6]
6. Cells are **interleaved per round**, at least 5 rounds, at least 3 separate
   process launches; per-round values are committed, not only medians. [R-C4,
   R-C7]
7. Two request directions: an empty request with the P2.2 response (today's
   grid), and a P2.2-sized request that the server decodes. Plus one streamed
   upload in 2 MiB chunks, which is ArmoniK's bulk path. [R-C8]
8. Concurrency levels 1, 8, 16 in flight, with CPU per call **and** wall clock
   per call both reported, because they can disagree (queue delivery).

**What each cell does**
9. **Decode is reported twice**: the bare call, and decode plus reading every
   field. A comparison against upb's `FromString` states which. [R-C2]
10. **The incumbent is the production path (R14)** in every host: gRPC's
    generated marshaller (grpc++ `SerializationTraits`, grpc-java's marshaller,
    Grpc.Net's marshaller over `ReadOnlySequence`, grpcio's
    `SerializeToString`/`FromString` from the stub). The library's best entry
    point is a labelled second row. [R-C10, R-C11]
11. Every RPC call checks status and response length; a failed call aborts the
    run. [R-D3]
12. Server-side work is either identical across cells (pre-serialised response
    bytes) or fully timed and subtracted, not partially. [R-C5]

**Measurement mechanics**
13. CPU from `getrusage(RUSAGE_THREAD)`/`clock_gettime(CLOCK_THREAD_CPUTIME_ID)`
    or process-level `getrusage` on the client process only.
    `Process.TotalProcessorTime` is not allowed. [R-C6]
14. Allocator state is pinned or warmed identically for every arm (the J26
    lesson). JIT hosts state warm-up policy and the JIT state of the incumbent,
    and run protobuf-java's content-set rows in both string-coder states. [R-C12]
15. Crossing calibration reports forward and reverse separately, with
    `perf stat` cycles and instructions per iteration, so a sub-cycle figure is
    recognisable. [R-B5]
16. Transport configuration: every cell runs both **as shipped** (what
    `packages/<lang>` configures) and **pinned** (4 MiB windows, stated), and B/C
    follow the same switch as A/D. [R-C13]

**Runtimes** (decisions D1 to D3 fill the blanks)
17. C#: floor netstandard2.0 on real .NET Framework 4.8, target `<D2>`.
    Google.Protobuf 3.32.0, Grpc.Net.Client 2.71.0 (the versions in
    `packages/csharp`).
18. Java: floor Java 8, target `<D3>`. grpc-java 1.74.0 as in
    `packages/java/pom.xml`.
19. Python: floor 3.7 (correctness only), target `<D1>`. protobuf and grpcio at
    versions inside `packages/python/pyproject.toml`'s ranges, stated.
20. C++: floor C++11, target C++17. grpc++ at the version ArmoniK builds
    (`v1.54.0` in `packages/cpp/tools/Dockerfile.worker`) and at a current one,
    both stated.

Then each slice agent conforms its harness and commits a smoke log. Per-slice
deltas from today, as the review found them:

| Slice | Harness changes |
|---|---|
| rust | server out of process (stage 6 grid is in-process); cells interleaved; request-direction cell exists already, add the empty-request variant so it matches the others |
| cpp | server out of process; cell A decode through the same entry point as B or both through the marshaller; server serialise timed or pre-serialised; production-path incumbent row |
| csharp | server out of process; CPU via `getrusage`, not `TotalProcessorTime`; B/C honour `--shipped`; min-of-9 replaced by committed per-round values |
| java | all cells in one client process, interleaved, server in its own process; raw `RunRpc` output committed; production-path incumbent as headline row |
| python | server out of process; A and B/C interleaved; decode-plus-read cells; status and length checks; same server args for A and B in pinned rows |

### WP4. Correctness fixes (owners as listed; each needs a committed log)

In priority order. Items 1 to 3 may invalidate existing data or crash a host.

| # | Fix | Owner | Settled by |
|---|---|---|---|
| 1 | **Length-varint wrap.** `pos + n` overflows in release builds (no `overflow-checks` in `poc/codec/Cargo.toml`). Fix with a checked comparison (`n > len - pos`) in `poc/codec/crates/ak-rt/src/dec.rs` `len_body` and every other length check, and in `poc/cpp/include/ak/rt.h`. Also: do not flush groups after a decode error (`codec.rs` `flush!()`/`apply` after the loop), and make every slice `&buf[a..b]` in an `extern "C"` path non-panicking. [R-D1] | aggregating session for the core (behaviour change), cpp for `rt.h` | The 11-byte input `7A F5 FF FF FF FF FF FF FF FF 01` to `ak_decode_ListResultsResponse` returns an error within 1 s, before and after the fix, logged |
| 2 | **Corpus vectors that would have caught 1 and the generator gaps**: a length wrapping 2^64 from several positions; a known tag at the wrong wire type (per root); tag 0 on every root; `-0.0`; negative int32/int64 decode projected (C2), not only hashed. | corpus agent | Vectors in `corpus.json`, gate selftest passes, every slice re-runs the corpus |
| 3 | **C++ `ak_client_opts`** declares 3 fields, the core reads 6, so `tcp_nagle` is stack garbage (*verified*). Include a generated header instead of the hand declaration in `poc/cpp/src/rpc_common.h`, and add a `static_assert` on size. Then check with `git log -S tcp_nagle -- poc/codec` whether `logs/cpp/rpc.log` and `rpcflow.log` predate the 6-field struct; if not, mark their TCP rows invalid. [R-D2] | cpp | Layout guard compiles; log of the history check |
| 4 | **Python RPC arm gate**: check status and length on every call (`poc/python/rpc.py`, `native/binding.c` `take_bytes`); run `conformance.py` with `AK_FFI_MODULE=_akffi_rpc`; add `_akffi_rpc` to `build.sh`'s R5 boundary loop. [R-D3] | python | Conformance log for `_akffi_rpc` |
| 5 | **Python floor 3.7**: the generated shim uses `Py_NewRef` (3.10+) and `PyObject_CallNoArgs`/`PyObject_CallOneArg` (3.9+); the PyO3 arm is `abi3-py310`. Give the shim generator a floor level emitting 3.7-compatible calls, build on 3.7, run the corpus. [R-D4] | python | Corpus log on CPython 3.7 |
| 6 | **Gate every timed arm**: C++ `ffi-valtc` (add to `conformance.cpp` `run_case`); Java `ffi-pull`/`ffi-pull-walk` (a committed conformance log naming them). [R-D5] | cpp, java | Conformance logs listing the arms |
| 7 | **Sticky error slot on encode**: encode entry points ignore `hdr.err`, so a host that calls `ak_fail` and returns 0 gets a successful encode; decode checks it only at the end. Align with `design/ABI-v1.md` section 5. [R-D6] | aggregating session (core) | A test callback that calls `ak_fail` and returns `AK_OK` makes encode and decode fail |
| 8 | **C++ concurrency must-fail control reaches the core**: build the planted variant of the shared core (`--features pad-widths` or equivalent) and link it, so `ffi > 0` is shown possible. Report 22 distinct wrong encodes, not 44. [R-D7] | cpp | Log with `ffi > 0` under the plant |
| 9 | **Rust concurrency coverage**: `concur.rs` `together()` uses only P1.3 and P2.5 (absent-path); add P1.2 and P2.2; run under ThreadSanitizer if the toolchain allows. [R-D8] | rust | Log |
| 10 | Minor: C# `ak_bytes_free` on non-OK status (`src/Rpc/CoreTransport.cs`); JNI `GetPrimitiveArrayCritical` null check (`native/generated/shim.c` encode path); `from_raw_parts(null, 0)` in `tc_utf8*` (`poc/codec/.../lib.rs`); `u32` truncation of spans for buffers over 4 GiB (reject at entry); `opts_word` XOR collision; Rust `lifecycle.sh`/`guardprice.sh` no longer build a guard-OFF arm because `init-guard` became default. [R-D9] | each owner | Per item |

### WP5. Generator consistency (aggregating session for `poc/codec/gen`, slices for their `gen/`)

The maintenance argument rests on "one generator". The review counted seven
independent traversal emitters with their own wire rules. This is a fact to
record (WP1 W12's sibling), and the following are defects to fix:

1. **`poc/codec/gen/rust_core.py`** says the ABI core and the core-native control
   share one traversal through `GroupEnc`/`FixDec`, which do not exist
   (*verified*: `rust_abi.py` imports only `Sites` from it). Either make it true
   or correct the docstring and the generated header comment, and record in
   `README.md` that `ffi - native` differences in Rust and C++ include traversal
   differences, not only the boundary. [R-E1]
2. **Wire-type check on packed/unpacked decode**: `rust_abi.py` accepts wire 0 or
   1 for every kind; `rust_core.py` and `cpp_core.py` accept any; C# and Java
   match exactly. Make all emitters match the declared wire type and skip
   otherwise. [R-E2]
3. **`codec/gen/ir.py` has no `fixed32`**, so the core generator cannot load the
   corpus schema, and the corpus release gate (ABI v1 section 12.1) never reaches
   the core's encoder. Add it. [R-E3]
4. **`-0.0`**: `rust_abi.py`, `rust_core.py`, `cpp_core.py` drop it (`!= 0.0`).
   Compare bits, as C# does. [R-E3]
5. **Java arm R** (`poc/java/gen/java_codec.py`): run the corpus; fix tag 0,
   unconditional map key, `-0.0`, merge semantics for repeated singular
   messages, unknown oneof case. [R-E4]
6. **Python `py_codec.py`**: wire-type check on known fields, int32/int64 sign on
   decode, tag 0; extend `walk.ROOTS` so the corpus reaches `WireZoo` and `Nest`.
   [R-E5]
7. **C# `abi_ir.py`**: sort by tag like the core, and make the layout probe
   check field lists, not only sizes. [R-E6]
8. **UTF-8 decode policy** differs per runtime (C# managed lossy by default,
   others reject). Make it one generator option with one default, stated. [R-E7]
9. **Field order**: emitters that write oneofs after plain fields match
   protobuf's tag order only because `Probe`'s oneof tags are last. Emit in tag
   order. [R-E8]

### WP6. STATE hygiene and re-review

1. **Each slice agent rewrites its `STATE.md`**: remove self-contradictions (the
   review found each of C#, Java, Python and C++ saying in one place that the RPC
   arm, pull family or concurrency suite does not exist and in another that it
   does); remove recommendations; label every figure instrumentation; delete any
   figure with no committed raw log. Specific: C++ `STATE.md:62-78` headline table
   has no matching log (*verified*, R-C1) and goes; Java RPC grid figures have no
   raw runner output (R-C9) and go unless it is committed; Rust `STATE.md:110-137`
   is a retired table (R-C14).
2. **Re-review** with the `ffi-review` skill, scoped to what WP3 to WP5 changed,
   plus one agent on `design/CAMPAIGN.md` asking "can any cell of this still
   produce a sign that is a harness property". The implementer does not review
   their own work.

Done when: section 7 has a disposition on every line, and the re-review raises
nothing that blocks the campaign.

## 3. What this plan deliberately does not do

- It does not re-take any timing in a container.
- It does not add TLS, retry, metadata, deadlines or streaming to the core's
  transport (owner position 3). The streamed-upload cell in W11 item 7 needs
  client streaming in the core; if that is more than a small addition, the
  implementer records it as a gap and asks the owner rather than building a
  streaming engine.
- It does not choose between options. It does not define "material".
- It does not change anything under `packages/`.

## 4. Facts gathered while writing this plan

For the implementer, so they are not re-derived. Checked 2026-09-24 against the
repository at `1d85b1d`.

| Package | What it ships | Where |
|---|---|---|
| C# client | netstandard2.0 | `packages/csharp/ArmoniK.Api.Client/ArmoniK.Api.Client.csproj` |
| C# worker | net6.0 | `packages/csharp/ArmoniK.Api.Worker/ArmoniK.Api.Worker.csproj` |
| C# client tests | net4.7, net4.8, net6.0, net8.0 | `packages/csharp/ArmoniK.Api.Client.Test/...csproj` |
| C# libraries | Google.Protobuf 3.32.0, Grpc.Net.Client 2.71.0, Grpc.Tools 2.72.0 | the `.csproj` files |
| Java | `maven.compiler.release` 17, grpc-java 1.74.0, protoc 3.19.0 | `packages/java/pom.xml:56-60` |
| Python | `requires-python >=3.7`, `grpcio>=1.62,<2`, `protobuf>=4.21.6,<8`; no runtime pinned anywhere in the repo | `packages/python/pyproject.toml` |
| C++ | gRPC v1.54.0 in the worker image, Alpine 3.21 | `packages/cpp/tools/Dockerfile.worker:2-3` |

External facts:
- .NET 6 went out of support on 2024-11-12; .NET 8 (LTS) ends support on
  2026-11-10; .NET 10 (LTS) was released 2025-11-11 and is supported to
  2028-11-14. Source: Microsoft, ".NET and .NET Core Support Policy",
  https://dotnet.microsoft.com/en-us/platform/support/policy/dotnet-core
  (retrieved 2026-09-24).
- CPython end-of-life dates could not be retrieved from this environment
  (python.org and peps.python.org are blocked by the network policy). Check them
  at https://devguide.python.org/versions/ before deciding D1.

Two facts that bear on the design constraints and are not in `README.md`:
- **`packages/java` targets Java 17, not Java 8.** The README's Java 8 floor is a
  customer constraint stated by the base design, not what the package builds
  today. Record both; the owner decides whether the floor stands.
- **The C# worker targets net6.0, which is out of support**, and the slice
  measured .NET 8. The generated `LibraryImport` binding needs .NET 7 or later,
  so it does not build for net6.0 as it stands.

## 5. Effort, roughly

| WP | Who | Size |
|---|---|---|
| WP1, WP2 | aggregating session | about one session together |
| WP3 spec | aggregating session | half a session |
| WP3 harnesses | five slice agents in parallel | one session each; C++ and Java the largest |
| WP4 | core + four slices + corpus | items 1 to 4 in one session; the rest opportunistic |
| WP5 | aggregating session + java + python + csharp | one session; items 1 to 4 are the core generator |
| WP6 | slices, then review agents | half a session each |

## 6. Decisions the owner must make (the plan does not make them)

- **D1. Python target for the campaign.** Facts: the package declares `>=3.7`
  and pins no runtime; the Python slice's container runs used CPython 3.11; 3.13
  has an optional free-threaded build that changes the GIL assumptions in README
  section 9. Options: (a) the CPython version ArmoniK's Python users actually
  deploy, if known; (b) the oldest CPython still in upstream support on the
  campaign date; (c) 3.11 for continuity with the existing harness; (d) two
  targets, one of (a)/(b) plus 3.13 free-threaded as a labelled extra row.
- **D2. C# target**: net6.0 (what the worker ships, out of support, and the
  current binding does not build for it), .NET 8 (what the slice used, support
  ends 2026-11-10), or .NET 10 (current LTS).
- **D3. Java target**: 17 (what `packages/java` builds) or 21; and whether the
  Java 8 floor stands given the package itself targets 17.
- **D4. Decision 11 (unknown-field retention)**: does ArmoniK re-encode anything
  it decoded (the worker forwarding path is the candidate)? A product fact the
  owner can answer and no slice can.
- **D5. Streamed-upload cell**: build client streaming in the core for the
  campaign, or leave bulk transfer out of the campaign and say so.
- **D6. Traffic profile**: whether a coarse call and byte mix from a running
  deployment can be obtained, so the payload set can be stated as representative
  or not (review P5). Without it the report says the payloads are not weighted.

## 7. Findings register

Source angle: P plan, MN measurement native, MM measurement managed, CN
correctness native, CM correctness managed, G generator, X comparability.
Disposition column is filled in as work lands.

### A. Plan

| ID | Finding | Source | Goes to | Disposition |
|---|---|---|---|---|
| R-A1 | Maintenance case never measured; no work item, no REPORT question | P | WP1 (W12) | |
| R-A2 | Outcome criteria undefined ("materially", aggregation, which decode column) | P | WP1 (decision rule removed) | |
| R-A3 | Option "protoc codecs + core RPC" (cell B) missing | P | WP1 | |
| R-A4 | Core transport lacks TLS, retry, metadata, deadlines, status | P | none | closed by owner position 3 |
| R-A5 | REPORT Q2 decomposition subtracts cross-machine absolutes | P, X | WP1 (moved to campaign) | |
| R-A6 | ABI decision 11 marked "Blocks: nothing" but changes behaviour in four languages | P | WP2, D4 | |
| R-A7 | Floors incomplete: net48 only on Mono; Python 3.7 undemonstrated; Rust MSRV unverified | P, X | WP3 item 17, WP4 item 5 | |
| R-A8 | Payload representativeness asserted, not established | P | D6 | |
| R-A9 | No RPC arm measures encode, streaming or the worker path | P, X | WP3 item 7 | |

### B. Container figures stated wrongly in the documents (all removed by WP2)

| ID | Finding | Source | Disposition |
|---|---|---|---|
| R-B1 | Java outcome-2 bullet uses withdrawn +368 and an unsourced -268 (*verified*, `README.md:1190`) | P, MM | |
| R-B2 | C++ empty-call sign reversed: log says the core costs 44-89 % more (*verified*, `logs/cpp/rpc.log:694`) | MN | |
| R-B3 | Python encode "0.700-0.719 of upb" is from a log the slice superseded (J26); W7 and `findings/python.md` still carry it | P, X, MM | |
| R-B4 | "34x to 60x" is the pure-Python control's encode, not the core; "19.4-20.3x" from superseded log 40 | P, MM, X | |
| R-B5 | Crossing table mixes forward and reverse, and different metrics across containers; C# 7.5-12 ns never measured by the slice | MN, X | |
| R-B6 | README section 3 stale; corpus count 328 vs 336 | P, X | |

### C. Harness defects (closed by WP3 conformance plus deleting the affected figures)

| ID | Finding | Source | Disposition |
|---|---|---|---|
| R-C1 | C++ `STATE.md:62-78` table matches no committed log; batching verdict (ABI decision 1) flips in the cited log (*verified*) | MN | |
| R-C2 | Python grid's C - B compares lazy `FromString` with eager facade decode | P, MM, CM, X | |
| R-C3 | Python B - A sign depends on configuration; one run per cell; not interleaved; in-process; A and B hit differently configured servers | MM, CM | |
| R-C4 | C#, C++, Python grids in-process; Java's flipped sign when moved out | P, MM, CM, MN | |
| R-C5 | Grid rows are different experiments (delivery, decode family, units, server accounting, request direction) | X, MN | |
| R-C6 | C# CPU quantised at 10 ms; min-of-9 selection; blocks not interleaved | MM, CM | |
| R-C7 | Java two-process codec half below its own stated resolution; unpaired JVMs | MM, CM | |
| R-C8 | Only the response direction measured in RPC (Rust aside) | P, X | |
| R-C9 | Java RPC grid has no raw runner log; Python grid run on uncommitted code; C# stages 18-19 built against a core not on the branch | MM | |
| R-C10 | Incumbent is the library's best path in C++ and Java headlines, not gRPC's marshaller (R14); Java's R14 quotient suggests the encode sign differs | MN, X | |
| R-C11 | C# measured on .NET 8 and Google.Protobuf 3.28.3, ArmoniK ships net6.0 worker and 3.32.0 | X | |
| R-C12 | Java wide-content encode measured with protobuf-java in its slow JIT state | MM | |
| R-C13 | C# B/C stay pinned under `--shipped`; C# and Python grids use the pinned rather than shipped transport | MM, CM | |
| R-C14 | Rust `STATE.md:110-137` shows a retired table | MN, X | |
| R-C15 | C++ batching-crossover compared against other containers' absolutes; tax sweep does not reproduce run to run | MN, X | |
| R-C16 | Rust crossing bimodal (1.8/2.1); guard "±0.003 ns" against a 0.70 ns control resolution | MN | |

### D. Correctness (WP4)

| ID | Finding | Source | Disposition |
|---|---|---|---|
| R-D1 | Length-varint wrap: hang, 4 GiB out-of-bounds span to host, panic across `extern "C"` (source *verified*, not run) | CN | |
| R-D2 | C++ `ak_client_opts` 3 of 6 fields; `tcp_nagle` read from stack (*verified*) | CN | |
| R-D3 | Python RPC arm ungated; failures timed as cheap successes | CM | |
| R-D4 | Python shim cannot compile on 3.7 | CM | |
| R-D5 | `ffi-valtc` (C++) and pull arms (Java) not in any gate log | CN, CM | |
| R-D6 | Encode ignores the sticky error slot | CN | |
| R-D7 | C++ concurrency plants never reach the core; wrong encodes double-counted | CN | |
| R-D8 | Rust concurrency suite mostly absent-path payloads | CN | |
| R-D9 | Minor boundary items (see WP4 item 10) | CN, CM | |

### E. Generators (WP5)

| ID | Finding | Source | Disposition |
|---|---|---|---|
| R-E1 | "One traversal" claim in `rust_core.py` false (*verified*) | G | |
| R-E2 | Wire-type acceptance differs across emitters | G | |
| R-E3 | Core generator lacks `fixed32`; `-0.0` dropped by core emitters | G | |
| R-E4 | Java arm R never ran the corpus; five rule gaps | G | |
| R-E5 | Python `py_codec` decode: wire type, sign, tag 0; corpus roots unreachable | G | |
| R-E6 | C# re-derives ABI layout; probe shares the field list it checks | G | |
| R-E7 | UTF-8 decode policy differs per runtime | G | |
| R-E8 | Field order correct only by accident of tag numbering | G | |
| R-E9 | "One generator" is not true of the tree: seven traversal emitters | G | record as fact in W12's neighbourhood (WP1) |

### F. STATE hygiene (WP6)

| ID | Finding | Source | Disposition |
|---|---|---|---|
| R-F1 | C#, Java, Python, C++ `STATE.md` contradict themselves on what exists | CM, X, CN | |
| R-F2 | Python `STATE.md` "concurrency still unanswered" vs suite present; `57-gc-bias.log` closing line contradicts `bench.py` | MM, CM | |
