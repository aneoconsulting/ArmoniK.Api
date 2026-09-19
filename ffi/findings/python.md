# Reading the python slice

The aggregating session's reading of `poc/python`, which is not the same document
as the slice's own `STATE.md`. What is here is what its results mean for the
branch.

**Work unit 1 is complete and the slice proper is not started.** What exists is
encode, over the M1 subtree, on P1.1, P1.2 and P1.3, byte-identical to the
validated manifest in four independent encoders on four interpreters. What does
not exist is decode in any arm, any message beyond M1, the RPC arm, concurrency,
and — importantly — **any arm that composes the shim with the Rust core**. The two
edges are priced separately and have never been put together.

**Configuration** (R7): CPython 3.11.15 target, also built and passing on 3.10,
3.12 and 3.13; `protobuf` 7.36.2 on the **upb** C extension, confirmed at run time
rather than assumed; `grpcio` 1.84.0 installed and unused. R13: this container's
rust-slice crossing is **2.1 ns** forward-plus-reverse and 2.8 ns forward, against
1.8 ns in the rust slice's container and 1.5 / 2.1-2.2 in the C++ slice's. **Three
containers, three numbers**, which is the third independent confirmation that
section 2's table was a table about machines.

## 1. The result that changes the branch: outcome 2 is not available in Python

README section 13 offers three outcomes, and **outcome 2 — adopt the RPC layer and
generate the codec into each host language — is Java's own fallback recommendation
and the one the branch has been treating as the safe retreat.**

The pure-Python codec, generated from the same description by the same generator,
measures **19.4 to 20.3 times upb** on P1.2. That is not a margin anyone argues
with, and it is the same arm that on Java beats the C ABI in both directions.

So the retreat is not uniform. **In Java the generated codec is the recommendation;
in Python it is not a candidate at all.** Any recommendation that says "generate
the codec" without qualification is wrong for one of the five languages, and the
report has to say outcome 2 means *outcome 3 with Python on the core* — or accept
that Python keeps its own protobuf runtime, which is the one thing the maintenance
case exists to stop.

This is the most valuable thing work unit 1 produced and it cost one control arm.

## 2. The binding mechanism is settled, and two candidates are refused with evidence

Forward, per call from Python, every arm reaching the same callee in the same
shared library, net of the Python loop that drives it:

| mechanism | net ns | × a Rust forward crossing here |
|---|---|---|
| C extension | 11.4 - 12.5 | 4.1 - 4.5 |
| PyO3 | 41.5 - 42.3 | 14.8 - 15.1 |
| cffi, API mode | 65.4 - 66.3 | 23.4 - 23.7 |
| cffi, ABI mode | 184.8 - 190.0 | 66.0 - 67.9 |
| ctypes | 224.6 - 230.5 | 80.2 - 82.3 |

**The reverse direction is what decides it**, and README 9.1 predicted the shape
correctly: a `ctypes` or `cffi` callback costs 165 to 170 times a C-to-C call
through a function pointer, and six times the C extension's own interpreter
re-entry. **Compiling the extension fixes the forward direction and does nothing
at all for the reverse one** — cffi API mode is as slow as cffi ABI mode on the
callback. That is the cleanest possible confirmation that the crossing argument
is about the reverse path, and it is why both stay refused for the codec and
remain candidates for the RPC layer, where the count is two per call.

**PyO3's cost is its per-call wrapper, not its primitives**: an empty
`#[pyfunction]` already costs 47.9 to 49.9 ns while its primitives run 0.67 to
1.45 of the C-API spelling. A shim makes one forward call and thousands of
primitive calls, so this bears on the RPC layer rather than on the codec.

## 3. The facade storage verdict, and a correction to README 9.1

Only one storage wins, and not for the reason the design gave:

| storage, as the shim reaches it | ns per field read | crossings per element | P1.2 / upb |
|---|---|---|---|
| plain class, `PyObject_GetAttr` | 9.80 - 9.88 | 29 | 1.229 - 1.252 |
| `__slots__`, `PyObject_GetAttr` | 11.20 - 11.30 | 29 | 1.194 - 1.227 |
| C extension type, member descriptor | 11.22 - 11.29 | 29 | — |
| C extension type, **struct member read** | 0.48 - 0.50 | **7** | **0.600 - 0.612** |

**README 9.1's ordering is wrong in the middle term and its conclusion is right.**
The document says the three storages are "each more work than the last and each
faster". Measured through `PyObject_GetAttr`, which is what a C shim actually
calls, `__slots__` is *slower* than a plain class, and so is a C extension type
reached through its member descriptor. The three do not differ in how fast a
crossing is; they differ in **whether the shim can stop making one** — 29 per
element down to 7. I have amended 9.1 accordingly.

**And the slice found an argument the design did not anticipate**, which is now
the strongest one in that section: a specialised bytecode `LOAD_ATTR` costs 3.37
to 3.91 ns while `PyObject_GetAttr` from C with an interned key costs 9.44 to
9.52. CPython's interpreter has an inline cache and the C API has no equivalent
entry point, so **a C shim reading a plain facade does the same work about 2.5
times more slowly than the interpreter would.** Speaking the C API is not
unconditionally cheaper than being in Python; it is cheaper only when it lets you
stop crossing.

**The absent path separates the storages much further** — P1.3 at 0.934-0.953 for
the struct read against 4.433-4.643 for getattr — because the getattr shim pays
all 29 crossings on an element that encodes to nothing. That is the same shape the
rust slice found, where the absent path inverts a verdict.

**The cost this buys is not priced.** A C extension type is not an idiomatic
Python class, and the maintenance case rests on each language keeping hand-written
idiomatic types. A facade that must be a C extension type to be fast is a real
charge against that, and nobody has costed what it does to the public surface.

## 4. Open question 4 is now answerable on one of its three halves

**The wheel policy is cheap.** abi3 against the full C-API is 0.995 to 1.072 per
primitive, paired in-process — "no measurable difference on this workload" rather
than a ratio. The one real loss is structural rather than timed: `PyList_SET_ITEM`
is absent from the limited API, so a shim uses `PyList_SetItem` at 1.97 to 2.01
times the cost, and that survives a controlled rerun because it is an API fact.
Everything else the slice needs is in the limited API from 3.10, and **the
incumbent has already made this trade**: protobuf ships `cp3x-abi3` wheels and its
upb extension on disk is `_message.abi3.so`. So one wheel across 3.x is
affordable, and 9.2's "packaging cost against a per-field cost" is a smaller trade
than the section implies — with the caveat that the widest gap would be on decode,
where `PyList_SET_ITEM` is used per element rather than per field, and decode does
not exist yet.

**The floor is not demonstrable on this machine**: no python3.7, and apt lists
3.7.17. The incumbent does still exist there (`protobuf-4.24.4-cp37-abi3`,
`grpcio-1.62.3-cp37`), but it is three years older than the target's, so under R7
a floor-against-target ratio would be a comparison of library versions as well as
of runtimes. **The free-threaded arm cannot be measured here at all** — no 3.13t
build and none installable from this container — so whether 3.13t is a target, an
arm or out of scope stays open on the evidence rather than on the argument.

## 5. Two numbers the design documents should carry

- **Releasing the GIL costs 34.2 to 34.7 ns**, about twelve C-API primitives. That
  bounds when 9.1's "a longer window in which the pure parse runs with the GIL
  released" is worth doing, and it is the only input the branch has to the GIL
  question.
- **The UTF-8 passthrough is free only for ASCII on CPython.** Reading a `str`'s
  UTF-8 is 2.19 to 2.27 ns for ASCII and **64.2 to 67.8 ns** for Latin-1 and
  above-U+00FF when the object has no UTF-8 cache — which is the state a string
  that came off the wire is in. ArmoniK's identifiers are ASCII GUIDs, so the
  common path is the cheap one, but this also means ABI v1's `ak_tc_latin1` and
  `ak_tc_ucs4` would be competing with CPython's own cache rather than with
  nothing. Noted in decision 3's "what survives".

## 6. What the slice's defect log is worth to the other slices

Four defects, **every one found by something running rather than by review**, and
two of them are branch-level rather than Python-level:

- **D4: the committed tree could not rebuild itself.** A generated module carried
  the ffi root as an absolute path baked in at generate time, so a clone of the
  branch at any other path regenerated a different file and `--check` refused the
  whole tree as stale. Found by cloning the pushed branch and building it. **The
  practice that came out of it — clone the pushed branch to a different path and
  build it as the last step of a work unit — should be every slice's**, and it is
  R4's closing rule turned into a command rather than a principle.
- **D3: the repository `.gitignore` ate a generated source directory for the
  fourth time in this branch.** The root file excludes any directory named `gen`,
  and `ffi/.gitignore`'s re-inclusion did not reach a nested one. Four occurrences
  is not four slice defects, it is one branch defect, and I have widened the
  re-inclusion rather than leave the fifth to be discovered.
- D2 is worth reading beside R2's floor rule: the floor arm was `bytes(ref)` on a
  `bytes` object, which returns the same object and copies nothing, reading 58 ns
  at 858 B and at 218 KB alike. **A floor that is not doing the work is worse than
  no floor**, because it licenses exactly the claims the floor exists to bound.

## 7. What is not established, and one of them is the slice's own question

The slice's list is long and honest; these are the ones that bear on the report.

- **"Can it survive the GIL" is not answered.** Nothing runs two threads. The
  slice measured what releasing the GIL costs and stopped there, which is the
  input to the question rather than the answer.
- **Decode is entirely unmeasured, and the storage verdict may not survive it.**
  Constructing one facade element costs 40.7 to 41.7 ns — as dear as a full
  interpreter re-entry, and more than 300 times a struct member *read*. What a
  struct member *write* costs was not measured. So "the C extension type wins" is
  an encode statement, and decode is where ABI v1's two delivery families (7.1)
  first become a real choice for this host.
- **No arm is the design end to end.** The shim-to-facade edge (a crossing per
  field) and the shim-to-core edge (a forward call at 22.2-23.1 ns, a C-to-C
  reverse at 1.42 ns) are each priced; neither composition exists.
- **No shape coverage beyond M1**: no oneof, no explicit presence, no map, no
  packed field, no repeated string, no adapter site, no bulk bytes. The generator
  raises rather than skips on anything outside the M1 subtree, so the scope is
  enforced by the build rather than claimed — which is R1 working as intended, and
  it means the gap is visible rather than silent.
- **No unknown-field vectors** (they are a decode obligation), no P2.5, no content
  sets on a whole message, no allocation column.

## 8. What this asks of the design documents

1. **README 9.1's storage parenthetical**: amended. The three storages differ in
   whether the shim stops crossing, not in how fast a crossing is.
2. **README 9.1 gains the inline-cache argument**: a C shim reading a plain facade
   is about 2.5 times slower than the interpreter reading the same attribute.
3. **ABI v1 decision 3**: `ak_tc_latin1` and `ak_tc_ucs4` compete with CPython's
   own UTF-8 cache on this host, at 64.2-67.8 ns uncached.
4. **README section 2**: a Python forward crossing through a C extension is 11.4
   to 12.5 ns net, 4.1 to 4.5 times a Rust crossing on the same machine. The
   number that belongs beside the managed rows is not the interpreter re-entry
   (39.9-40.5 ns, between .NET and JNI) but the C-API primitive at 2.83-20.54 ns,
   **because the whole point of 9.1 is that this design does not make that call.**
5. **`design/SHAPES.md` needs nothing.** The M1 subtree, the value rules and the
   P1.x hashes reproduced exactly, in four independent encoders, first time.
