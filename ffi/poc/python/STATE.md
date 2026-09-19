# python slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | **work unit 1 complete**: the binding mechanism and the facade storage are each chosen by microbenchmark, the premise of README 9.1 is settled for encode, and the whole thing is byte-identical to the validated manifest on P1.1, P1.2 and P1.3. The slice proper (the shapes of `design/SHAPES.md`, decode, the RPC arm, the Rust core) is **not started** |
| **Blocked on** | nothing. ABI v1 decision 1 did not touch this work unit and is now answered by the C++ slice anyway, so the next work unit can build against ABI v1 rather than around it |
| **Floor** (must build and pass correctness) | still open question 4. **Not demonstrated**: no python3.7 on this machine (apt lists 3.7.17-1+noble2). Facts for the decision are in `ffi/logs/python/01-environment.log` |
| **Target** (where the clock runs) | 3.11, as proposed. Every arm also builds and passes on 3.10, 3.12 and 3.13, and the verdict's shape is the same on all four |
| **Incumbent** (the baseline every ratio is against) | `protobuf` 7.36.2 on **upb**, confirmed at run time by `api_implementation.Type()`. `grpcio` 1.84.0 installed but unused: there is no RPC arm yet |
| **This machine's Rust crossing (R13)** | **2.1 ns** forward-plus-reverse, **2.8 ns** forward, against **1.8 ns** on the Rust slice's container and **2.1 to 2.2 / 1.5 ns** on the C++ slice's. Every absolute below is also a multiple of that |
| **How to read every absolute here** | as **instrumentation, not a deliverable** (README section 8, after R13, and `CLAUDE.md`'s invariant). The cross-language comparison is re-taken on a controlled physical machine once every slice exists. What this work unit produces that a rerun cannot are the **crossing counts**, the **byte identity**, the **within-arm deltas** that settle README 9.1's premise, and the **feasibility** facts. The nanoseconds rank the candidates and nothing more |

## Ambiguous rankings, left for the controlled run

Three comparisons here are **too close to call on this machine**, and the
instruction is to record that rather than grind at it. None of the three changes
a decision, which is why they are cheap to leave open.

| comparison | what was measured | why it is left |
|---|---|---|
| plain class against `__slots__`, both through `PyObject_GetAttr` | **the sign flips between payloads**: on P1.2 `__slots__` is marginally ahead (1.194 - 1.227 against 1.229 - 1.252 of upb), on P1.3 plain is (4.433 - 4.643 against 4.668 - 4.803), and at the primitive level plain is ahead (9.80 - 9.88 against 11.20 - 11.30 ns) | it does not matter: both lose to the C-extension type by a factor of two on P1.2 and by five on P1.3, and that gap is far outside any spread here. What is **not** ambiguous is the refutation of README 9.1's ordering, which only needs "`__slots__` is not clearly faster" |
| `METH_O` against `METH_FASTCALL` | 22.2 - 23.1 against 22.8 - 23.6 ns: overlapping | the mechanism choice is C extension either way, and which calling convention a generated binding emits is not a question this slice has to answer |
| abi3 against the full C-API, per primitive | 0.995 to 1.072 paired per process, and the forward call is the same row | the honest statement is **"no measurable difference on this workload"**, not a ratio. The one real difference is structural rather than timed: `PyList_SET_ITEM` does not exist in the limited API, so a shim uses `PyList_SetItem` at 1.97 to 2.01 times the cost of the macro. That one survives a controlled rerun because it is an API fact |

## The question this slice answers

Does the amended ABI beat an incumbent that is already native, and can it
survive the GIL?

## What work unit 1 establishes

Read `ffi/poc/python/JOURNAL.md` for the derivations; this is the short form.
Ranges are the spread of three separate processes on 3.11 unless stated.

### 1. The binding mechanism: the C extension, and it is not close

Forward, per call from Python, every arm calling the same callee in the same
`libakmech_cabi.so` (R7). The empty Python `for` loop that drives every row costs
10.4 - 11.7 ns and the `net` column has it subtracted.

The last column is R13's: the same figure as a multiple of **this machine's**
Rust forward crossing, 2.8 ns, which is the form that survives the move to
another container. It is taken on the net column, because the Rust figure has no
Python loop in it either.

| mechanism | ns | net of the loop | x a Rust forward crossing here |
|---|---|---|---|
| C extension, `METH_O` | 22.2 - 23.1 | 11.4 - 12.5 | 4.1 - 4.5 |
| C extension, `METH_FASTCALL` | 22.8 - 23.6 | | |
| C extension, abi3 | 22.2 - 23.3 | | |
| PyO3 | 52.4 - 53.3 | 41.5 - 42.3 | 14.8 - 15.1 |
| cffi, API mode | 76.7 - 77.1 | 65.4 - 66.3 | 23.4 - 23.7 |
| cffi, ABI mode | 195.2 - 200.6 | 184.8 - 190.0 | 66.0 - 67.9 |
| ctypes, `argtypes` declared | 235.0 - 241.1 | 224.6 - 230.5 | 80.2 - 82.3 |
| *(a pure-Python function call)* | 40.3 - 40.9 | 29.2 - 29.9 | 10.4 - 10.7 |

Reverse, per reverse call, driven from inside the C library so the forward cost
is amortised away. The floor is a C-to-C call through a function pointer at
1.41 - 1.42 ns.

| | ns | x the C-to-C floor |
|---|---|---|
| C-API call on a primitive (the design's default) | **2.83 - 20.54** | 2 - 15 |
| into the interpreter, C extension | 39.9 - 40.5 | 28 |
| into the interpreter, PyO3 | 51.0 - 51.2 | 36 |
| ctypes `CFUNCTYPE` callback | 119.4 - 119.8 | 84 |
| cffi callback, **API mode as well as ABI mode** | 233.8 - 239.9 | 165 - 170 |

These are quoted against the C-to-C floor rather than against the Rust crossing,
because a C-API call on a Python object is not a boundary crossing in the sense
R13's number measures: the shim and CPython are in one address space with no
dynamic-linker hop between them. The floor here (1.41 - 1.42 ns) and the Rust
slice's forward-plus-reverse figure on this machine (2.1 ns) are the two
comparable quantities, and they differ because the Rust loop makes a forward
call as well.

**`ctypes` and `cffi` are refused for the codec path and the log says why**, as
README 9.1 asks: their callback is 165 to 170 times the floor and 6 times the C
extension's own interpreter re-entry. Compiling the extension (cffi API mode)
fixes the *forward* direction and does nothing at all for the reverse one.
They remain candidates for the RPC layer, where the crossing count is two per
call.

**PyO3's cost is its per-call wrapper, not its primitives.** A `#[pyfunction]`
with no arguments at all already costs 47.9 - 49.9 ns; its primitives run 0.67
to 1.45 of the C-API spelling (faster on `extract::<i64>`, slower on
`PyString::new` and `PyList::new`). Since a shim makes one forward call and thousands
of primitive calls, this bears on the RPC layer more than on the codec.

**One `ctypes` row is wrong, not fast.** Without `argtypes` it is 143.0 - 145.4
ns and truncates the return to `int`. The correctness gate in `bench_mech.py`
catches it and the table carries the note.

### 2. The facade storage: the C extension type, and only if the shim reads the struct

| storage, as the shim reaches it | ns per field read | crossings per element |
|---|---|---|
| plain class, `PyObject_GetAttr` | 9.80 - 9.88 | 29 |
| `__slots__` class, `PyObject_GetAttr` | 11.20 - 11.30 | 29 |
| C extension type, through its member descriptor | 11.22 - 11.29 | 29 |
| C extension type, **struct member read** | 0.48 - 0.50 (0.12 - 0.14 net of the loop floor: at the resolution limit) | **7** |

Over a whole `ListResultsResponse`, against upb, P1.2 (1,000 elements):

| arm | / upb |
|---|---|
| `cshim member / C ext type` | **0.600 - 0.612** |
| `cshim getattr / __slots__` | 1.194 - 1.227 |
| `cshim getattr / plain` | 1.229 - 1.252 |
| `cshim pyacc (C attrgetter)` | 3.584 - 3.703 |
| `cshim pyacc (python fn)` | 3.984 - 4.085 |
| `pycodec` (pure Python, R3's no-boundary control) | 19.372 - 20.270 |
| *(floor: one copy of the 218 KB output)* | 0.018 |

**The absent path separates them much further.** On P1.3 the C-extension-type
arm is 0.934 - 0.953 and the getattr arm is 4.433 - 4.643, because the getattr
shim still pays all 29 crossings on an element that encodes to nothing. This is
the same shape as the Rust slice's finding that the absent path inverts a
verdict, and here it inverts one storage and not the other.

### 3. The premise (README 9.1, last bullet): settled, and the layer earns its place

The control is the same generated C traversal with **the same counted
crossings**, each field reached by a Python-level accessor call instead of a
C-API read. A within-arm delta (R4). It is **3.2 to 3.5 times worse** on the
full payloads and **5.4 to 5.6 times worse** on the absent path. A
C-*implemented* accessor called the same way recovers about a tenth of that, so
the cost is the call and not the language the accessor is written in.

### 4. Answers to open question 4 that are facts about the machine

`ffi/logs/python/01-environment.log`, and none of this decides anything.

- **Interpreters here**: 3.10.20, 3.11.15, 3.12.3, 3.13.12, with dev headers for
  all four. 3.11 is the default. Every arm builds and passes conformance on all
  four and the verdict's shape holds across them.
- **No free-threaded build exists here and none is installable from this
  container's apt**, so the 3.13t arm cannot be measured on this machine at all.
- **python3.7 is not present**; apt lists 3.7.17-1+noble2. **The incumbent does
  still exist for 3.7**: pip resolves `protobuf-4.24.4-cp37-abi3` and
  `grpcio-1.62.3-cp37`. So a floor arm is buildable, with an incumbent three
  years older than the target's, which under R7 makes a floor-against-target
  ratio a comparison of library versions as well as of runtimes.
- **abi3 (9.2) costs almost nothing for this workload.** One `.so` built against
  `Py_LIMITED_API=0x030A0000` on 3.10 is loaded and timed by all four
  interpreters. Forward call: the same row. Seven primitives, paired per
  process: **0.995 to 1.072**. The one real loss is `PyList_SET_ITEM`, a macro
  absent from the limited API, so a shim uses `PyList_SetItem` at **1.97 to
  2.01 times** the cost on that one call. Everything else this slice needs is in
  the limited API from 3.10.
- **The incumbent has already made this trade**: protobuf ships `cp3x-abi3`
  wheels and its upb extension on disk is `_message.abi3.so`.

### 5. Two numbers for the design documents

- **Releasing the GIL costs 34.2 - 34.7 ns** on 3.11 (34.3 - 43.4 across 3.10 to
  3.13), which is about twelve C-API primitives. It bounds when 9.1's "a longer
  window in which the pure parse runs with the GIL released" is worth doing.
- **On CPython the UTF-8 passthrough is free only for ASCII.** Reading a `str`'s
  UTF-8 is 2.19 - 2.27 ns for ASCII and **64.2 - 67.8 ns** for Latin-1 and
  above-U+00FF content when the object has no UTF-8 cache yet, which is the state
  a string that came off the wire is in. ArmoniK's ids are all ASCII GUIDs, so
  the common path is the cheap one.

## What exists

```
mech/
  build.sh              every arm, every interpreter. Includes the R5 check that
                        the boundary into libakmech_cabi.so is a real import
  run.sh                work unit 1 end to end; writes ffi/logs/python/*
  environment.sh        what this machine offers (open question 4)
  harness.py            interleaved rounds, median with min and max, per-case
                        calibration, gc off (R4)
  summarise.py          collapses a multi-process log into the ranges a finding
                        may quote. Every table above regenerates from it
  conformance.py        R2: byte identity across every arm against the validated
                        manifest, plus R5 crossing counts from a counting build
  bench_mech.py         the mechanism, primitive and storage microbenchmarks
  bench_codec.py        the codec arms over M1, against upb
  arms.py               the arm list, shared by conformance and bench
  cffi_build.py         the cffi API-mode arm
  gen/generate.py       THE generator (R1). Reads ffi/schema/shapes.json through
                        ffi/schema/emit/shapes.py. One field walker; a shape it
                        has no case for RAISES
  gen/out/              emitted and COMMITTED: facade.py, pycodec.py,
                        payload_values.py, _akcodec_gen.c
  native/cabi.c         the plain C library: the callee every mechanism reaches
  native/_akmech.c      the mechanism and primitive probe. Built twice, full API
                        and abi3, from one source
  native/_akcodec.c     the module wrapper around gen/out/_akcodec_gen.c
  pyo3/                 the PyO3 arm, full and abi3
```

**Reproduce everything**: `mech/run.sh <target-python> [<other pythons>...]`.
It builds, gates on conformance, and writes every log. The R13 calibration is
separate and is `AK_BENCH_ONLY=P1.1 ffi/poc/rust/target/release/bench`.

**Verified from a clean clone of the pushed branch, at a different path**, which
is how D4 was found: `git clone --branch claude/ffi-slice-python . /tmp/v && cd
/tmp/v/ffi/poc/python/mech && ./build.sh <python> && <python> conformance.py`
builds every arm and passes every check. Do this at the end of a work unit. It is
README R4's last paragraph as a command: a figure whose harness is not in the
tree, or is in the tree and does not build, cannot be defended.

## Correctness

**Established, for what is built.** Every codec arm is byte-identical to
`ffi/schema/generated/manifest.json` on **P1.1, P1.2 and P1.3**, on all four
interpreters, including the incumbent and the pure-Python control, and the
counting build is checked to produce the same bytes as the measured build. The
absent path (P1.3) is in the gate, not beside it. `bench_codec.py` runs
`conformance.py` as a subprocess and refuses to time anything if it fails.

The unknown-field vectors are **not** covered: they are a decode obligation and
there is no decode arm. Neither is P2.5, which is an M2 payload.

`gen/generate.py --check` runs as a build step, so a measured artifact cannot
have been compiled from a generated tree that no longer matches `shapes.json`.

## Open defects

| # | where | what |
|---|---|---|
| D1 | `native/_akmech.c` | the forward arm used `PyLong_AsLongLong` and raised above 2^63 while the PyO3 and cffi arms answered, so the arms were not doing the same work. **Fixed** (`PyLong_AsUnsignedLongLong`). Found by the correctness gate, not by reading the code |
| D2 | `bench_codec.py` | the floor arm was `bytes(ref)` on a `bytes`, which returns the same object and copies nothing: 58 ns at 858 B and at 218 KB alike. **Fixed** (`memoryview.tobytes()`). A floor that is not doing the work is worse than no floor |
| D3 | `ffi/poc/python/.gitignore` | the repository-wide `.gitignore` excludes any directory named `gen`, and `ffi/.gitignore` re-includes `poc/*/gen/**`, which does not reach `poc/python/mech/gen/`. **Fixed** by a re-inclusion in this slice's own `.gitignore`, checked with `git check-ignore`. This is the fourth time that file has eaten a source directory in this branch |
| D4 | `gen/generate.py` | `payload_values.py` carried the ffi root as an **absolute path baked in at generate time**, so a clone of this branch at any other path regenerated a different file and `generate.py --check` refused the whole tree as stale: the committed sources could not rebuild themselves. **Fixed**: the generated module walks up to `schema/shapes.json` instead. Found by cloning the pushed branch and building it, not by reading the code, and it is the reason that clone-and-build is now the last step of a work unit. No measurement is affected: `_akcodec_gen.c` is byte-identical across the fix, and the only file that changed computes paths |

None open. Every one of the four was found by something running, not by review:
D1 by the correctness gate, D2 by a ratio that did not move with payload size,
D3 by reading what `git status` did NOT list, D4 by building a clean clone.

## What is not measured

Long, and deliberately so.

- **Decode, in every arm.** The whole of work unit 1 is encode. The storage
  verdict may not carry: decode constructs objects rather than reading them, and
  constructing one facade element (`PyObject_CallNoArgs` on the type) is 40.7 -
  41.7 ns, as dear as a full interpreter re-entry and more than 300 times a
  struct member READ. What a struct member *write* costs was not measured, which
  is itself part of why decode is the next step and not a footnote.
- **The Rust core.** No core in any arm. The codec arms price the
  **shim-to-facade** edge, which is the one that is a crossing per field; the
  shim-to-core edge is priced separately (a forward call at 22.2 - 23.1 ns and a
  C-to-C reverse at 1.42 ns) but the two have never been composed, so no arm
  here is "the design end to end".
- **M2 through M7**, and every payload but P1.1, P1.2 and P1.3. The generator
  raises on a message outside the M1 subtree rather than skipping it (R1), so the
  scope is enforced by the build. That means **no oneof, no explicit presence, no
  map, no packed field, no repeated string, no adapter site, no 4-level nesting,
  no bulk bytes**, and none of the shape-coverage rows of `design/SHAPES.md`.
- **The RPC arm.** Nothing. `grpcio` is installed and unused.
- **The unknown-field vectors and the conformance corpus.**
- **Content sets on a whole message.** Priced at the primitive level only; no
  payload in this work unit carries non-ASCII content.
- **Concurrency, and therefore the GIL question itself.** The cost of releasing
  the GIL is measured; nothing runs two threads. README 9's question "can it
  survive the GIL" is **not** answered by work unit 1.
- **The floor (3.7) and the free-threaded arm.** Neither is on this machine.
  Free-threaded is not installable here at all.
- **Allocation per operation**, in any arm. Only time.
- **A PyO3 spelling cheaper than the default `#[pyfunction]`**, if one exists.
- **What abi3 costs on decode**, where `PyList_SET_ITEM` is used per element
  rather than per field and the gap would be widest.

## Requests to the aggregating session

Written here because a slice agent does not edit `design/**` or `README.md`.

1. **README 9.1's storage parenthetical is wrong in the middle term.** It says
   "a plain class (a dict lookup), a `__slots__` class (a descriptor offset), or
   a C extension type (a struct member ...). Each is more work than the last and
   each is faster." Measured through `PyObject_GetAttr`, which is what a C shim
   calls, `__slots__` is **slower** than a plain class (11.20 - 11.30 against
   9.80 - 9.88) and so is a C extension type reached through its member
   descriptor. Only the struct member read moves, so the sentence's conclusion is
   right and its ordering is not. Suggested amendment: the three storages differ
   only in whether the shim can **stop making a crossing**, not in how fast the
   crossing is.
2. **A finding for 9.1 that the design did not anticipate**: a specialised
   bytecode `LOAD_ATTR` costs 3.37 - 3.91 ns and `PyObject_GetAttr` from C with
   an interned key costs 9.44 - 9.52. CPython's interpreter has an inline cache
   and the C API has no equivalent entry point, so **a C shim reading a plain
   facade is doing the same work about 2.5 times more slowly than the interpreter
   would**. It is the strongest argument in the slice for the C-extension facade
   type and it belongs in 9.1 beside the crossing-count argument.
3. **ABI v1 section 4, the transcoder table.** `ak_tc_latin1` ("CPython 1-byte")
   and `ak_tc_ucs4` ("CPython 4-byte") would, on CPython, be competing with
   CPython's own UTF-8 cache rather than with nothing: an uncached read costs
   64.2 - 67.8 ns and the object keeps the result. Worth a line in decision 3's
   "what survives", since it changes what the converting transcoders are for on
   this host.
4. **A number for section 2's per-runtime crossing table, with R13 applied.**
   A Python host's forward crossing through a C extension costs **11.4 - 12.5 ns
   net of the Python loop that calls it** (22.2 - 23.1 ns gross), against
   **2.8 ns** for the Rust slice's own forward crossing measured on this same
   machine: **4.1 to 4.5 times a Rust crossing**, which is the form that survives
   the move to another container. For comparison with the managed rows of that
   table, a *reverse* call into the interpreter is 39.9 - 40.5 ns, between .NET 8
   (7.5 to 12) and JNI (98.4) -- but the whole point of README 9.1 is that the
   Python design does not make that call, and the number that belongs beside the
   others is the C-API primitive at 2.83 - 20.54 ns.
5. **Nothing in `design/SHAPES.md` needs changing.** The M1 subtree, the value
   rules and the P1.x manifest hashes all reproduced exactly, in four
   independent encoders, on the first attempt.

## Next step

In order, and the first one is the one that could still change the verdict.

1. **Decode, over the same M1 subtree, same three storages, same premise
   control.** It is the direction where the storage answer is least safe (object
   construction at ~41 ns per element dominates a field write at ~0.1 ns) and
   where ABI v1's two delivery families (7.1) first become a real choice for
   this host. Until it exists, "the C extension type wins" is an encode
   statement.
2. **Widen `SCOPE` in `gen/generate.py` to the rest of `design/SHAPES.md`**, one
   shape at a time, letting the walker's `Unsupported` raise drive the order.
   M2 and P2.2 first, since P2.2 is the shape the control plane actually moves
   and the Rust slice's decode convergence finding says a thin payload's verdict
   may not survive it.
3. **Compose the two halves**: put the generated Rust core behind the generated C
   shim, so there is an arm that is the design rather than one of its edges.
   **ABI v1 decision 1 is now answered by the C++ slice**, so this builds against
   ABI v1 rather than around it; the shim should still be written so that an ABI
   change lands only in the binding backend. What this arm owes is a **crossing
   count** and **byte identity**, both of which survive the controlled rerun; its
   nanoseconds do not need to be tight.
4. **The RPC arm**, where `ctypes` and `cffi` are back in the running and the
   crossing count is two per call.
5. **Concurrency**, which is where README section 9's actual question lives and
   where nothing has been measured. `Py_BEGIN_ALLOW_THREADS` at ~35 ns is the
   only input to it so far.

## What the scope change means for this slice

Relayed on 2026-09-19 and merged as `4aec4e8`: nobody tries hard at
cross-language performance until the controlled physical run. Nothing in work
unit 1 is withdrawn by it and nothing needs re-taking, because what this work
unit was asked for is exactly what the note says survives: a mechanism choice
(sign and rough magnitude), a storage choice (2x and 5x gaps, not percentages),
the premise control (a 3.2x to 5.6x within-arm delta), counted crossings, and
feasibility. The three comparisons that *were* close are recorded above as
ambiguous rather than resolved.

What it changes going forward: the next work units buy correctness, crossing
counts and feasibility first, and no work unit is spent tightening a spread that
already ranks its candidates.

One cross-check the C++ slice's arrival makes possible. Its container measures
the Rust crossing at **1.5 ns forward and 2.1 to 2.2 forward-plus-reverse**; this
one measures **2.8 forward and 2.1 forward-plus-reverse**. The forward-plus-
reverse row agrees between the two containers to 0.1 ns and the forward row
differs by 1.9 times, which says the **forward loop** is the one that does not
travel, not crossings in general. It is a third data point on the oddity recorded
in `JOURNAL.md` J1 and a reason to prefer the forward-plus-reverse figure when
one number is wanted.

## Log index

**The R13 calibration is the first row on purpose**: every absolute in this
directory is a fact about this container and is quotable against it (R13).

| Log | Configuration | What it establishes |
|---|---|---|
| `ffi/logs/python/00-r13-rust-crossing.log` | `ffi/poc/rust` unmodified, rustc 1.94.1 release, `libak_core.so` through the dynamic linker, 3 processes, 4 shared vCPU | **R13: this machine's Rust crossing is 2.1 ns (fwd+reverse) to 2.8 ns (forward)**, against 1.8 ns on the Rust slice's container. Also the R5 proof that the boundary is a real dynamic import |
| `ffi/logs/python/01-environment.log` | this container | What python 3.x this machine can offer, that no free-threaded build exists or is installable, that 3.7 is absent but apt-reachable, that the incumbent is upb, and which wheels pip resolves for 3.7. Facts for README open question 4, no decision |
| `ffi/logs/python/10-build.log` | gcc 13.3, `-O2 -Werror`, PyO3 0.29.2 release | Every arm built for 3.10 - 3.13; the R5 import check on each; the one abi3 artifact built on 3.10 loaded and run by all four |
| `ffi/logs/python/20-conformance.log` | 3.10, 3.11, 3.12, 3.13 | **R2**: every codec arm byte-identical to the validated manifest on P1.1, P1.2 and P1.3 (the absent path included), on every interpreter. **R5**: crossing counts from the counting build -- 29.00 per element through `getattr`, **7.00** through a struct member read, 29.00 through the Python-accessor control |
| `ffi/logs/python/30-mechanism-py3.11.log` | python 3.11.15, 3 separate processes, 11 interleaved rounds, gc off | The mechanism table, the reverse-call table, the primitive table, the abi3 comparison, the PyO3-against-C-API comparison, the three content sets, and the storage table. Everything in sections 1, 2, 4 and 5 above |
| `ffi/logs/python/31-mechanism-all.log` | 3.10, 3.11, 3.12, 3.13, one process each | The same, across every interpreter here. Absolutes across the blocks are four processes and do not compare; what it establishes is that the **shape** of the answer holds from 3.10 to 3.13 |
| `ffi/logs/python/40-codec-py3.11.log` | python 3.11.15, protobuf 7.36.2 on upb, 3 separate processes | The codec table of section 2 and the premise control of section 3, with the output-copy floor arm R2 requires |
| `ffi/logs/python/41-codec-all.log` | 3.10 - 3.13, one process each | The same across interpreters: `cshim member` 0.597 - 0.637 of upb, `cshim getattr` 1.198 - 1.310, the premise control 3.880 - 5.021, pure Python 19.3 - 28.3 |

## Slice-specific notes

- A reverse call into Python must hold the GIL, so the drafted ABI's per-field
  upcall is the worst possible shape here. **Measured**: 39.9 - 40.5 ns against
  2.83 - 20.54 for a C-API primitive.
- `packages/python` reads no transport environment configuration today, so
  configuration homogeneity is a pure gain rather than a migration. Unchanged;
  nothing in work unit 1 touched it.
- A pure-Python control loses to the native incumbent by an order of magnitude.
  **Measured at 19.4 - 20.3 times on P1.2**, and R9 says in advance that this is
  a result: the codec question in Python is native against native.
- Floor and target may be different code, gated at import rather than compiled
  out. Untested: there is no floor build.
