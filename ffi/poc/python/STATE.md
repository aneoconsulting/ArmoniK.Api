# python slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | **Complete.** Every shape in `design/SHAPES.md` is in scope -- M1 through M7, all sixteen payloads, three backends, both directions, gated and counted. **The concurrency suite (ABI v1 obligation 12.5) exists and passes.** The concurrency suite and **the RPC arm, as a three-cell grid**, both exist and pass |
| **Blocked on** | nothing |
| **Floor** | still README open question 4. Not demonstrated: no python3.7 here (apt lists 3.7.17-1+noble2). Facts in `logs/python/01-environment.log` |
| **Target** | 3.11. Every arm builds and passes on 3.10, 3.12 and 3.13, and the verdict's shape holds on all four |
| **Incumbent** (R14) | `protobuf` 7.36.2 on **upb**, through **the path ArmoniK runs**: `Message.SerializeToString` and `Message.FromString`, with no size pass and no buffer writer. **Derived, not assumed** -- `verify_r14.py` generates the real `Protos/V1/results_service.proto` stub and reads the serializer off it (`logs/python/52-r14-baseline.log`) |
| **Core** | `ffi/poc/codec` (R0), reached by path, never copied. `poc/codec/gen/one_core.sh` passes |
| **R13** | this machine's rust-slice crossing is **2.1 ns** (fwd+reverse) to **2.8 ns** (forward). The composed arm also prices the boundary **in its own process and build** at the foot of every bench table, which is the better number to quote against because it shares a build |
| **How to read every absolute** | **instrumentation, not the deliverable**. Signs and size classes only; the controlled physical rerun owns the decimals. And read the allocator row below before quoting any large-payload encode |

## The five things to know

1. **The ABI is not where Python's crossing problem is, and M6 says it loudest.**
   The core's own boundary costs **0.01 to 5 crossings per element**; the
   shim-to-facade edge costs **3 to 302**. A packed run of 30 values crosses the
   ABI **once** and reads 30 Python ints one at a time on this side.
2. **The facade storage decides everything.** A C extension type the shim reads
   as a struct: 7 crossings per element on M1. The two Python storages: 22 to 29,
   and 1.5 to 4 times the incumbent.
3. **An encode absolute above ~128 KiB is a property of the process's allocator,
   not of the codec** -- 1.9x on the incumbent's P1.2, 3.8x on its P2.4. The bench
   now pins it; work unit 2's headline was withdrawn because of it. `allocator.py`,
   `logs/python/55-allocator.log`, JOURNAL J26.
4. **`FromString` does not construct Python objects and caches nothing.** The
   bare decode call and the like-for-like decode are two different questions, and
   the report has to pick one deliberately. Decision 13's borrowed span is
   available to this facade and **not** already taken by the incumbent.
5. **Nothing in the ABI needed extending for M3 to M7.** oneof, explicit
   presence, packed scalars and bulk bytes are a binding exercise in Python. What
   the ABI already had: `<oneof>_case` carrying the active member's tag, a
   presence word, `ak_run_i32/i64/f64/u8`, and section 8's direct argument.

## The measurement, every payload (`logs/python/62-all-shapes-py3.11.log`)

Three processes on 3.11, the target interpreter; the range is their spread.
`63-all-shapes-every-interpreter.log` has 3.10, 3.12 and 3.13 at one process
each, and what those establish is that the SHAPE of the answer holds, not a
number. `mech/summarise.py <log>` regenerates every table here.

The arm is `core-ffi / C ext type` throughout -- the only facade storage that
competes (see below). **The allocator is pinned** (`allocator.py`), so these
figures do not depend on which payloads precede them in the run.

| | encode | decode, the call | decode **+ read every field** |
|---|---|---|---|
| P1.1 M1, 4 elements | 1.301 - 1.325 | 1.354 - 1.438 | 0.725 - 0.731 |
| **P1.2** M1, 1,000 | **1.171 - 1.184** | 1.768 - 1.805 | **0.711 - 0.733** |
| P1.3 M1, absent path | 1.326 | 5.80 - 5.89 | 0.765 - 0.788 |
| P2.1 M2, 1 element | 1.662 - 1.886 | 1.819 - 1.890 | 0.662 - 0.670 |
| **P2.2** M2, 500 -- **read this one first** | **1.370 - 1.386** | 2.355 - 2.438 | **0.645 - 0.652** |
| P2.3 M2, repeated strings | 1.931 - 1.941 | 2.157 - 2.183 | 0.714 - 0.719 |
| P2.4 M2, the length-placeholder move | 2.476 - 2.507 | 2.128 - 2.157 | 0.770 - 0.792 |
| P2.5 M2, absent path | 1.518 - 1.826 | 2.122 - 2.138 | 0.637 - 0.642 |
| P3.1 M3, oneof + `optional` | 1.463 - 1.488 | 2.58 - 2.62 | 0.430 - 0.464 |
| P4.1 M4, the adapter site | 2.151 - 2.173 | 1.93 - 1.96 | 0.679 - 0.692 |
| P5.1 M5, 36 B | 2.43 - 2.67 | 1.08 - 1.19 | 0.796 - 0.823 |
| P5.2 M5, 64 KB | 1.30 - 1.39 | 0.853 - 0.966 | 0.526 - 0.562 |
| P5.3 M5, 1 MB | 1.075 - 1.087 | 0.952 - 0.981 | 0.402 - 0.419 |
| **P5.4** M5, **4 MB** | **1.017 - 1.031** | **0.962 - 0.967** | **0.455 - 0.472** |
| P6.1 M6, packed | 2.785 - 2.796 | 3.799 - 3.819 | 0.639 - 0.653 |
| P7.1 M7, 98 bytes | 1.99 - 2.09 | 1.54 - 1.67 | 0.90 - 0.94 |

**Encode loses everywhere and by a bounded amount**: 1.02 to 2.8 of the
incumbent, worst on the two shapes with the most per-element Python reads (M6's
packed runs, M4's ten `TaskOptions` strings) and best on the one with fewest
(M5's blob, at parity). **The bare decode call loses too.** **The like-for-like
decode wins everywhere**, 0.40 to 0.94, and that is the column that compares the
same work.

### Why there are two decode columns, and what the floor says

`FromString` does not construct Python objects. It parses into a upb arena and
materialises a `str`, an element wrapper or a nested message only when something
reads it -- **and it caches nothing, so it re-materialises on every read.** The
floor arm is what makes this impossible to argue with:

| | floor: N bare facade elements + one copy of the input | as a fraction of upb's whole decode |
|---|---|---|
| P1.2 | 1,000 `ResultRaw` | **0.805 - 0.827** |
| P1.3 | 300 `ResultRaw` | **8.64 - 8.80** |
| P3.1 | 200 `Probe` | **1.93 - 2.00** |
| P5.4 | no elements, one copy of 4 MB | **0.974 - 1.008** |

Constructing the objects, with no parsing at all, already costs four fifths of
upb's entire decode on P1.2, twice it on P3.1 and nine times it on P1.3. A decode
cannot cost less than producing what it produces, so **upb is not producing it**.
P5.4's row says the other half: a 4 MB decode is one copy of the input for
everyone, and there is nothing there for anyone to win.

### The re-read, and the sharpest number in the slice

A **second** full read of the *same* decoded message, both sides:

| | upb | facade / C ext type | |
|---|---|---|---|
| P1.2 | 4.61 - 4.82 ms | 3.06 - 3.13 ms | 0.65 |
| P2.2 | 13.7 - 14.2 ms | 7.17 - 7.45 ms | 0.52 |
| P6.1 | 2.01 - 2.05 ms | 0.64 - 0.67 ms | 0.32 |
| **P5.4** | **365 - 380 us** | **1.09 - 1.12 us** | **0.003** |

P5.4 is a factor of **330**. The facade holds one `bytes` object and reading it
again is an attribute load; upb re-materialises four megabytes on every read of
`data_chunk`. **A caller that reads its response twice pays upb twice and the
facade once**, and on a bulk field it pays the whole payload twice.

### The facade storage decides everything, and only one of three works

P2.2 encode, all three storages, same core and same shim:

| storage | / upb | crossings per element |
|---|---|---|
| **C extension type**, read as a struct | **1.370 - 1.386** | **10.02** |
| `__slots__`, via `PyObject_GetAttr` | 2.230 - 2.270 | 32 |
| plain class, via `PyObject_GetAttr` | 2.420 - 2.438 | 32 |
| `pyacc`, a Python-level accessor CALL (README 9.1's premise control) | 6.10 - 7.16 | 32 |
| `pycodec`, R3's no-boundary control | 33.9 - 34.4 | 0 |

R9 said in advance that a pure-Python control may lose by an order of magnitude,
and it does: 34x on M2 and **60x on M6**, where a packed run is 6,000 varints
written by interpreted code. That is a result, not a defect -- it says the codec
question in Python is native against native, and README outcome 2 ("generate the
codec into the host") is off the table for Python in a way it is not for Java.

### The boundary, priced in this process and this build

**1.82 ns forward, 2.43 ns forward+reverse**, against the Rust slice's 2.1-2.8 ns
on the same machine (R13). Both are noise next to the 7-to-302 shim-to-CPython
crossings per element above, which is the whole finding restated.

## Concurrency: obligation 12.5, and README section 9's actual question

`logs/python/56-concurrency.log`. No slice in the branch had one, and 12.5 is the
obligation with the most evidence behind it and the least existence.

**Correctness passes on every row.** Two payload shapes (P1.2, a leaf element;
P2.2, a non-leaf with a map), threads in sequence as well as together at 2 and 4,
per-thread facades **and one facade shared across every thread** -- the analogue
of the shared client the rust slice tripped over -- every encode asserted against
a reference rather than counted. Zero wrong bytes anywhere.

The reference is each arm's OWN output taken before any thread starts, with a
column saying whether that equals the canonical bytes. It is not the manifest:
`SerializeToString` does not sort map entries, so the incumbent legally writes
another accepted form on every M2 row, and comparing to the manifest reported
the incumbent as producing wrong bytes under concurrency when the fact is about
map ordering. `conformance.py` adjudicates that; this suite must not re-litigate it.

**The GIL, and the control is what makes it mean anything:**

| threads | 1 | 2 | 4 |
|---|---|---|---|
| P1.2 encode, upb | 1.00 | 0.95 | 0.96 |
| P1.2 encode, core-ffi / C ext | 1.00 | 1.01 | 0.96 |
| P2.2 encode, upb | 1.00 | 0.99 | 0.99 |
| P2.2 encode, core-ffi / C ext | 1.00 | 1.03 | 1.00 |
| **control: CPU-bound work that RELEASES the GIL** | **1.00** | **2.08** | **3.96** |

The control is `hashlib` on a 4 MB buffer, which drops the lock above 2 KiB. It
scales 3.96x at four threads, so the flat 1.00 above is **the lock and not the
machine**. Neither codec gains anything from threads, and they are equally flat:
the composed arm cannot release the GIL, because the core calls back into the
host for every element and every one of those reads a Python object -- and it
costs nothing relative to the baseline, because `SerializeToString` cannot
release it either. **A native codec behind a thread pool buys nothing in this
host, whichever codec it is.** That is README section 9's question answered for
Python, and the answer is about CPython rather than about this design.

Not covered: decode under threads, and more than 4 threads on a 4 vCPU box.

## Crossings, counted in both halves (`logs/python/53-conformance-all-shapes.log`)

Per element, C extension facade. The core counts its own and the shim counts what
it does to CPython; neither half can count the other's (R5).

| payload | what it is | shim -> CPython | core fwd | core rev |
|---|---|---|---|---|
| P1.2 | M1, a leaf element | **7.00** enc / **7.00** dec | 0.01 | 0.00 / 0.01 |
| P2.2 | M2, a non-leaf element with a map | 10.02 enc / 7.00 dec | 5.02 | 5.00 / 7.00 |
| P3.1 | M3, a oneof and 3 `optional` scalars | **5.40** enc / **3.15** dec | 0.01 | 0.01 |
| P4.1 | M4, the adapter site | 23.00 enc / 23.00 dec | 1.01 / 0.01 | 1.00 / 3.00 |
| P5.x | M5, bulk bytes, **36 B to 4 MB** | **3.00** enc / **3.00** dec | 1.00 | 0.00 / 1.00 |
| P6.1 | M6, 5 packed fields x 30 values | **302** enc / **152** dec | 5.01 / 0.01 | 5.00 / 7.00 |
| P7.1 | M7, two interleaved repeated fields | 2.00 enc / 2.00 dec | 0.50 / 0.17 | 0.33 / 1.17 |

Three of these are worth reading twice.

**P3.1 is CHEAPER than M1.** A oneof reads the discriminant and then exactly one
member, so five wire-level alternatives cost two reads rather than five. The
by-value group's decision to inline every member beside a `<name>_case` rather
than union them (ABI v1 section 6) costs group SIZE and buys this.

**P5.x is constant in the payload size.** 3 crossings whether the blob is 36
bytes or 4 megabytes, because section 8's direct-argument path hands the bytes to
the call beside the group. Two caveats, and the second is a finding: the cost of
a 4 MB encode is the copy, not the boundary, at any crossing count; and **CPython
has nothing to pin**, since a `bytes` object's buffer is already a stable address
for as long as the facade holds it. So the direct path should buy this slice
nothing, which is the Rust slice's conclusion reached independently in a second
language. Section 8's 0.16-to-0.34 figure remains a JVM claim resting on one
slice, and now two slices have failed to reproduce its mechanism.

**P6.1 is the packed control doing its job.** 302 shim crossings per element
against 5.01 core forward calls: five packed fields, one `ak_run_*` each, and 150
Python integers read individually on this side of the boundary. The ABI boundary
disappears for a packed run and the CPython boundary does not move at all. That
is README 9.1's three layers in one row.

## Correctness

It gates everything: `bench.py` runs `conformance.py` as a subprocess and refuses
to time anything if it fails.

- Encode and decode **byte-identical to `ffi/schema/generated/manifest.json` on
  all sixteen payloads**, every arm, on 3.10 / 3.11 / 3.12 / 3.13. The absent
  paths (P1.3, P2.5) are in the gate, not beside it.
- **P7.1 is the one payload with an allowance, and `design/SHAPES.md` grants it in
  advance**: no writer that emits a repeated field contiguously can produce those
  bytes. Checked as a permutation of the same (tag, wire type, body) triples, and
  additionally as parsing to the same message under the incumbent's parser.
- Decode checked **twice**: by re-encoding to the same bytes, and field by field
  against the incumbent, driven from **the incumbent's own descriptor** so a field
  this slice forgot is a failure rather than an omission nobody sees. The oneof is
  compared through `WhichOneof` against the facade's `<oneof>_case`, and explicit
  presence through `HasField` against `is None`.
- ABI version and the group layout facts checked at import (section 10,
  obligation 12.3), and the boundary proved from the artifact in both builds (R5).
- **126 of 336 corpus vectors** (W8), with projections, accepted-encoding sets and
  the reject vectors.

## The corpus (W8): 126 of 336 rows, and an incumbent defect

Scope is now every root in `walk.ROOTS`, six of them, rather than
`ListResultsResponse` alone. `logs/python/70-corpus-subset.log`.

| arm | C1 parse | C2 project | C3 re-encode | C4 refuse |
|---|---|---|---|---|
| `pycodec / plain` | 126/126 | 123/123 | 126/126 | 2/2 |
| `core-ffi / C ext type` | **126/126** | **123/123** | **126/126** | 2/2 |
| `core-ffi / plain` | **126/126** | **123/123** | **126/126** | 2/2 |

**Every obligation, every arm, against the three-oracle corpus.** C2's denominator
is 123 and not 124 because the corpus withdrew `U-map-entry`'s projection rather
than deciding it -- see below.

**D7 is closed.** This slice reported it, the core fixed it (`Dec::skip` now takes
the tag and recurses to a MATCHING `END_GROUP`, bounded at 100 nests), and
rebuilding against the fixed core took both `core-ffi` arms from 123/126 to
126/126 on C1 and C3 -- `U-root-group`, `U-nested-group` and `U-oneof-group`, the
third of which only became reachable when M3's root came into scope.

**Byte identity against the manifest could never have found it.** No proto3
schema can express a group, so nothing the generator emits produces one, and a
gate built from the same description as the codec is blind to the whole class.
The same is true of the incumbent defect below. Both were reachable only because
a second reader existed, which is `corpus/CONTRACT.md`'s argument for itself,
twice.

**The C2 miss became a corpus change.** It was reported rather than folded into
the pass count, the corpus now runs three oracles, and it withdrew that vector's
projection instead of deciding it. The finding underneath is an incumbent defect,
found the way CONTRACT.md says a first consumer finds things:

> **protobuf 7.36.2 on upb DROPS an entire map entry that carries any unknown
> field.** The map comes back empty. The same version's pure-Python backend keeps
> the entry, and so do this slice's core and its pure-Python control. Isolated on
> a two-field message rather than inferred from the vector. A map entry is a
> message on the wire, so an unknown field inside it is skipped and the entry
> survives -- which is what the vector's own `why` says it tests. **upb is the
> default, and it is what R14 measures against.**

The corpus's own projection recorded those entries as `_unknown` content of
`TaskOptions`, so its reader did not treat tag 1 as the map either. Three readers,
three answers; this slice is with the reference implementation, and the corpus
resolved it by marking the row disputed rather than by picking a winner.

**ABI v1 decision 11, answered for python: this slice DROPS unknown fields.** The
core carries `ak_unk_f` vtable slots and this shim passes NULL for every one, so
the drop is the binding's choice and not a limit of the ABI.

## What exists

```
gen/generate.py        THE generator (R1, R0). Imports poc/codec/gen/ir.py and the
                       cpp slice's cpp_header.py read-only; writes nothing outside
                       this slice. `--out DIR` emits elsewhere for a dry run
gen/walk.py            the one field walker. A shape outside SCOPE raises, and a
                       oneof member and an `optional` scalar come through with
                       cardinalities nothing had a case for until it grew one
gen/py_facade.py       the facade, plain and __slots__, with <oneof>_case
gen/py_codec.py        the pure-Python codec, encode and decode (R3's control)
gen/py_binding.py      the COMPOSED arm's shim: 3 backends x 2 directions
gen/py_shim.py         work unit 1's no-core C encoder, kept so its column stands
gen/py_values.py       facade objects carrying the manifest's values
gen/out/               emitted and COMMITTED, ak_abi.h included
native/binding.c       the module wrapper. Names no message and no field
build.sh               R0 check, R1 check, the core, the shim, the R5 boundary proof
run.sh                 the whole slice end to end; writes logs/python/5x, 6x, 7x
conformance.py         R2 and R5: byte identity both directions, layout facts,
                       crossing counts (the counting pass in its own process)
corpus.py              W8: 126 of 336 rows, every root this scope can reach
allocator.py           the control for the large-payload encode absolute
verify_r14.py          derives the baseline from Protos/V1
bench.py / arms.py     the arms and the timing
mech/                  work unit 1, frozen: the mechanism, primitive and storage
                       microbenchmarks, and the harness every work unit shares
```

**Reproduce**: `./build.sh <python>...` then `./run.sh <target-python> <others>...`.
**Verify from a clean clone at another path** before calling a work unit done --
that is how D4 was found and it is a step, not an idea.

## Open defects

| # | where | what |
|---|---|---|
| D1-D6 | this slice | **all fixed.** The forward arm above 2^63; a floor arm that copied nothing; a `.gitignore` that ate the generator; an absolute path in the generated tree; a nested length bounds-checked against the whole buffer; an unknown GROUP field that raised. Every one found by something running |
| D8, D9 | this slice | **fixed** (work unit 3, M2): decode constructed a fresh inlined child and discarded the one a run had made (ABI v1 decision 10 in the flesh); and the like-for-like reader walked nested messages with `dir()`, which made the facade's re-read look 2.8x upb's on M2 when it is 0.49x |
| **D10** | `gen/py_codec.py` | **fixed.** The pure-Python codec wrote a map entry's KEY unconditionally. Both members of the pair message are implicit-presence leaves, so an entry whose key and value are both empty is an EMPTY entry. Invisible on every manifest payload, where no key is empty; found by `E-map-entry-empty`, the corpus vector that exists for it. The shared core had it right |
| D7 | `poc/codec` -- reported here, **fixed there** | `ak_decode_*` returned `AK_ERR_MALFORMED` on an unknown field of the deprecated GROUP form, on three `expect: accept` vectors. **Closed**, and re-gated from this slice: 123/126 to 126/126 on both `core-ffi` arms |
| **U1** | **the incumbent, not this branch** | **protobuf 7.36.2 on upb drops a whole map entry that carries an unknown field.** Isolated, not inferred. The same version's pure-Python backend keeps it. It is a data-loss bug in the library ArmoniK runs, found by this slice as the corpus's second consumer |

Also fixed and worth naming because it was invisible: **`corpus.py` had stopped
running altogether.** The M2 commit gave the binding a root argument and renamed
`arms.CEXT`, and nothing noticed, because `run.sh` never ran it -- the corpus pass
was run by hand once and its committed log outlived the code that produced it. It
is step 4 of `run.sh` now. A log whose script no longer runs is worse than no log.

## Requests to the aggregating session

1. **U1 is new and belongs in the report.** An incumbent that silently drops map
   entries carrying unknown fields is a migration argument on its own, and it is
   the kind of thing only a second implementation finds. Two backends of one
   library at one version disagree; the default one loses data.
2. **D7 is closed and confirmed here** -- 123/126 to 126/126 on both `core-ffi`
   arms after rebuilding against the fixed core. `corpus.py`'s upstream-defect
   table is emptied rather than left carrying it, so a regression would be red
   again.
3. **Section 8's direct-argument path has now failed to reproduce in a second
   language.** Rust said there is no pinning to avoid; Python says the same, for
   the same reason -- a `bytes` buffer is already a stable address. The
   0.16-to-0.34 figure is a JVM claim resting on one slice, and the report should
   say which hosts it does **not** apply to rather than leaving it general.
4. **The allocator hazard is cross-cutting, and it is the same mechanism as the
   C++ slice's C16.** Same subsystem, glibc's release of large blocks; different
   manifestation. C16 is a transient outlier ROUND on a decode that amortises
   after the first few messages; this is a **steady-state** factor of 1.9 to 3.8
   on an ENCODE, in every round, that never amortises within a payload -- only a
   *larger* allocation elsewhere in the process lifts it. Two knobs there, one
   here: raising `M_MMAP_THRESHOLD` alone recovers nothing, `M_TRIM_THRESHOLD`
   alone a third, `M_TOP_PAD` alone all of it, so what is being paid for here is
   the heap being TRIMMED between calls rather than the buffer being mmap'd. Worth
   one section, not two.
5. **README 9.1's storage parenthetical is still wrong in its middle term**:
   `__slots__` is not faster than a plain class from a C shim.
6. **Decision 13 needs its Python premise corrected.** upb copies on **every**
   attribute read and caches nothing, so the borrowed-span option is open here and
   the incumbent has not taken it. What a borrowed Python string *is* has no
   draft, and that is the blocker rather than the measurement.
7. **Nothing in `design/SHAPES.md` needs changing.** Every payload hash reproduced
   first time in a sixth independent encoder, P7.1 included by its own rule.

## The pull family (ABI v1 7.1), and why this slice is where it buys least

Recorded as a hypothesis, per the aggregating session's note, not measured. Every
decode figure in this slice is a **push** figure. The reason to expect pull to buy
little here is in the numbers above: this slice's reverse calls are C-API calls on
primitives at 2.83 to 20.54 ns, not interpreter re-entries, and the composed arm
makes **zero** per field -- the core's reverse count is 0.01 per element on M1 and
7.00 on M2, against a shim-to-CPython cost of 7 to 302. Pull removes upcalls; the
upcalls are not what this host pays. If pull is priced anywhere, price it where
the reverse call re-enters an interpreter.

## What is not measured

- **Concurrency**, so README section 9's actual question -- can it survive the
  GIL -- is still unanswered, and ABI v1 obligation 12.5 has no python row.
  `Py_BEGIN_ALLOW_THREADS` at 34.2-34.7 ns is the only input to it.
- **The server side of an RPC**, and an encode-side RPC arm. Both cells decode at
  the client; making the server decode is a different experiment.
- **Streaming, TLS, a real network, failure injection.** `design/SHAPES.md` lists
  these as out of the RPC arm and they are.
- **Allocation per operation**, in any arm. Only time, and now the allocator's
  effect on time.
- **The corpus beyond this slice's roots**: 210 of 336 rows, including every
  `Surrogate` vector (the transcode pair) and every `WireZoo` one.
- **The floor (3.7) and free-threaded CPython.** Neither is on this machine and
  free-threaded is not installable here at all.
- **Decision 13's borrowed-span arm.** Priced as an opportunity, not built.
- **abi3 on the composed arm.** Work unit 1 priced abi3 on the primitives; the
  shim is built full-API only.
- **The unknown-field bag.** The core has `ak_ufix_*` groups and `ak_unk_f` slots
  and this shim passes NULL, so retention is unpriced here.
- **A readable oneof discriminator.** The facade stores the active member's TAG,
  because that is what the group holds and it costs an int compare where a name
  would cost a string compare. A facade holding the member's NAME is more
  idiomatic and is not priced.
- **The pull decode family** (above).

## The RPC arm, as a grid (`logs/python/80-rpc-grid.log`)

"The host's stack against the core's" moves the codec and the transport at once,
so the arm is three cells against **one** grpcio server that returns
pre-serialised bytes and never encodes. The server is therefore in no difference,
and the only thing that changes between A and B is which client sends the call.

| cell | codec | transport | what it is |
|---|---|---|---|
| **A** | upb | grpcio | the incumbent, end to end |
| **B** | upb | **the core** | **B - A is the TRANSPORT difference** |
| **C** | the core | the core | **C - B is the CODEC difference** |

Both arms decode at the client, which is stated rather than hidden: an
encode-side RPC arm would need the server to decode and is a different
experiment.

**Three builds of one core now exist in this slice** -- plain, counting, and the
`rpc`-feature one that pulls tonic and tokio in -- and the first of them loaded in
a process satisfies the others' `NEEDED` entry by soname. `AK_FFI_MODULE` chooses
which, before `arms` is imported, because importing `arms` is what loads a shim.
The first draft of `rpc.py` got this wrong in the direction that fails loudly: a
shim whose section 9 symbols resolved to a core that has none, refused at import.

### What the transport costs, and what the codec costs

CPU per RPC is the headline; wall clock is beside it because R9's hazard moves
wall clock and not CPU. P2.2, 540,422 bytes, over a Unix domain socket, at the
**shipped** configuration.

CPU per RPC, P2.2 (540,422 B), **shipped** configuration, blocking delivery, in
microseconds. Every row is the same server and the same bytes.

| transport | in flight | A upb+grpcio | B upb+core | C core+core | **B - A** | **C - B** |
|---|---|---|---|---|---|---|
| UDS | 1 | 2,788 | 2,653 | 5,849 | **-135** | +3,195 |
| UDS | 8 | 3,284 | 3,017 | 6,166 | **-267** | +3,149 |
| UDS | 16 | 3,528 | 2,999 | 6,207 | **-529** | +3,208 |
| TCP | 1 | 2,917 | 2,494 | 5,869 | **-422** | +3,375 |
| TCP | 8 | 3,430 | 3,320 | 6,877 | **-110** | +3,557 |
| TCP | 16 | 3,523 | 3,302 | 6,547 | **-221** | +3,245 |

**B - A is negative in all six: the core's transport is 3 to 15 percent cheaper in
CPU per RPC than grpcio's, with the codec held still.** That is README section
13's outcome 2 priced on its **transport** half, and it is the opposite sign to
its codec half, which this slice already settled -- a pure-Python codec loses to
upb by 34x to 60x. **Outcome 2 is off the table for Python's codec and on it for
Python's transport.**

**C - B is +3.1 to +3.6 ms and dominates.** The transport saving is real and about
a tenth the size of what the facade's decode costs on this payload. A report that
quoted only "the core's stack against the host's" would have netted a small win
against a large loss and called the result a wash; the grid is what separates them.

The no-decode floor is 1.69 to 2.35 ms of the 2.8 to 3.5 ms that cell A costs, so
**the transport is roughly two thirds of the incumbent's cost per call**. An
in-process codec ratio overstates what an RPC caller feels, which is the same
shape the C# slice reported (10 percent of CPU per call against 25 percent
in-process).

**Loopback TCP is not slower than the Unix socket here** -- the two are within each
other's spread on every row, and TCP is cheaper on three of six. On this container
`SHAPES.md`'s reason for preferring a UDS, that it removes the TCP/IP stack from
both arms, does not show up as a cost worth removing.

### The three deliveries, and why Python's is the queue

ABI v1 section 9 offers blocking, a completion queue and a callback, and on this
host they are not interchangeable for a reason that is CPython's rather than
borrowed from the JVM:

- the **queue**'s drainer is a Python thread that drops the GIL while it waits in
  `ak_queue_next` and takes it back to hand the bytes over. **No thread the core
  owns ever touches a `PyObject`;**
- the **callback** arrives on a tokio worker, a thread CPython has never seen,
  which must `PyGILState_Ensure` before it can do anything and release after.

**And the callback's GIL acquisition is not measurable here.** Across 24
comparisons -- two cells, two transports, two configurations, three thread counts
-- the callback/queue ratio runs from **0.855 to 1.130** with a median near 1.03,
and it straddles 1.0 in five of them. There is a weak tendency for the callback to
cost a few percent more and it is not stable across configurations.

**I reported 19 percent at 16 in flight from a single run and it does not
reproduce.** That figure came from a run that aborted before finishing, and the
complete run puts the same comparison at 1.3 percent. Withdrawn.

The result is better than the one I withdrew, because it says something the JVM's
number would not have predicted: `PyGILState_Ensure` from a foreign thread costs
a few hundred nanoseconds, and an RPC carrying 540 KB costs three to six
milliseconds, so the acquisition is four orders of magnitude below the signal and
**the choice between queue and callback is not a performance question in Python**.
The queue is still the right default here, for reasons that are not speed: the
drainer is a thread CPython already knows, nothing the core owns touches a
`PyObject`, and there is no attach step to get wrong. On the JVM, where an
uncached upcall is near 300 ns against much cheaper calls, the same choice is a
performance question. **That is the finding: the mode matters where the call is
cheap, and this host's calls are not.**

### Flow control, established rather than relayed

Five configurations, each run with the core's own `flowctl` and `bdp_estimator`
tracing, reading what the CORE printed rather than what was passed to it. Neither
the grpc-java answer (a set window disables auto-tuning) nor the .NET one (a floor
that doubles to a 16 MiB cap, connection window hardcoded at 64 MiB) transfers.

1. **The static default stream window is 65,535.** The 4 MiB grpcio reaches by
   default is BDP auto-tuning, not a large default.
2. **`grpc.http2.lookahead_bytes` is a FLOOR, not a cap.** Asking for 65,535 still
   yields 4 MiB while probing is on, so a slice that pinned a window and called it
   pinned would be reporting the value it passed rather than the one in force.
3. **Setting a window does NOT turn BDP probing off**, which is the opposite of
   grpc-java. Pinning takes both arguments.
4. **There is no channel argument for the CONNECTION window at all.** The
   separate-knob trap cannot be reached from Python: a caller cannot set it.

**And the 4 MiB window is what ArmoniK INTENDS, not what it ships.**
`packages/rust/armonik-transport`'s `ClientConfig` carries timeouts, a rate limit,
keepalive, the HTTP/2 ping settings and a max header list size, and nothing for
either window; `packages/csharp` cannot set one at all. So the stack-default rows
are the shipped configuration and the pinned rows are the intended one.

**Nagle does not reach a Python caller.** The rust slice found a 40 ms delayed-ACK
artifact in tonic's own test server; the test that identifies it is that a SMALL
response costs MORE than a large one. On grpcio it does not: monotone in size on
both transports, nothing near 40 ms. The C core sets `TCP_NODELAY` itself, and
there is no `grpc.tcp_nodelay` in this build for a caller to have got wrong.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| `00-r13-rust-crossing.log` | `ffi/poc/rust` unmodified, 3 processes | **R13**: this machine's rust crossing, 2.1 to 2.8 ns |
| `01-environment.log` | this container | Which interpreters exist, that no free-threaded build does or can, that 3.7 is absent but apt-reachable and its incumbent still exists. README open question 4's facts |
| `10-build.log`, `20-conformance.log` | work unit 1 | The mechanism arms built and gated on 3.10 - 3.13 |
| `30-mechanism-py3.11.log`, `31-mechanism-all.log` | 3 processes / 4 interpreters | Work unit 1: the mechanism, primitive, abi3, content-set and storage tables |
| `40-codec-py3.11.log`, `41-codec-all.log` | work unit 1, no core | The shim-to-facade edge priced on its own, and README 9.1's premise control |
| `50-build-wu2.log`, `51-conformance-wu2.log` | work unit 2, M1 only | **SUPERSEDED** by 54 and 53. Kept because work unit 2's tables cite them |
| `52-r14-baseline.log` | `Protos/V1/results_service.proto` | **R14 derived**: the production path is `SerializeToString` / `FromString` |
| `53-conformance-all-shapes.log` | 3.10 - 3.13, all 16 payloads | **R2** both directions against the validated manifest; the layout facts; **R5's crossing counts in both halves** |
| `54-build-all-shapes.log` | gcc 13.3 `-O2 -Werror`, cargo 1.94.1 | Built for 3.10 - 3.13; R0's one-core check; **R5's boundary proof from the artifact** |
| `55-allocator.log` | 2 subprocesses, cold and warm | **The allocator owns the large-payload encode absolute**: 1.9x on the incumbent's P1.2, 3.8x on its P2.4, and every decode clean |
| `56-concurrency.log` | 3.11, 1/2/4 threads, 2 shapes | **ABI v1 obligation 12.5**, and the GIL against a control that releases it |
| `60-composed-py3.11.log`, `61-composed-all.log` | work unit 2, M1 only | **SUPERSEDED** by 62 and 63, and their P1.2 ENCODE rows are wrong for the reason in JOURNAL J26: the allocator was cold. The M1 decode rows stand |
| `62-all-shapes-py3.11.log` | 3.11, 3 processes, allocator pinned | **The measurement**: encode, decode, decode+read, re-read and the in-process boundary, on all 16 payloads |
| `63-all-shapes-every-interpreter.log` | 3.10 - 3.13, 1 process each | The same: the verdict's shape holds across interpreters |
| `57-gc-bias.log` | 3.11, GC off against GC on | **D11**: the collector discounts the FACADE's decode by 1.09 to 1.26 and nothing else. Why `bench.py` keeps it off and prices it here |
| `80-rpc-grid.log` | 3.11, cells A/B/C, 2 transports, 3 configurations | **The RPC arm**: the transport difference, the codec difference, the three deliveries, the flow-control table and the Nagle probe |
| `70-corpus-subset.log` | 3.11, 126 of 336 vectors | **W8**: C1 to C4, the accepted forms written, decision 11 answered, D7 re-gated, and the upb map-entry defect |

## Slice-specific notes

- A reverse call into Python must hold the GIL, so the drafted ABI's per-field
  upcall is the worst possible shape here. Measured: 39.9 - 40.5 ns against
  2.83 - 20.54 for a C-API primitive, and the composed arm makes **zero** of them
  per field.
- `packages/python` reads no transport environment configuration today, so
  configuration homogeneity is a pure gain rather than a migration. Untouched.
- Floor and target may be different code, gated at import. Untested: there is no
  floor build.

## Next step

Every work item on this slice's list is done. What is left is optional and each
item says what it would answer.

1. **Decision 13's borrowed-span facade**, if the aggregating session wants the
   option priced rather than only noted. This slice has the strongest case for it
   of any host measured: P5.4's re-read is a factor of **330**, because upb
   re-materialises four megabytes on every read and a borrowed span would not.
   The premise is corrected in the document; what a borrowed Python string *is*
   still has no draft, and that is the blocker rather than the measurement.
2. **`ctypes` and `cffi` in the RPC regime** (README 9.1). Two crossings per call
   rather than one per field is where their 165-to-170x callback cost should stop
   mattering, and the grid now has the harness to test it: cell B's client is
   already schema-free, so swapping the binding mechanism under it changes one
   thing.
3. **Decode under threads**, which the concurrency suite does not cover, and an
   encode-side RPC arm, which needs the server to decode.
