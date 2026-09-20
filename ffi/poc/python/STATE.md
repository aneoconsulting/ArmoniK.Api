# python slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | **work units 1 and 2 complete.** Work unit 1 chose the binding mechanism and the facade storage by microbenchmark and settled README 9.1's premise. Work unit 2 built **the composed arm** -- the shared core at `poc/codec/` behind a generated CPython shim, both directions -- added **decode**, and became the **conformance corpus's first consumer**. M2 through M7, the RPC arm and concurrency are **not started** |
| **Blocked on** | nothing |
| **Floor** | still README open question 4. Not demonstrated: no python3.7 here (apt lists 3.7.17-1+noble2). Facts in `logs/python/01-environment.log` |
| **Target** | 3.11. Every arm builds and passes on 3.10, 3.12 and 3.13, and the verdict's shape holds on all four |
| **Incumbent** (R14) | `protobuf` 7.36.2 on **upb**, through **the path ArmoniK runs**: `Message.SerializeToString` and `Message.FromString`, with no size pass and no buffer writer. **Derived, not assumed** -- `verify_r14.py` generates the real `Protos/V1/results_service.proto` stub and reads the serializer off it (`logs/python/52-r14-baseline.log`) |
| **Core** | `ffi/poc/codec` (R0), reached by path, never copied. `poc/codec/gen/one_core.sh` passes |
| **R13** | this machine's rust-slice crossing is **2.1 ns** (fwd+reverse) to **2.8 ns** (forward), against 1.8 on the rust slice's container and 2.1-2.2 / 1.5 on the cpp slice's. The composed arm also prices the boundary **in its own process and build** at the foot of every bench table (**1.8 ns** forward, **2.4 ns** fwd+reverse), which is the better number to quote against because it shares a build with the arms |
| **How to read every absolute** | **instrumentation, not the deliverable**. Signs and size classes only; the controlled physical rerun owns the decimals |

## The answer, in four lines

1. **Encode: the composed arm beats the incumbent.** 0.700 to 0.719 of upb on
   P1.2 with a C-extension facade (0.679 to 0.719 across four interpreters).
2. **Decode: it beats the incumbent too, once the comparison is like for like.**
   1.77 to 1.80 of upb on the bare call, **0.888 to 0.897** once both sides have
   actually produced Python values -- and `FromString` does not produce them.
3. **The facade storage decides everything, and only one of the three works.**
   A C extension type the shim reads as a struct: 7 crossings per element. The
   two Python storages: 22 to 29, and 1.5 to 4.0 of upb.
4. **The ABI is not where Python's crossing problem is.** The core's own boundary
   costs **0.01 crossings per element**; the shim-to-facade edge costs 7 to 29.

## What work unit 2 establishes

Ranges are the spread of **three processes on 3.11**, the target interpreter,
unless a row says otherwise; the four-interpreter range is given beside the
headline rows, and it is wider mostly because 3.10 is a slower interpreter rather
than because the arms move against each other.
`logs/python/60-composed-py3.11.log` and `61-composed-all.log`;
`mech/summarise.py <log>` regenerates every table here.

### 1. Encode, P1.2 (1,000 elements), against upb on the R14 path

| arm | / upb, 3.11 | / upb, 3.10 - 3.13 |
|---|---|---|
| **`core-ffi / C ext type`** | **0.700 - 0.719** | 0.679 - 0.719 |
| `core-ffi / __slots__` | 1.530 - 1.548 | 1.508 - 1.802 |
| `core-ffi / plain` | 1.623 - 1.659 | 1.623 - 1.765 |
| `core-ffi / pyacc` (the premise control) | 4.885 - 5.790 | 4.885 - 6.855 |
| `pycodec` (R3's no-boundary control) | 25.1 - 26.5 | 25.1 - 37.8 |
| *(floor: one copy of the 218 KB output)* | 0.017 | 0.017 - 0.018 |

Work unit 1 measured the same storage at 0.600 - 0.612 **with no core behind
it**. Adding the real core, the real ABI and the sparse fill moves it to 0.700 -
0.719, so **the core and the boundary together cost about 0.10 of upb's encode**
on this shape -- which is the decomposition README 4.1 asks every slice for, and
it is small.

On **P1.3, the absent path**, the C-extension arm is 1.271 - 1.393 and the two
Python storages are 6.6 to 10.1. Decision 9's sparse fill is what keeps the first
of those near 1 rather than near 2: it is built in from the start here, and the
C# slice's composed arm not doing it cost its absent path a factor of two.

### 2. Decode, P1.2: two tables, because the bare call does not compare the same work

| arm | decode (the call) | decode **+ read every field** |
|---|---|---|
| **`core-ffi / C ext type`** | 1.773 - 1.804 | **0.888 - 0.897** |
| `core-ffi / __slots__` | 3.692 - 3.740 | 1.013 - 1.029 |
| `core-ffi / plain` | 3.908 - 3.974 | 1.036 - 1.065 |
| `core-ffi / pyacc` | 11.28 - 11.61 | 1.656 - 1.726 |
| `pycodec` | 37.0 - 37.1 | 3.86 - 3.93 |
| *(floor: 1,000 bare facade objects + one copy)* | **0.803 - 0.807** | - |

Across four interpreters: the C-extension arm is 1.763 - 1.990 on the call and
**0.865 - 0.975** like for like.

**The floor row is why there are two tables.** Constructing the host objects and
copying the input, with no parsing at all, already costs four fifths of upb's
entire decode. A decode cannot cost less than producing what it produces, so upb
is not producing it: `FromString` parses into a upb arena and materialises a
Python object only when something reads it. The right-hand column applies the
same read to every arm and puts them on the same work.

Neither column is "the" answer and the report has to pick one deliberately: a
caller that reads its fields pays the right-hand column, a caller that decodes
and discards pays the left.

**P1.3 makes the point unmissable.** On the absent path the floor -- 300 bare
objects and a copy of 605 bytes -- is **8.7 to 24.9 times** upb's entire decode,
because upb has almost nothing to do and the facade must still construct 300
objects. The bare call reads 6.2 to 6.9 for the C-extension arm and is a
statement about object construction and nothing else; like for like it is
**0.902 to 1.072**, parity.

### 3. upb does not cache, and it changes which column is generous

A second full read of the **same** upb message costs **3.12 - 3.16 ms** against
3.41 - 3.48 for the first: all but about 10 percent of the materialisation is
paid again. The facade's second read is **2.38 - 2.54 ms**.

So the like-for-like column above charges upb the materialisation once and
production would charge it per pass. **A caller that reads its response twice
pays upb twice and the facade once.**

**This refutes the hypothesis decision 13 was carrying.** upb aliases strings
into its input buffer in C (`upb/wire/decode.h:29`), but a Python `str` is a
fresh object built on every attribute read, so there is nothing borrowed at the
Python level. Decision 13's borrowed span is **available to this facade and not
already taken by the incumbent** -- the opposite of what the open question
assumed. What a borrowed Python string would be is a facade question nobody has
drafted, and this slice did not build it.

### 4. Crossings, counted in both halves (`51-conformance-wu2.log`)

Per element, P1.2. The core counts its own and the shim counts what it does to
CPython; neither half can count the other's.

| | shim -> CPython | core, forward | core, reverse |
|---|---|---|---|
| encode, C extension facade | **7.00** | 0.01 | 0.00 |
| encode, `PyObject_GetAttr` | 29.00 | 0.01 | 0.00 |
| decode, C extension facade | **7.00** | 0.00 | 0.01 |
| decode, `PyObject_GetAttr` | 22.00 | 0.00 | 0.01 |

**The batched element run turns 1,000 elements into about ten core entries.** So
the ABI's crossings are already negligible in Python and every crossing that
matters is between the shim and the facade. That is a different sentence from
every other slice's and it is what README 9.1's three layers predict.

### 5. The corpus (W8): first consumer, and three defects

48 of 336 rows root at `ListResultsResponse`, which is what this scope reaches;
every other row is reported out of scope **by root**, never silently dropped.
CONTRACT.md rule 0 is checked rather than asserted: the three messages are
identical to `corpus.proto` and the superset adds seven `u_*` fields this reader
does not know. `logs/python/70-corpus-subset.log`.

| arm | C1 parse | C2 project | C3 re-encode | C4 refuse |
|---|---|---|---|---|
| `pycodec / plain` | 47/47 | 47/47 | 47/47 | 1/1 |
| `core-ffi / C ext type` | 45/47 | 45/47 | 45/47 | 1/1 |
| `core-ffi / plain` | 45/47 | 45/47 | 45/47 | 1/1 |

Two defects were this slice's and are fixed (D5, D6 below). **The two the
core-ffi arms still fail are a defect in the shared core** and are the most
important thing in this section: see the request to the aggregating session.

**ABI v1 decision 11, answered for python: this slice DROPS unknown fields**, 29
of 34 unknown-class rows re-encoding to the `unknown-dropped` form. The core
carries `ak_unk_f` vtable slots and this shim passes NULL for every one, so the
drop is the binding's choice and not a limit of the ABI.

## What exists

```
gen/generate.py        THE generator (R1, R0). Imports poc/codec/gen/ir.py and the
                       cpp slice's cpp_header.py read-only; writes nothing outside
                       this slice
gen/walk.py            the one field walker. A shape outside SCOPE raises
gen/py_facade.py       the facade, plain and __slots__
gen/py_codec.py        the pure-Python codec, encode and decode (R3's control)
gen/py_binding.py      the COMPOSED arm's shim: 3 backends x 2 directions
gen/py_shim.py         work unit 1's no-core C encoder, kept so its column stands
gen/py_values.py       facade objects carrying the manifest's values
gen/out/               emitted and COMMITTED, ak_abi.h included
native/binding.c       the module wrapper. Names no message and no field
build.sh               R0 check, R1 check, the core, the shim, the R5 boundary proof
run.sh                 work unit 2 end to end; writes logs/python/5x, 6x
conformance.py         R2 and R5: byte identity both directions, layout facts,
                       crossing counts (the counting pass in its own process)
corpus.py              W8: the corpus, scoped to what M1 can root
verify_r14.py          derives the baseline from Protos/V1
bench.py / arms.py     the arms and the timing
mech/                  work unit 1, frozen: the mechanism, primitive and storage
                       microbenchmarks, and the harness both work units share
```

**Reproduce**: `./build.sh <python>...` then `./run.sh <target-python> <others>...`.
**Verify from a clean clone at another path** before calling a work unit done --
that is how D4 was found and it is a step, not an idea.

## Correctness

Established for what is built, and it gates everything: `bench.py` runs
`conformance.py` as a subprocess and refuses to time anything if it fails.

- Encode and decode byte-identical to `ffi/schema/generated/manifest.json` on
  **P1.1, P1.2 and P1.3**, every arm, on 3.10 / 3.11 / 3.12 / 3.13. The absent
  path is in the gate, not beside it.
- Decode checked **twice**: by re-encoding to the same bytes, and field by field
  against the incumbent, so a value both the decode and the re-encode lost
  cannot hide.
- ABI version and **380 group layout facts** checked at import (section 10,
  obligation 12.3).
- The boundary proved from the artifact in both builds (R5).
- 48 corpus vectors, with the projections, the accepted-encoding set and the
  reject vector (W8).

## Open defects

| # | where | what |
|---|---|---|
| D1 | `mech/native/_akmech.c` | the forward arm raised above 2^63 while other arms answered. **Fixed.** Found by the correctness gate |
| D2 | `mech/bench_codec.py` | the floor arm was `bytes()` on a `bytes`, which copies nothing. **Fixed.** Found by a ratio that did not move with payload size |
| D3 | `.gitignore` | the repo-wide `gen` rule reached this slice's generator. **Fixed**, and `ffi/.gitignore` has since been widened to `poc/**/gen/**` |
| D4 | `gen/py_values.py` | the generated tree embedded an absolute path, so a clone at another path could not rebuild itself. **Fixed.** Found by cloning the pushed branch and building it |
| D5 | `gen/py_codec.py` | the pure-Python decoder bounds-checked a nested length against the whole buffer instead of the enclosing message, and accepted `X-nested-len-overrun`. **Fixed.** Found by the corpus |
| D6 | `gen/py_codec.py` | the pure-Python decoder raised on an unknown GROUP field instead of skipping it. **Fixed.** Found by the corpus. The two fixes cost the pure-Python decode about 12 percent (33.0 -> 37.0 of upb), and the tables above are the post-fix run, so every figure here is the committed tree's |
| **D7** | **`poc/codec` -- NOT this slice's to fix** | **`ak_decode_*` returns `AK_ERR_MALFORMED` on an unknown field of the deprecated GROUP form.** `U-root-group` and `U-nested-group` are `expect: accept`, upb accepts both (confirmed locally, not taken on trust), and every conformant parser must skip an unknown group: it carries no length, so a skipper has to recurse to its `END_GROUP`. **Open**, and it affects every slice |

D1 to D6 closed. **D7 is open and belongs to the aggregating session** (R0: a
change to existing behaviour in the core moves every slice's gate at once).

Every one of the seven was found by something running, never by review: the
correctness gate, a floor arm, what `git status` did not list, a clean-clone
build, and the corpus twice.

## Requests to the aggregating session

1. **D7 is the one that matters.** The shared core refuses a legal message. Two
   corpus vectors, `expect: accept`, upb accepts both. It is a decode-path
   defect in `poc/codec/crates/ak-rt`'s unknown-field skip, it is the same for
   every slice, and it was invisible until the corpus had a consumer -- which is
   exactly what `corpus/CONTRACT.md` says the first consumer is for.
2. **README 9.1's storage parenthetical is still wrong in its middle term**
   (work unit 1's request 1, unchanged and now confirmed on decode as well):
   `__slots__` is not faster than a plain class from a C shim. Only the struct
   member read moves, and on decode it is worth **2.0 to 4.4 times**.
3. **Decision 13 needs its Python premise corrected.** The branch's note
   supposes upb may already borrow at the Python level. It does not: it copies on
   **every** attribute read and caches nothing (measured, section 3). So the
   borrowed-span option is open here and the incumbent has not taken it -- but
   what a borrowed Python string *is* has no draft, and that is the blocker
   rather than the measurement.
4. **A Python row for section 2's crossing table, R13 applied.** A forward
   crossing from a Python host through a C extension is 11.4 - 12.5 ns net of
   the Python loop, **4.1 to 4.5 times a Rust forward crossing measured on the
   same machine**. But the number that matters for Python is not that one: it is
   **0.01 core crossings per element**, because the batching makes the ABI
   boundary disappear and the CPython boundary is the whole cost.
5. **Nothing in `design/SHAPES.md` needs changing.** Every M1 hash reproduced
   first time, in five independent encoders now.

## Ambiguous rankings, left for the controlled run

| comparison | why it is left |
|---|---|
| plain against `__slots__` through `PyObject_GetAttr` | the sign flips between payloads and between work units; both lose to the C extension type by 2 to 4 times either way, which is far outside any spread here |
| `METH_O` against `METH_FASTCALL` | overlapping; the mechanism is a C extension either way |
| abi3 against the full C-API per primitive | 0.995 to 1.072 paired is "no measurable difference". The one real difference is structural: `PyList_SET_ITEM` is absent from the limited API |
| `pyacc` with a Python function against `operator.attrgetter` | 3.58-3.70 against 3.98-4.09 on encode and inverted on decode. The finding is that **the call** costs, not what the accessor is written in, and it does not need the two separated |

## What is not measured

- **M2 through M7**, and every payload but P1.1, P1.2, P1.3. The generator
  **raises** on a message outside `walk.SCOPE`, so the scope is enforced by the
  build. That means no oneof, no explicit presence, no map, no packed field, no
  repeated string, no adapter site, no 4-level nesting, no bulk bytes. **P2.2,
  the shape the control plane actually moves, is the largest single gap**: the
  rust slice's convergence finding says a thin payload's verdict may not survive
  an element that gains containers.
- **The corpus beyond `ListResultsResponse`**: 288 of 336 rows, including every
  `Surrogate` vector (the transcode pair) and every `WireZoo` one.
- **The RPC arm.** Nothing. `grpcio` is installed and unused.
- **Concurrency**, so README section 9's actual question -- can it survive the
  GIL -- is still unanswered. `Py_BEGIN_ALLOW_THREADS` at 34.2-34.7 ns is the
  only input to it.
- **Allocation per operation**, in any arm. Only time.
- **The floor (3.7) and free-threaded CPython.** Neither is on this machine and
  free-threaded is not installable here at all.
- **Decision 13's borrowed-span arm.** Priced as an opportunity, not built.
- **abi3 on the composed arm.** Work unit 1 priced abi3 on the primitives; the
  shim is built full-API only.
- **The unknown-field bag.** The core has `ak_ufix_*` groups and `ak_unk_f`
  slots and this shim passes NULL, so retention is unpriced here.

## Next step

1. **M2 and P2.2.** Widen `walk.SCOPE` one shape at a time and let the walker's
   raise drive the order. It is the largest gap and the one most likely to move
   the verdict.
2. **Concurrency**, which is where README section 9's question lives.
3. **The RPC arm**, where `ctypes` and `cffi` are back in the running.
4. **Decision 13's borrowed-span facade**, if the aggregating session wants the
   option priced rather than only noted.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| `00-r13-rust-crossing.log` | `ffi/poc/rust` unmodified, 3 processes | **R13**: this machine's rust crossing, 2.1 to 2.8 ns |
| `01-environment.log` | this container | Which interpreters exist, that no free-threaded build does or can, that 3.7 is absent but apt-reachable and its incumbent still exists (protobuf 4.24.4 / grpcio 1.62.3). README open question 4's facts |
| `10-build.log`, `20-conformance.log` | work unit 1 | The mechanism arms built and gated on 3.10 - 3.13 |
| `30-mechanism-py3.11.log` | 3 processes, 11 interleaved rounds | Work unit 1: the mechanism, primitive, abi3, content-set and storage tables |
| `31-mechanism-all.log` | 3.10 - 3.13, 1 process each | The same, and that the shape holds across interpreters |
| `40-codec-py3.11.log`, `41-codec-all.log` | work unit 1, no core | The shim-to-facade edge priced on its own, and the premise control |
| `50-build-wu2.log` | gcc 13.3 `-O2 -Werror`, cargo 1.94.1 | The composed arm built for 3.10 - 3.13; R0 check; **R5's boundary proof from the artifact** |
| `51-conformance-wu2.log` | 3.10 - 3.13 | **R2** both directions against the validated manifest, the absent path included; the layout facts; **R5's crossing counts in both halves** |
| `52-r14-baseline.log` | `Protos/V1/results_service.proto` | **R14 derived**: the production path is `SerializeToString` / `FromString` |
| `60-composed-py3.11.log` | python 3.11.15, protobuf 7.36.2 on upb, 3 processes | **The composed arm**: encode, decode, decode+read, re-read, and the boundary priced in-process |
| `61-composed-all.log` | 3.10 - 3.13, 1 process each | The same across interpreters: the verdict's shape holds |
| `70-corpus-subset.log` | 3.11, 48 of 336 vectors | **W8's first consumer**: C1 to C4, the accepted forms written, decision 11 answered, and **D7** |

## Slice-specific notes

- A reverse call into Python must hold the GIL, so the drafted ABI's per-field
  upcall is the worst possible shape here. Measured: 39.9 - 40.5 ns against
  2.83 - 20.54 for a C-API primitive, and the composed arm makes **zero** of them
  per field -- the core's reverse count is 0.01 per element.
- `packages/python` reads no transport environment configuration today, so
  configuration homogeneity is a pure gain rather than a migration. Untouched.
- A pure-Python control loses to the native incumbent by an order of magnitude
  (25 to 37 times). R9 says in advance that this is a result: the codec question
  in Python is native against native, and README outcome 2 ("generate the codec
  into the host") is off the table for Python in a way it is not for Java.
- Floor and target may be different code, gated at import. Untested: there is no
  floor build.
