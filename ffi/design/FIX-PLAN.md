# Fix plan after the 2026-09-24 adversarial review

Status: **WP1 to WP5 done; decision 11 (unknown fields) implemented in all five slices with the owner's rules; the RPC grid runs C and D in retain and drop in all five slices; the no-unknown build (unknown fields compiled out, `Options(unknown="drop")`) rendered, gated and in the campaign harness of all five slices; WP3 harnesses on each ecosystem's standard framework; `ffi/campaign.sh` written** (2026-09-25). Crossing counts, no-unknown against drop, push decode: only P1.2 decode reverse 8 to 5 in every slice (java's pull path also drains fewer chunks). Next: WP6 (STATE hygiene, every gate re-run from a clean tree, a targeted re-review).
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
4. **Language levels are fixed** (section 6 records them): Python floor 3.7,
   target CPython 3.12 (Ubuntu 24.04); C# floors net6.0 and .NET Framework 4.8,
   target net8.0; Java floor 8, target 17.
5. **There is one generator implementation.** Every generated codec and binding,
   in every language, comes out of the same generator with the wire rules
   written once. Per-language backends render syntax; they do not decide wire
   behaviour. This is the premise of the maintenance case, so the PoC has to
   demonstrate it rather than approximate it (WP5).
6. **Unknown-field retention is undecided, so it is measured**: the generator
   and the core carry both behaviours (drop, retain) and the campaign times both.
7. **There are no statistics on real payloads.** The payload set is not
   weighted by traffic and the report says so.

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
- **Never two work packages in one commit** (a package may take several), prefixes as `CLAUDE.md` says (`docs(ffi):`,
  `poc(<lang>):`). Slice agents do not push; the aggregating session pushes to
  the working branch.
- **Every closed finding gets one line** in section 7: `fixed <commit>`,
  `refuted <log>`, `closed by owner position N`, or `removed with the figure`.

## 2. Work packages, in order

Order of execution (the numbering is kept for reference, the order is not
numeric):

1. **WP1, WP2**: documents. Start immediately; nothing depends on code.
2. **WP4 items 1 to 3**: correctness defects that can crash a host or invalidate
   a log. Fix them in the current code so the existing gates can be re-run.
3. **WP5**: the one generator. It replaces most of the generated code every
   harness calls, so harness work before it would be done twice.
4. **WP4 remaining items**: most of them land in the generator's single rule set
   and are done as part of WP5; the rest follow.
5. **WP3**: the campaign specification (can be written in parallel with WP5),
   then each slice conforms its harness to it on the generated code.
6. **WP6**: STATE hygiene and re-review.

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
3. **Add the one-generator invariant to `CLAUDE.md`** beside R0 ("one core, not
   one emitter"): every generated codec and binding comes from
   `poc/codec/gen/`, wire rules are written once in its plan layer, and a wire
   rule in a slice `gen/` is a defect in the same way a second copy of the core
   is. The reference encoders are the stated exception (WP5 item 5).
4. **Extend the roles table in `CLAUDE.md`**: no role writes a recommendation,
   the aggregating session included. Add to `ffi-slice.md`: a `STATE.md` states
   what exists and what was checked, never what a binding "should" choose.
5. **Reclassify facts.** Add to `CLAUDE.md` invariants a list of what counts as
   a result in this phase: byte identity; crossing *counts*; floor builds and
   corpus passes; feasibility (it builds, it links, it round-trips); defects
   found. Timings are "harness validated" or "harness defect found" and nothing
   else.
6. **New work items in `README.md` section 7:**
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
   - **W14. One generator implementation**: WP5 as a work item, so the README's
     table carries it.
7. **Close the section 15 open questions that are answered**: 1 (moot, nothing to
   import), 3 (C++11 and C++14 both viable per `findings/cpp.md`); record 4 as
   "floor 3.7, target CPython 3.12 (Ubuntu 24.04)". Record 6 (Java packaging)
   as still open. Record the owner's levels (section 6) in README section 5.
8. **Correct `README.md` section 7 cells that contradict the slices**: W5 says the
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
   in-process servers can flip a delta's sign (Java, the former `logs/java/rpc.log`, withdrawn in c23533ea7 under R-C9, no raw runner output);
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
   grid), and a P2.2-sized request that the server decodes. A streamed upload
   in 2 MiB chunks, ArmoniK's bulk path, is optional (item 22). [R-C8]
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

**Runtimes** (fixed by the owner, section 6)
17. C#: floors **net6.0** and **.NET Framework 4.8**, target **net8.0**.
    Floors are correctness gates only (corpus and byte identity), target is where
    the clock runs. Google.Protobuf 3.32.0, Grpc.Net.Client 2.71.0 (the versions
    in `packages/csharp`). Two consequences the implementer must handle:
    - .NET Framework 4.8 runs only on Windows, so the net48 gate needs a Windows
      machine or runner. Mono 6.8, which the slice used, is not .NET Framework
      and does not count as the gate.
    - The generated `LibraryImport` binding needs .NET 7 or later, so both
      floors (net6.0 and net48) use `DllImport`. The generator emits **one**
      binding file with both forms under `#if NET7_0_OR_GREATER` /
      `#else`, not two renderings (WP5 item 3, conditional compilation).
18. Java: floor **8** (correctness), target **17** (JNI, as the slice already
    uses). grpc-java 1.74.0 as in `packages/java/pom.xml`.
19. Python: floor **3.7** (correctness), target **CPython 3.12, the version Ubuntu
    24.04 LTS ships** (section 4). protobuf and grpcio at versions inside
    `packages/python/pyproject.toml`'s ranges that publish wheels for 3.12,
    stated in the log. The 3.7 gate runs on an interpreter installed for the
    purpose (Ubuntu 24.04 does not package it), with the newest protobuf and
    grpcio releases that still support 3.7, stated.
20. C++: floor C++11, target C++17. grpc++ at the version ArmoniK builds
    (`v1.54.0` in `packages/cpp/tools/Dockerfile.worker`) and at a current one,
    both stated.

**Unknown fields** (owner position 6)
21. Every host times decode, and decode followed by re-encode, on the corpus's
    unknown-field payloads, with the core and the generated codecs in both
    modes, **drop** and **retain**, and the incumbent in its default mode (stated).
    Byte identity of a retain-mode round trip is part of the gate.

**Streaming** (optional, scheduled last)
22. The streamed upload of item 7 is optional. It is built only after items 1 to
    21 hold in every slice, and only if client streaming in the core is a small
    addition. Otherwise the report lists bulk transfer as not measured.

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
| 5 | **Python floor 3.7**: the generated shim uses `Py_NewRef` (3.10+) and `PyObject_CallNoArgs`/`PyObject_CallOneArg` (3.9+); the PyO3 arm is `abi3-py310`. Emit the newer calls under `#if PY_VERSION_HEX >= ...` with a 3.7-compatible `#else` in the one generated shim, build on 3.7, run the corpus. [R-D4] | python | Corpus log on CPython 3.7 |
| 6 | **Gate every timed arm**: C++ `ffi-valtc` (add to `conformance.cpp` `run_case`); Java `ffi-pull`/`ffi-pull-walk` (a committed conformance log naming them). [R-D5] | cpp, java | Conformance logs listing the arms |
| 7 | **Sticky error slot on encode**: encode entry points ignore `hdr.err`, so a host that calls `ak_fail` and returns 0 gets a successful encode; decode checks it only at the end. Align with `design/ABI-v1.md` section 5. [R-D6] | aggregating session (core) | A test callback that calls `ak_fail` and returns `AK_OK` makes encode and decode fail |
| 8 | **C++ concurrency must-fail control reaches the core**: build the planted variant of the shared core (`--features pad-widths` or equivalent) and link it, so `ffi > 0` is shown possible. Report 22 distinct wrong encodes, not 44. [R-D7] | cpp | Log with `ffi > 0` under the plant |
| 9 | **Rust concurrency coverage**: `concur.rs` `together()` uses only P1.3 and P2.5 (absent-path); add P1.2 and P2.2; run under ThreadSanitizer if the toolchain allows. [R-D8] | rust | Log |
| 10 | Minor: C# `ak_bytes_free` on non-OK status (`src/Rpc/CoreTransport.cs`); JNI `GetPrimitiveArrayCritical` null check (`native/generated/shim.c` encode path); `from_raw_parts(null, 0)` in `tc_utf8*` (`poc/codec/.../lib.rs`); `u32` truncation of spans for buffers over 4 GiB (reject at entry); `opts_word` XOR collision; Rust `lifecycle.sh`/`guardprice.sh` may no longer build a guard-OFF arm (the harness crate's defaults include `init-guard`; the shared core's defaults do not, R-G7). [R-D9] | each owner | Per item |

### WP5. One generator implementation (aggregating session owns the design and the shared part; slice agents port their backends)

Owner position 5 makes this a requirement, not a clean-up. What exists today
(counted 2026-09-24, Python lines excluding generated output):

| Where | Lines | What it decides on its own |
|---|---|---|
| `poc/codec/gen/` (`ir.py`, `rust_abi.py`, `rust_core.py`, `cpp_layout.py`) | 3,915 | the core's wire rules (`rust_abi.py` walks), a second set for the core-native control (`rust_core.py`) |
| `poc/cpp/gen/` | 4,107 | `cpp_core.py` native codec rules; binding |
| `poc/csharp/gen/` | 5,429 | a second IR (`ir.py`, `abi_ir.py`) re-deriving the ABI layout; `cs_managed.py` wire rules |
| `poc/java/gen/` | 3,727 | `java_codec.py` wire rules (arm R) |
| `poc/python/gen/` | 2,988 | `py_codec.py` wire rules over its own `walk.py` rather than the IR |
| `poc/rust/gen/` | 1,014 | `rust_facade.py`, including an emitted prost impl for the incumbent arm |

The review counted seven independent traversal emitters and found rule
divergences between them (R-E1 to R-E8): wire-type acceptance, tag 0, `-0.0`,
map-key default, merge semantics, int sign on decode, UTF-8 policy, field order.
Every one of those is a symptom of the rules being written more than once.

**Target architecture.** One package, `poc/codec/gen/`, is the only generator.
Slice `gen/` directories keep only build and harness glue (project files, arm
lists, payload dumpers), never a wire rule, an IR or a layout derivation.

1. **One front end.** `schema/emit/shapes.py` loads `shapes.json`;
   `poc/codec/gen/ir.py` is the only IR. Retire `poc/csharp/gen/ir.py`,
   `abi_ir.py` and `poc/python/gen/walk.py`. The IR must also load the corpus
   schema (`fixed32` and every wire type the corpus uses, R-E3).
2. **One rule layer: a lowering from IR to language-neutral plans.** A new module
   (suggested `poc/codec/gen/plan.py`) turns each message into:
   - an **encode plan**: the ordered field writes in tag order (R-E8), presence
     test per field (bit comparison for floats, so `-0.0` survives, R-E3), map
     entries in the canonical form, key and value each omitted when empty (the
     manifest's form; R-E4's defect was Java writing the key unconditionally, and an
     earlier wording here, "key always written", was that defect restated: corrected
     2026-09-24 after the rust slice showed `E-map-entry-empty` rejects it), packed runs, submessages with
     the learned-width length strategy, oneof dispatch with a refusal for an
     unknown case (R-E4), unknown-field re-emission in retain mode;
   - a **decode plan**: a dispatch on (field number, wire type) where a known
     field at the wrong wire type is skipped, identically for every kind (R-E2);
     tag 0 rejected (R-E4, R-E5); packed and unpacked both accepted; merge
     semantics for repeated singular and oneof messages (R-E4); sign extension
     for int32/int64 (R-E5); group skip; recursion limit; UTF-8 policy as a
     generator option with one default (R-E7); unknown fields dropped or retained
     as a generator option (owner position 6);
   - the **ABI layout** (`ak_efix_*`, `ak_dfix_*`, presence bits, loop slots), from
     which every language's struct declaration is rendered, so no binding
     hand-declares or re-derives one (R-D2, R-E6).
   The rules in this layer are the ones every backend obeys. A backend that
   needs a rule the plan does not express adds it to the plan, not to itself.
3. **Backends render plans, nothing else.** One backend per target text:
   - Rust: the core's codec behind the C ABI **and** the core-native control,
     from the same plan, so their difference is the boundary only (R-E1);
   - C++: the native control, the binding and the ABI header;
   - C#: the managed codec and the binding, one file per message set with
     `LibraryImport` under `#if NET7_0_OR_GREATER` and `DllImport` otherwise
     (net6.0 and net48);
   - Java: the managed codec (arm R) and the JNI binding, Java 8 and 17 trees
     from one backend with a level (as today);
   - Python: the pure-Python codec and the C shim, one shim source whose
     3.7-incompatible calls sit under `PY_VERSION_HEX` conditionals (R-D4).
   A backend may choose how to express a plan step idiomatically (a Java
   `switch`, a Rust `match`), and may choose buffer strategies native to its
   runtime, but it may not add, drop or reorder a wire decision.

   **Language levels are conditional compilation inside one generated output,
   wherever the language has it**: C# `#if NET7_0_OR_GREATER` (and
   `NET8_0_OR_GREATER` where the target gains more), C++ `#if __cplusplus >=`,
   the Python C shim `#if PY_VERSION_HEX >=`. The floor and the target build the
   same generated file with different compiler settings, so one file is gated by
   the corpus at every level. Java has no preprocessor, so its Java 8 and 17
   trees remain two outputs of the one backend with a level parameter, as today;
   Rust has one level. In C++ a conditional must not change the layout of an
   installed header type (`CLAUDE.md` invariant).
4. **Incumbent arms are not generated.** The incumbent is what ArmoniK ships
   (R14), built by that ecosystem's own tool (protoc, prost-build, Grpc.Tools).
   `poc/rust/gen/rust_facade.py`'s emitted prost impl is checked against
   prost-build output; if it differs, the incumbent arm uses prost-build.
5. **Oracles stay independent, on purpose.** `schema/emit/payloads.py` and
   `corpus/emit/encode.py` are reference encoders, validated against upb and
   protobuf. They must **not** be ported onto the generator: an oracle that
   shares the implementation under test cannot catch its defects. This is the
   one exception to "one generator", and it is stated as such in `README.md`.
6. **Migration order**, each step gated by the corpus and byte identity before
   the next starts:
   1. `plan.py` plus the Rust backend for the core and core-native. Regenerate;
      the core's output must stay byte-identical on the payload set, and the
      corpus (with WP4 item 2's new vectors) must pass. Differences are
      rule fixes and are listed in the commit.
   2. C++ backend. 3. Java backend. 4. C# backend (retires its IR).
   5. Python backend (retires `walk.py`).
   Each slice agent ports its own backend onto the shared plan; the plan layer
   itself changes only through the aggregating session (`CLAUDE.md`: changes to
   existing core behaviour).
7. **Guard against regression to per-language rules.** A check in
   `poc/codec/gen/generate.py --check` that fails if a backend module imports
   anything from the schema other than the plan (backends take plans, not IR
   messages), and a corpus run over every generated codec in every language as
   part of each slice's gate.

Done when: `poc/codec/gen/generate.py` (one command) emits every generated file
of every slice; `--check` finds no drift; no slice `gen/` directory contains a
wire rule, an IR or a layout derivation; every generated codec in every language
passes the full corpus, in both unknown-field modes; the Rust `ffi` and
`core-native` arms are rendered from the same plan. The count of independent
traversal emitters, recorded as a fact in `README.md`, goes from seven to one.

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
- It does not add TLS, retry, metadata or deadlines to the core's transport
  (owner position 3). Client streaming is added only for the optional
  streamed-upload cell (W11 item 22), and only if it is small; otherwise the
  implementer records the gap and asks the owner.
- It does not choose between options. It does not define "material". It does
  not decide unknown-field retention; it makes both behaviours measurable.
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
- Ubuntu 24.04 LTS ("noble") ships `python3` 3.12.3 (package
  `3.12.3-0ubuntu2.1`). Source: https://packages.ubuntu.com/noble/python3
  (retrieved 2026-09-24). The owner first chose Ubuntu 26.04's CPython (3.14.3,
  https://packages.ubuntu.com/resolute/python3) and moved to 3.12 for
  compatibility with Ubuntu 24.04.
- The Python slice's timed container runs used 3.11; its `STATE.md` records that
  every arm also builds and passes on 3.10, 3.12 and 3.13.
- CPython end-of-life dates could not be retrieved from this environment
  (python.org and peps.python.org are blocked by the network policy); they are
  not needed now that the target is fixed.

Facts that bear on the design constraints and are not in `README.md`:
- **`packages/java` builds with `release` 17.** The Java 8 floor is the owner's
  constraint for the binding, not what the package builds today. Both are
  recorded.
- **The C# worker targets net6.0, which is out of support**, and the slice
  measured .NET 8. The generated `LibraryImport` binding needs .NET 7 or later,
  so both C# floors need `DllImport`, under `#if` in the same file (WP3 item 17).
- **.NET Framework 4.8 is Windows-only**; the slice's net48 evidence was taken on
  Mono 6.8.

## 5. Effort, roughly

| WP | Who | Size |
|---|---|---|
| WP1, WP2 | aggregating session | about one session together |
| WP3 spec | aggregating session | half a session |
| WP3 harnesses | five slice agents in parallel | one session each; C++ and Java the largest |
| WP4 | core + four slices + corpus | items 1 to 4 in one session; the rest opportunistic |
| WP5 | aggregating session (plan layer, Rust backend), then each slice agent (its backend) | the largest package: about 20,000 lines of generator today across eight places; one session for the plan layer and Rust backend, then one per language backend, sequential because each is gated before the next |
| WP6 | slices, then review agents | half a session each |

## 6. Owner decisions (recorded 2026-09-24)

| # | Question | Decision |
|---|---|---|
| D1 | Python levels | floor 3.7 (correctness), target CPython 3.12, as shipped by Ubuntu 24.04 LTS |
| D2 | C# levels | floors net6.0 and .NET Framework 4.8 (correctness), target net8.0 |
| D3 | Java levels | floor 8 (correctness), target 17 |
| D4 | Unknown-field retention | not decided; both behaviours built and measured (WP3 item 21, WP5) |
| D5 | Streamed upload | worth having but not required; optional and scheduled last (WP3 item 22) |
| D6 | Traffic statistics | none exist; the report states the payload set is not weighted by traffic |
| D7 | Generator | one generator implementation, wire rules written once (WP5); crucial |

Still open, and not blocking this plan: README section 15 question 6 (Java
packaging), question 7 (audience of `REPORT.md`).

## 7. Findings register

Source angle: P plan, MN measurement native, MM measurement managed, CN
correctness native, CM correctness managed, G generator, X comparability.
Disposition column is filled in as work lands.

### A. Plan

| ID | Finding | Source | Goes to | Disposition |
|---|---|---|---|---|
| R-A1 | Maintenance case never measured; no work item, no REPORT question | P | WP1 (W12) | WP1: W12 added; REPORT question 5 |
| R-A2 | Outcome criteria undefined ("materially", aggregation, which decode column) | P | WP1 (decision rule removed) | WP1: decision rule removed; the decision is the owner's |
| R-A3 | Option "protoc codecs + core RPC" (cell B) missing | P | WP1 | WP1: option 3 in README section 13 |
| R-A4 | Core transport lacks TLS, retry, metadata, deadlines, status | P | none | closed by owner position 3 |
| R-A5 | REPORT Q2 decomposition subtracts cross-machine absolutes | P, X | WP1 (moved to campaign) | WP1: moved to the campaign (README 4.1, REPORT question 2) |
| R-A6 | ABI decision 11 marked "Blocks: nothing" but changes behaviour in four languages | P | WP2, WP3 item 21, WP5 | |
| R-A7 | Floors incomplete: net48 only on Mono; net6.0 not built; Python 3.7 undemonstrated; Rust MSRV unverified | P, X | WP3 items 17, 19, WP4 item 5, WP5 | net6.0 core-ffi passes 7aad2c2; Python 3.7 passes 92a74da; net48 compiled only (needs Windows); Rust MSRV still unverified |
| R-A8 | Payload representativeness asserted, not established | P | none | closed by owner position 7: no statistics exist, report says unweighted |
| R-A9 | No RPC arm measures encode, streaming or the worker path | P, X | WP3 items 7, 22 | |

### B. Container figures stated wrongly in the documents (all removed by WP2)

| ID | Finding | Source | Disposition |
|---|---|---|---|
| R-B1 | Java outcome-2 bullet uses withdrawn +368 and an unsourced -268 (*verified*, `README.md:1190`) | P, MM | removed with the figure (WP2) |
| R-B2 | C++ empty-call sign reversed: log says the core costs 44-89 % more (*verified*, `logs/cpp/rpc.log:694`) | MN | removed with the figure (WP2) |
| R-B3 | Python encode "0.700-0.719 of upb" is from a log the slice superseded (J26); W7 and `findings/python.md` still carry it | P, X, MM | removed with the figure (WP2) |
| R-B4 | "34x to 60x" is the pure-Python control's encode, not the core; "19.4-20.3x" from superseded log 40 | P, MM, X | removed with the figure (WP2) |
| R-B5 | Crossing table mixes forward and reverse, and different metrics across containers; C# 7.5-12 ns never measured by the slice | MN, X | removed with the figure (WP2) |
| R-B6 | README section 3 stale; corpus count 328 vs 336 | P, X | fixed in WP1/WP2 (section 3 rewritten; 336 in the manifest, 328 sealed, 8 baseline rows unsealed) |

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
| R-D1 | Length-varint wrap: hang, 4 GiB out-of-bounds span to host, panic across `extern "C"` (source *verified*, not run) | CN || confirmed and fixed: core 6ede244 (logs/rust/rd1-wrap-*.log), C++ rt.h 834705f (logs/cpp/rd1-lenwrap.log, 9 hangs and 4 ASan OOB reads before); Python host segfaulted on the old core (logs/python/89-rd1-lenwrap.log) |
| R-D2 | C++ `ak_client_opts` 3 of 6 fields; `tcp_nagle` read from stack (*verified*) | CN || confirmed and fixed 834705f: generated declaration plus size and offset asserts; rpc.log and rpcflow.log predate the 6-field struct, so their TCP rows stand (logs/cpp/rd2-history.log) |
| R-D3 | Python RPC arm ungated; failures timed as cheap successes | CM || confirmed and fixed c55a11a: 68 of 80 failure rows produced a figure before, 80 of 80 abort after (logs/python/81, 83); an OK status with a wrong body also fooled cell A |
| R-D4 | Python shim cannot compile on 3.7 | CM || fixed e7728e4/92a74da: PY_VERSION_HEX conditionals; gated on real CPython 3.7.5 (Ubuntu 18.04 package via archive.ubuntu.com) |
| R-D5 | `ffi-valtc` (C++) and pull arms (Java) not in any gate log | CN, CM || confirmed and fixed: C++ 937ae78 (every arm gated before timing; planted refusal now exits 1), Java 1c69964 (pull arms 1,389 checks x3 levels) |
| R-D6 | Encode ignores the sticky error slot | CN | confirmed and fixed c10e934 (logs/rust/rd6-sticky-*.log); stated in plan.py |
| R-D7 | C++ concurrency plants never reach the core; wrong encodes double-counted | CN || confirmed and fixed 937ae78: planted cores linked; core-only plant shows ffi 23/96 with native 0; distinct counts (22 of 48, not 44) |
| R-D8 | Rust concurrency suite mostly absent-path payloads | CN | confirmed and fixed 24dce7d: four shapes; TSan 0 warnings on the suite, 77 on the planted race (logs/rust/rd8-tsan.log, wp5-tsan.log) |
| R-D9 | Minor boundary items (see WP4 item 10) | CN, CM || u32 6ede244; core transcoders on (NULL,0), opts_word collision, guard-off scripts c10e934/24dce7d; C# leak e96e6ee; JNI null 1c69964: all confirmed and fixed |

### E. Generators (all closed by WP5, the one generator)

| ID | Finding | Source | Disposition |
|---|---|---|---|
| R-E1 | "One traversal" claim in `rust_core.py` false (*verified*) | G | fixed 77f91ee: core ffi and core-native render the same plans |
| R-E2 | Wire-type acceptance differs across emitters | G | fixed in plan.py 77f91ee for the Rust backends; other backends on port |
| R-E3 | Core generator lacks `fixed32`; `-0.0` dropped by core emitters | G | fixed 77f91ee: fixed32 in the IR (WireZoo reachable through the C ABI); -0.0 by bit comparison (no corpus row exercises singular -0.0: corpus gap) |
| R-E4 | Java arm R never ran the corpus; five rule gaps | G || fixed 287deca: arm R 688/688 on the corpus; merge, unknown oneof case, -0.0 confirmed by RunRuleGaps |
| R-E5 | Python `py_codec` decode: wire type, sign, tag 0; corpus roots unreachable | G || fixed e7728e4: pycodec 688/688; the 42 earlier failures pass |
| R-E6 | C# re-derives ABI layout; probe shares the field list it checks | G | fixed 5d4c30d: probe parses ak-abi independently and compares by name both ways |
| R-E7 | UTF-8 decode policy differs per runtime | G | one option in plan.py, reject by default (31 T-dec rows now refused) 77f91ee; C# lossy default to follow on port |
| R-E8 | Field order correct only by accident of tag numbering | G | tag order stated in plan.py 77f91ee |
| R-E9 | "One generator" is not true of the tree: seven traversal emitters | G | WP5 (consolidation), WP1 (invariant) | fixed: all five slices render from plan.py (77f91ee, f9ed1d0, 2889d87, 5d4c30d, e7728e4), consolidated 57b6180 |

### G. Found while fixing (2026-09-24)

| ID | Finding | Source | Disposition |
|---|---|---|---|
| R-G1 | upb accepts field number 0 on a message with no fields (`X-tag-zero-Empty`, `-nested-Empty`); pure-python and protobuf C++ refuse. Published as disputed | corpus agent, ca03d6d | recorded; decided by nobody, as for `U-map-entry` |
| R-G2 | Python `corpus.py` crashed on its first failing row (`NameError: UPSTREAM`), so it could only report a pass (D12) | python slice | fixed 519d0c1 |
| R-G3 | C++ corpus driver ignored the binary's exit status, so a core panic blanked every later row of every arm | cpp slice | fixed 834705f |
| R-G4 | Python `native/binding.c` restates `ak_client_opts` by hand (6 fields, matching today) and uses 3.10+ calls outside the generator | python slice | fixed e7728e4 |
| R-G5 | `ak-abi` declares `ak_queue_next`'s timeout as `i32` where the core exports `u64`; `ak_bytes` and `ak_completion` are declared twice in Rust with no layout check tying them; the remaining RPC prototypes are hand-declared in `poc/cpp/src/rpc_common.h` (cpp C34, from reading) | cpp slice | fixed: RPC ABI rendered from plan.rpc in every C header (c_abi.py) and in C#; `ak_queue_next` is u64; C# counting surface pending (D40) |
| R-G6 | After a decode error, what the host's output object contains was compared between arms and differed on 46 of 52 refused rows once the core stopped delivering groups after an error (cpp C33) | cpp slice | **ruled by the aggregating session**: after a decode error the output object is unspecified and a host discards it; conformance compares error codes on refused rows only. ABI v1 section 5 already says decode stops; WP5's plan layer states the rule for every backend |
| R-G7 | The C# binding never called `ak_init` (ABI v1 section 3); every gate passed because the shared core's default features do not include `init-guard`, so the check was off. Against a guarded core, core-ffi failed 16 of 16 | csharp slice | fixed in every binding (ak_init rendered from plan.lifecycle); every gate runs on an init-guard core with an ak_init-skipped control |
| R-G8 | Hand-written runtimes narrow or wrap the 64-bit length prefix: C# `Dec.LenEnd` cut it to `int` (an exception, counted as a pass, refused `X-len-huge`); Java arm R's `Dec.readLen` checks `pos + n > limit` in `int` | csharp, java slices | fixed: 64-bit checked lengths stated in plan.py and rendered by every backend (C# e96e6ee, Java 287deca) |
| R-G9 | Java 8 floor build broken since 5241ced (`ProcessHandle`, Java 9); no floor log was committed in that window | java slice | fixed 1c69964 |
| R-G10 | The old emitter named every nested reader `cd`, so an error two levels down was lost; the old core accepted a truncated `tasks[0].options.max_duration` (logs/rust/wp5-nested2-before.log) | rust slice, WP5 | fixed 77f91ee |
| R-G11 | Unknown fields inside an inlined singular child, a oneof message member or a map entry have no decode-side slot in the C ABI, so retain mode writes the dropped form there (16 rows; rust D34) | rust slice, WP5 | **closed**: mechanism and rules specified by the owner (ABI-v1 decision 11); implemented in the core (29d515e, e897f57) and every slice (rust 4238d58, cpp 6feff87, java efe58d1/973e5ba, csharp 8d2e7ac, python 883ae3b/acb5128); C ABI retain keeps every non-disputed unknown row in all five; controls agree across slices (2,290 positions, 315 pairs, 0 mismatches) |
| R-G12 | A recursive message (corpus `Nest`) has no finite group, so the generator refuses it from the C ABI (rust D35) | rust slice, WP5 | **owner decided 2026-09-24: no recursive messages are planned; the generator keeps refusing them from the C ABI** |
| R-G13 | Plan gaps reported by all four ports: the ABI's fixed vocabulary (error codes, `ak_str`, `ak_span`, `ak_blob`, `ak_uspan`, `ak_err`, counters, `ak_bdr_rec`), fixed entry points and export list, `AK_INIT_*` values, vtable member order and pull record numbering, RPC counting surface, map entry order, `direct` presence on encode, varint 10th-byte overflow, group nesting vs message depth, field numbers above 2^29-1 (truncated by every backend, refused by protobuf) | cpp, java, csharp, python slices | 57b6180/41eb485: vocabulary, entry points, flags, vtable order, pull records, RPC counting in plan.FIXED; map order UTF-8 bytes; field numbers > 2^29-1 refused; group limit 100. 10th varint byte: bits beyond 64 discarded, matching all three oracles (conformance, recorded by the aggregating session). Open: in-group field-number check in hand runtimes (D38) |
| R-G14 | The C header is rendered by two backends (`cpp_abi.py` and `java_abi.py`); Python and C# reuse or bypass it. `generate.py`'s guard list names none of the new backends and does not regenerate the slices' outputs; `rust_core.py` and `poc/cpp/gen/cpp_header.py` survive as adapters; `one_core.sh --selftest` fails before planting since WP5 step 1 (scratch copy lacks `ffi/corpus`); `poc/python/mech/` keeps its own generator with wire rules | cpp, java, python slices | 57b6180/3cee365: c_abi.py is the one C header backend; guard over 26 backend modules; one command regenerates every slice (--check exit 0); rust_core.py, cpp_header.py, java_abi.py deleted; one_core.sh selftest fixed; mech generator retired |
| R-G15 | `ak_err` differed between ABI-v1 (`{code, msg_len, msg}`) and `ak-abi` (`{code, detail}`) | cpp slice | settled by the aggregating session: ABI-v1 now matches the implementation |
| R-G16 | Port defects found by the corpus: Java's bulk direct path never ran, zeroed fill dropped -0.0, `Utf8View` refused ASCII after a multi-byte character; Python split packed runs over 4096 values into several records, and 3.7/3.8 shims exported no `PyInit_`; C# lacked the direct-argument parameters of `ak_encode_UploadResultDataMessage` and misdeclared `ak_bdr_count_forward` | java, python, csharp slices | fixed 2889d87/287deca, e7728e4/92a74da, 5d4c30d/7aad2c2 |
| R-G17 | Hand-written runtimes' group skip accepted field numbers above 2^29-1 (C++ `rt.h`, Java `Dec.java`, C# `Wire.cs`); two gate scripts could run on stale builds (Java core target dir reused over a `git archive` snapshot, C++ gate did not rebuild); C# declared RPC counting by hand (rust D38, D39, D40) | rust slice, WP5 step 6 | fixed: C++ 563b330/fd3ec1a, Java bb98e8f/271fdd5, C# 8dbb4b1/81ba451; limits rendered from plan constants; stale builds refused (logs/cpp/d39-stale-refusal.log, logs/java/wp5s6-d39-keys.log) |

### F. STATE hygiene (WP6)

| ID | Finding | Source | Disposition |
|---|---|---|---|
| R-F1 | C#, Java, Python, C++ `STATE.md` contradict themselves on what exists | CM, X, CN || fixed for Java 1c69964 and C# e96e6ee; C++ corrected 937ae78; Rust pending |
| R-F2 | Python `STATE.md` "concurrency still unanswered" vs suite present; `57-gc-bias.log` closing line contradicts `bench.py` | MM, CM || confirmed and fixed 46bf20f |
| R-F3 | Python P2.2 crossing counts: `STATE.md` gives 10.02 / 7.00 (C extension type) and 32 (Python storages); `logs/python/53-conformance-all-shapes.log` gives 51.67 / 51.67 against 146.68 / 131.35. Found while rewriting `findings/python.md`, which now cites the log | WP2 || confirmed and fixed 46bf20f: 10.02 / 7.00 are core crossings, 51.67 / 51.67 shim crossings; "32" had no log |

### H. WP6 re-review (2026-09-26): raised, not yet confirmed

Five `ffi-review` agents, one angle each: measurement validity (MV), implementation
correctness (IC), generator sweep (GS), comparability (CP), and CAMPAIGN.md sign
hazards (CS). The findings are unconfirmed until the owning slice answers. Where two
angles raised the same thing, both are named. "Owner" marks a finding that needs a
decision rather than a fix.

**H1. Harness defects (a fix, no decision)**

| ID | Finding | Source | Disposition |
|---|---|---|---|
| R-H1 | Python `camp_summary.py` keys neither references nor groups by build, so full-build ratios use the no-unknown incumbent and pool A/B/incumbent rows across builds | MV1 | |
| R-H2 | C# RPC: B and C create `inflight` OS threads inside the timed window (`BlockingOp`), while A and D use the thread pool. Warm-up of 32 calls, tier not read back. Rust, C++ and Python spawn threads per batch inside the window for every cell | MV2 | |
| R-H3 | C# `CoreGate` crossing gate passes when a committed row is missing, or when the expect file is empty | MV5 | |
| R-H4 | C++ RPC abort leaves earlier cells' samples in the jsonl (req 18); the gate control checks only the exit code | MV6 | |
| R-H5 | C++ runner does not propagate a `campaign_calib` failure (rc 0) | MV7 | |
| R-H6 | C# codec: both builds append to one file with no `build` field | MV9 | |
| R-H7 | C++ binding `decode_with_<root>_opts` frees the caller's unconsumed pre-allocated buffers and leaves the pointers in the caller's options: use after free on the next decode with the same options (rule 7) | IC3 | |
| R-H8 | Rule 4 delivery path unexercised in Java and Python (no corpus row puts unknowns in a oneof member); Rust has no switch to a scalar member | IC2 | |
| R-H9 | C# "0 undelivered" and Python leak checks have no must-fail twin | IC7 | |
| R-H10 | Core: `ak_parse_*` resets the context's records before the wrong-root check, so a refused parse (-8) destroys an unread earlier parse | IC5 | core change, via the aggregating session |
| R-H11 | C# host-gen codec is rendered with unknown="both" and a run-time `Retain` flag; the other four render separate drop and retain codecs (the drop arm carries capture code in C# only) | GS1 | |
| R-H12 | Rust slice `gen/rust_facade.py` generates `prost_impl.rs` with its own presence and oneof-order rules (oneofs after plain fields, not tag order); WP5 item 4's prost-build check was not done | GS3 | |
| R-H13 | The one-generator guard tests imports only, over `codec/gen` plus hand-listed glue; rust and python slice `gen/` are unguarded; `one_core.sh` names a removed file | GS4 | |
| R-H14 | Error-code and refusal divergence: Python hard-codes the code table; undeclared oneof case gives a bare `ValueError` in Python (Java, C# and the core give ABI); a selected null message member raises in Python and writes an empty body in Java and C#; C# managed numbering differs and C4 checks only that some code came back | GS5 | |
| R-H15 | Backend-local tables (`PACKED_KIND`, `RUN_FN`, `java_layout` sizes and option-struct members); packed fixed32 refused at render, not in `check_expressible`; four backends test `options.unknown == "drop"` instead of `unknown_compiled_out` | GS6-8 | |
| R-H16 | Python core-ffi runs over the C-extension facade, host-gen over the plain facade (R3: same facade objects) | CP5 | |
| R-H17 | Java cell B copies the response twice before `parseFrom`; C++ and C# parse in place | CP8 | |
| R-H18 | RPC rotation is per round with the same schedule in every launch (C++, C#, Java, Python); Python codec arm order inside a block never changes; C# JIT-check failure only warns | MV10, MV4 | |
| R-H19 | Stated facts wrong: "in-process control" in the Java, C# and Python codec suites (every arm is its own process); C# smoke `.bdn.log` figures not stripped; Python STATE says calib needs a gate; C++ `instrumentation` flag true only on a dirty tree; Java codec suite records no JIT tier (req 24); `java_pull` docs say no reverse call (grow is one); C++ says rule 5 unexercised, the other four are silent; CAMPAIGN req 26 says "both" modes | MV3, MV11, MV12, IC4, IC6, CS | |

**H2. Needs an owner decision (contract or design)**

| ID | Question | Source |
|---|---|---|
| R-H20 | Rule 1 as implemented: the core decides "discard" when the context is armed, so an entry that was empty then and refilled later is ignored (rc 0, bag lost). Keep the behaviour and narrow the rule's text, or make the core read entries live | IC1 |
| R-H21 | Rule 5 edges: exactly 2^31 is refused; LIMIT is checked only on the grow path (a host buffer with cap at or above 2^31 is accepted); above 2 GiB with no grow gives CAPACITY rather than LIMIT | IC4 |
| R-H22 | What "no-unknown" removes: C++ and Rust keep the facade's unknown-field member; Java and Python remove it; C# is reported both ways (GS2 says kept, CP3 says removed). host-gen has a no-unknown arm in Rust, C# and Java, and not in C++ or Python. C++'s installed-header layout rule may be why it keeps it | GS2, CP3 |
| R-H23 | Order of arms (req 22): a one-step rotation over 3 launches uses 3 of 6 positions and never changes adjacency, so C-retain always precedes C-drop and D-retain precedes D-drop. Options: counterbalanced order, interleaving, or accept it, stated | CS1, MV10 |
| R-H24 | Ratios across processes (req 30): JMH forks per cell, BDN runs per unit, pyperf spawns per benchmark, so a "per-round ratio" pairs different processes in Java, C# and Python. The no-unknown build is always another binary and another process in every slice, and no in-process control goes through the core. Options: per-launch medians labelled cross-process; more launches; a layout control (k relinks, or fixed alignment); both cores in one process | CS2, CS3, CP6, MV3 |
| R-H25 | CPU definition (req 21): the "or" lets an RPC cell count one thread; codec CPU is thread CPU in four slices and one process-wide value per case (GC forced by BDN included, no per-round CPU) in C# | CS4, CP2, MV4 |
| R-H26 | Content sets: SHAPES.md does not say which payloads carry them; the slices share no (payload, non-ASCII set) pair (Python P2.4 only, Rust and C# P1.2 and P2.2, C++ five payloads, Java all) | CP1, MV8 |
| R-H27 | U-* rows differ: C++ has no encode direction; Python and Java use the corpus-schema core; Java has no `accept` filter; counts 92 or 311 | CP4 |
| R-H28 | RPC transport: UDS in C# and Java, loopback TCP elsewhere; Java's "shipped" has no source in `packages/java`; C# passes `adaptive_window = 0` in shipped | CP7 |
| R-H29 | Timing scope (req 11): graph construction and the encode end state are not fixed per arm (C++ incumbent allocates a ByteBuffer and core-ffi reuses; Python core-ffi returns fresh `bytes`; Java encodes a new graph per iteration from a 32 MiB pool, the others one hot graph) | CS7, CP9 |
| R-H30 | Req 16 fixes blocking delivery for B and C only; A and D may be async | CS5 |
| R-H31 | Req 19: the gate does not count resets, runs on a counting build rather than the timed one, does not cover RPC cells, and leaves retain buffer sizing (so grow crossings) to the harness | CS6 |
| R-H32 | GC and JIT state between blocks, warm-up placement, JIT tier with no consequence (req 24, 25) | CS8 |
| R-H33 | Server identity per launch, server warm-up and connection lifetime unspecified (req 13, 17) | CS9 |
| R-H34 | Idle states, SMT inside CLIENT, and CLIENT size not fixed for the campaign (req 2, 4) | CS10 |
| R-H35 | No cell serves option 2 (host-gen codec over the core's transport), though README section 13 says option 2 depends on B-A with the generated codec | CS |
| R-H36 | RPC direction (a): Python's like-for-like row is `a+read` (upb is lazy); a mapping is needed | CP10 |
