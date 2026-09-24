# python slice: journal

What was tried, what it measured, what refuted it, in order.
Append; do not rewrite. A later reader comes here to find out that an option
was already refuted and by what.

Every figure below is the spread of three separate processes on python 3.11 on
this machine unless it says otherwise, and every one names the log it came from.
`mech/summarise.py <log>` regenerates any table here from the file beside it.

## Entries

### J1. R13 first: the Rust slice's crossing benchmark, on this machine

Before anything of this slice's own was measured. `ffi/poc/rust` builds here
unmodified (protox is pure Rust, so the missing system `protoc` is not a
problem) and `AK_BENCH_ONLY=P1.1 target/release/bench` keeps the crossing rows,
which `bench.rs` retains unconditionally whatever the filter.

**This machine: 2.8 ns per forward crossing, 2.1 ns per forward-plus-reverse,
against 1.8 ns for both on the Rust slice's container.** Three processes, spread
under 0.1 ns on both rows. So this container is 1.2 to 1.6 times slower on a
crossing, and every absolute below is also quotable as a multiple of 2.1 to
2.8 ns. `ffi/logs/python/00-r13-rust-crossing.log`.

**Unexpected, and recorded rather than explained**: on the Rust slice's machine
the forward and the forward-plus-reverse rows were both 1.8 ns; here they
separate, and the row with *more* work in it is the cheaper one. The two loops
differ in more than the callee -- the reverse one passes a function pointer and
the forward one does not -- so this is a property of those two loops on this
microarchitecture rather than a finding about crossings. It is why the
calibration is carried as a range and not as a number.

The boundary is real, checked from the artifact rather than claimed (R5): the
harness imports 30-odd undefined `ak_*` symbols and carries `libak_core.so` as a
`NEEDED` entry.

### J2. Mechanism: the candidate list of README 9.1 survives, by a wide margin

Every mechanism arm calls the SAME callee (`ak_noop` in `libakmech_cabi.so`), so
the rows differ in the mechanism and in nothing else (R7).
`ffi/logs/python/30-mechanism-py3.11.log`, group 1.

| mechanism | ns per call | net of the 10.4-11.7 ns empty Python loop |
|---|---|---|
| C extension, `METH_O` | 22.2 - 23.1 | 11.4 - 12.5 |
| C extension, `METH_FASTCALL` | 22.8 - 23.6 | |
| C extension, abi3 | 22.2 - 23.3 | |
| PyO3 | 52.4 - 53.3 | 41.5 - 42.3 |
| cffi, API mode | 76.7 - 77.1 | 65.4 - 66.3 |
| cffi, ABI mode | 195.2 - 200.6 | 184.8 - 190.0 |
| ctypes, `argtypes` declared | 235.0 - 241.1 | 224.6 - 230.5 |
| *(a pure-Python function call)* | 40.3 - 40.9 | 29.2 - 29.9 |

**A C-extension forward crossing is cheaper than a Python function call**, by
about 2.4 times net. That is the row that makes the rest of the design plausible
at all in this host.

`ctypes` without `argtypes` measured 143.0 - 145.4 ns, faster than the declared
form -- **and the correctness gate refuses it**: it truncates the return to
`int`, so `ak_noop(2**31)` comes back as `18446744071562067970`. It is not a
faster mechanism, it is a mechanism not doing the work. The gate exists because
the first table had that row in it looking like a win.

### J3. PyO3 is dear in the wrapper and cheap in the primitives, and those are two different findings

PyO3's forward row is 2.3 times the C extension's on the same callee, which is
surprising enough to need floor arms (R2). Three of them split it: a
`#[pyfunction]` with **no arguments at all** already costs 47.9 - 49.9 ns, one
`i64` argument 49.2 - 50.4, one untyped argument 46.5 - 47.4, and the version
with the crossing removed 51.3 - 51.9. So it is **the per-call wrapper**, not
argument conversion and not the crossing.

But README 9.1 calls PyO3 "the same C-API calls with a Rust spelling", and at the
level of the *primitives* that is close to true: group 3d puts eight of them side
by side and PyO3 runs **0.67 to 1.45** of the C-API spelling, faster on
`extract::<i64>` (0.67 - 0.68, 8.9 ns against 13.2) and slower on `PyString::new`
(1.38 - 1.42) and `PyList::new` (1.16 - 1.19). One row, `PyList::append`, spans
0.77 to 1.45 on its own because the C-API arm threw a 10.9 ns outlier in one of
the three processes; it is the reason the range is quoted and not a mean.

Since a shim makes **one** forward call and **thousands** of primitive calls per
message, the wrapper cost is the less important of the two -- but it is not
nothing for the RPC layer, where the crossing count is two per call and there is
no per-field work to hide it behind.

Not refuted, not pursued: whether a cheaper PyO3 spelling exists. PyO3 0.29.2,
`--release`, LTO off. A different version may differ.

### J4. The premise holds at the primitive level, and the margin is large

README 9.1's premise is that the core should make **C-API calls on primitives**
rather than **calls into the interpreter**. Both measured, per reverse call,
driven from inside the C library so the forward cost is amortised to nothing.

- Into the interpreter: **39.9 - 40.5 ns** (C extension), 51.0 - 51.2 (PyO3),
  119.4 - 119.8 (ctypes `CFUNCTYPE`), 233.8 - 239.9 (cffi, **both** modes --
  5.93 to 6.01 times the C extension's own re-entry, paired per process).
- As a C-API call on a primitive: **2.83 - 20.54 ns**, the whole group 3 range,
  with the string read at the bottom and `PyList_New(4)` at the top.
- The floor both are quoted against: a C-to-C call through a function pointer,
  **1.41 - 1.42 ns**, with the loop alone at 0.18.

So the cheapest interpreter re-entry is **28 times** the C-to-C floor and the
typical primitive is **2 to 15 times** it. `cffi`'s callback is **165 to 170**
times it, in API mode as well as ABI mode: compiling the extension does not
change what a callback does.

### J5. The premise again, over a whole message, which is the number that counts

Per-primitive figures do not say how many primitives a message needs. The codec
arms encode `ListResultsResponse` over the M1 subtree, all generated from
`shapes.json` by one walker (R1), byte-identical to the validated manifest on
P1.1, P1.2 and P1.3 including the absent path, before anything was timed (R2).
`ffi/logs/python/40-codec-py3.11.log`.

The control 9.1 asks for is `cshim pyacc`: **the same generated C traversal, the
same counted crossings**, with each field reached by a call to a Python-level
accessor instead of by a C-API read. A within-arm delta, which is what R4 asks
for. Against upb:

| payload | C-API read | Python-accessor call | the control's penalty |
|---|---|---|---|
| P1.2, 1,000 elements | 1.229 - 1.252 | 3.984 - 4.085 | 3.2 to 3.3 times |
| P1.1, 4 elements | 1.817 - 1.904 | 6.072 - 6.407 | 3.3 to 3.5 times |
| P1.3, the absent path | 4.433 - 4.643 | 24.665 - 25.200 | 5.4 to 5.6 times |

A C-implemented accessor (`operator.attrgetter`), called the same way, recovers
only about a tenth of it (3.584 - 3.703 on P1.2), which says the cost is **the
call**, not the fact that the accessor is written in Python.

**The extra layer earns its place.** README 9.1's last bullet is answered for
encode on this shape: it is clearly worse, by a factor of three or more.

### J6. The storage question, and 9.1's parenthetical ordering is wrong in the middle

9.1 says: "a plain class (a dict lookup), a `__slots__` class (a descriptor
offset), or a C extension type (a struct member, so a field read stops being a
crossing at all). Each is more work than the last and each is faster."

The first half of that ordering does not reproduce. Through `PyObject_GetAttr`,
which is what a C shim actually calls (group 4, and the same shape on all four
interpreters in `31-mechanism-all.log`):

| storage, as the shim reaches it | ns per field read |
|---|---|
| plain class | 9.80 - 9.88 |
| `__slots__` class | 11.20 - 11.30 -- **slower** |
| C extension type, through its member descriptor | 11.22 - 11.29 -- also slower |
| C extension type, **struct member read** | 0.48 - 0.50, against a 0.36 loop floor |
| *(`PyObject_GetAttrString` on a plain class)* | 49.7 - 52.7 |

`__slots__` buys a C shim nothing: the descriptor call costs about what the dict
lookup saves. Only the third storage moves, and only when the shim casts and
reads the struct rather than going through the descriptor -- which is exactly
9.1's "a field read stops being a crossing at all", so the **conclusion is right
and the ordering that leads to it is not**.

Net of its loop floor the struct member read is **0.12 - 0.14 ns**, which is at
the resolution limit of this harness; the honest statement is "at or below the
floor", not a number.

Over a whole message it is **29 crossings per element against 7**, counted from a
counting build (`20-conformance.log`), and **1.229 - 1.252 of upb against 0.600 -
0.612** on P1.2. On the absent path the gap is much wider (4.433 - 4.643 against
0.934 - 0.953), because the getattr shim still pays all 29 crossings on an
element that encodes to nothing while upb pays almost nothing -- the same shape
as the Rust slice's finding that the absent path inverts a verdict, showing up
here for one storage and not for the other.

`PyObject_GetAttrString` is in the table as the trap it is: no generated shim
would emit it, but it is the obvious thing to write and it is five times the
interned-key form, because it rebuilds the name string per call.

### J7. A bytecode attribute read is cheaper than `PyObject_GetAttr` from C

Measured as a within-arm delta so the Python loop cancels: one, then four,
`o.session_id` in the same loop shape. Marginal cost of one read: **3.37 - 3.91
ns**. `PyObject_GetAttr` from C, with a pre-interned key and its own loop floor
subtracted: **9.44 - 9.52 ns**.

CPython's specialising interpreter has an inline cache for `LOAD_ATTR` and the C
API has no equivalent entry point, so **a C shim reading a plain facade through
`PyObject_GetAttr` does the same work about 2.5 times more slowly than the
interpreter would.** This is the sharpest argument in the slice for the
C-extension facade type, it is not anticipated by anything in the design
documents, and it is a fact about CPython rather than about this harness.

### J8. R9 confirmed: the pure-Python control loses by an order of magnitude

The generated pure-Python encoder is **19.4 - 20.3 times upb** on P1.2, 13.6 -
14.1 on the absent path, 25.8 - 27.3 on the small flat payload, and 19.3 - 28.3
across the four interpreters. README R9 says in advance that this is a result and
not a defect, and it is: it says the codec question in Python is native against
native, and it removes "generate a codec into Python" from the option list on the
codec path in a way it is not removed for Java, where the same control **won**.

### J9. abi3 costs almost nothing here, and one artifact really does load everywhere

One `.so` built against `Py_LIMITED_API=0x030A0000` **on python 3.10** is loaded
and exercised by 3.10, 3.11, 3.12 and 3.13 in the same tables
(`10-build.log` runs it on each, `30-mechanism-py3.11.log` times it beside the
full-API build). Forward call 22.2 - 23.3 ns against 22.2 - 23.1 for the
full-API build: the same row.

Across seven primitives, paired per process, abi3 / full-API is **0.995 to
1.072**. The dearest are `PyList_Append` (1.055 - 1.072) and `PyList_SetItem`
(1.058 - 1.070); `PyUnicode_FromStringAndSize` is 0.995 - 1.002.

**One operation is genuinely absent.** `PyList_SET_ITEM` is a macro and not in
the limited API, so a shim must use `PyList_SetItem`: 3.74 - 3.76 ns against
1.86 - 1.90, which is **1.97 to 2.01 times** on that one call. Everything else
this slice needs is in the limited API from 3.10, `PyUnicode_AsUTF8AndSize`
included -- the bare `PyUnicode_AsUTF8` is not, which the abi3 build says by
failing to declare it, and one spelling now serves both builds.

Worth knowing beside it: **`protobuf` itself ships abi3 wheels**
(`protobuf-7.36.2-cp310-abi3-...whl`, and its upb extension on disk is
`_message.abi3.so`). The incumbent has already made this trade.

### J10. Two numbers the design documents will want, found on the way

**Releasing the GIL costs 34.2 - 34.7 ns** on 3.11 (34.3 to 43.4 across 3.10 to
3.13): `Py_BEGIN_ALLOW_THREADS` plus `Py_END_ALLOW_THREADS` with nothing between
them. That is about twelve C-API primitive calls, or most of an interpreter
re-entry, and it bounds when 9.1's "a longer window in which the pure parse runs
with the GIL released" is worth doing at all.

**On CPython the UTF-8 passthrough is free only for ASCII.** Per SHAPES.md's
three content sets, reading a `str`'s UTF-8 with `PyUnicode_AsUTF8AndSize` is
2.83 ns (ASCII), 3.46 - 3.49 (Latin-1) and 3.47 - 3.50 (above U+00FF) **when the
object already carries a UTF-8 cache**. The uncached case is the one that
matters, because a string that came off the wire has none, and measured as a
within-arm delta (build+read minus build) it is **2.19 - 2.27 ns for ASCII** and
**64.2 - 66.0 (Latin-1) and 67.0 - 67.8 (above U+00FF)**: CPython allocates and
fills the cache on the first read, at about 29 times the ASCII cost.

So `ak_tc_utf8` really is a pointer return for ArmoniK's actual content -- every
id in the real schema is an ASCII GUID -- and ABI v1 section 4's `ak_tc_latin1`
and `ak_tc_ucs4` entries would, on CPython, be competing with CPython's own
cached conversion rather than with nothing.

### J11. Two defects in this slice's own probes, both found by a control

**D1.** The C-extension forward arm used `PyLong_AsLongLong` and raised above
2^63 while the PyO3 and cffi arms answered. Found by the correctness gate, not by
reading the code. Fixed to `PyLong_AsUnsignedLongLong`. Without the gate the
mechanism table would have compared arms doing different work and nothing would
have said so.

**D2.** The codec benchmark's floor arm was `bytes(ref)` where `ref` is already
a `bytes` -- which returns the same object and copies nothing. It read 58 ns at
every payload size, 858 B and 218 KB alike, which is what a floor that is not
doing the work looks like. `memoryview.tobytes()` copies: the 218 KB floor is
5.69 - 5.86 us, 1.8 percent of upb's own time on that payload. Both arms sit far
above it, so the sub-1.0 ratios survive the check R2 asks for.

### J11b. A third and a fourth defect, both in how the work is kept rather than in what it measures

**D3.** The repository-wide `.gitignore` excludes any directory named `gen`.
`ffi/.gitignore` re-includes `poc/*/gen/**` for exactly that reason, and this
slice's generator sits one level deeper, at `poc/python/mech/gen/`, where the
re-inclusion does not reach. `git add -A` listed 24 files and not one of them was
the generator. Found by reading what `git status` did **not** list, which is the
same way the Rust slice found it for `bin/`. This is the fourth word that file
has eaten in this branch (`logs`, `gen`, `bin`, and now `gen` again one level
down). Fixed by a re-inclusion in this slice's own `.gitignore`, checked with
`git check-ignore` rather than assumed.

**D4.** `gen/out/payload_values.py` carried the `ffi/` root as an absolute path
baked in at generate time. A clone of the pushed branch at any other path
therefore regenerated a different file, `generate.py --check` refused the tree as
stale, and `build.sh` -- which runs that check as a build step -- refused to
build the codec module at all. **The committed sources could not rebuild
themselves.** Found by cloning the pushed branch into a temporary directory and
building it, which is now the last step of a work unit rather than an idea.
Fixed: the generated module walks up until it finds `schema/shapes.json`.

No measurement is affected. `_akcodec_gen.c` is byte-identical across the fix and
the only file that changed computes paths, so the logs stand as taken.

### J11c. The C++ slice's calibration lands, and it isolates the odd row in J1

The exploration branch merged the C++ slice at `4aec4e8`, and it ran the same
R13 calibration on its own container: **1.5 ns forward, 2.1 to 2.2
forward-plus-reverse**, against this machine's **2.8 forward, 2.1
forward-plus-reverse** and the Rust slice's **1.8 for both**.

Three containers, and the forward-plus-reverse row agrees between two of them to
0.1 ns while the forward row spans 1.5 to 2.8 -- a factor of 1.9. So the thing
that does not travel is **the forward loop**, not crossings. J1 recorded the
asymmetry on this machine as a property of the two benchmark loops rather than a
finding about crossings, and a second container disagreeing in the opposite
direction (1.5 < 2.1 there, 2.8 > 2.1 here) is what that reading predicts.
Practical consequence: where one calibration number is wanted, the
forward-plus-reverse figure is the one to use.

### J11d. Scope change: absolutes are instrumentation, and three rankings are recorded as ambiguous

Relayed 2026-09-19, merged as `4aec4e8`. Nobody tries hard at cross-language
performance until the comparison is re-taken on a controlled physical machine.

Nothing here is withdrawn by it and nothing is re-taken, because what this work
unit was asked to produce is what the note says a rerun cannot: counted
crossings (29 against 7 per element), byte identity against the validated
manifest, the within-arm delta that settles README 9.1's premise (3.2x to 5.6x),
and feasibility. The rankings it produced are sign-and-magnitude ones -- 2x on
storage, 5x on the absent path, 6x on the cffi callback -- not percentages.

Three comparisons WERE close, and the instruction is to record rather than grind:

- **plain against `__slots__` through `PyObject_GetAttr`.** The sign flips
  between payloads and inverts again at the primitive level. Left ambiguous. The
  refutation of 9.1's ordering does not depend on it: that only needs
  "`__slots__` is not clearly faster", which three measurements agree on.
- **`METH_O` against `METH_FASTCALL`.** 22.2 - 23.1 against 22.8 - 23.6,
  overlapping. Left ambiguous; the mechanism is a C extension either way.
- **abi3 against the full C-API per primitive.** 0.995 to 1.072 paired per
  process is "no measurable difference on this workload", and quoting it as a
  ratio would be quoting noise. The one difference that survives a controlled
  rerun is not a timing at all: `PyList_SET_ITEM` is absent from the limited API.

### J12. What was NOT tried, and why none of it is a refutation

- **Decode.** Nothing in work unit 1 decodes. The storage verdict may well differ
  there: decode *constructs* objects, and `PyObject_CallNoArgs` on a type is
  40.7 - 41.7 ns, as dear as a full interpreter re-entry and about 90 times a
  struct member write. ABI v1 open decision 10 is about exactly that cost in
  another host.
- **The Rust core.** The codec arms have no core in them; they price the
  shim-to-facade edge, which is the one that is a crossing per field. The
  shim-to-core edge is priced separately as J2's forward row and J4's 1.42 ns
  C-to-C floor.
- **M2 through M7, and every payload but P1.x.** The generator **raises** on a
  message outside the M1 subtree rather than skipping it (R1), so the scope is
  enforced by the build and not merely documented.
- **Free-threaded CPython.** No free-threaded interpreter exists on this machine
  and none is installable from its apt, so the arm cannot be built here at all
  rather than having been declined. `01-environment.log`.
- **Python 3.7**, the floor `pyproject.toml` declares. Not present; apt lists
  3.7.17-1+noble2, so it is reachable. Not installed, because the floor is the
  aggregating session's to choose and installing one is not free of consequence
  for the other arms. **The incumbent does still exist there**, which I had
  guessed the other way before checking: pip resolves
  `protobuf-4.24.4-cp37-abi3` and `grpcio-1.62.3-cp37` for 3.7. So a 3.7 floor
  arm is possible but its incumbent would be three years older than the target's,
  which makes any floor-against-target ratio a comparison of library versions as
  well as of runtimes -- README R7's hazard, and README 5.2's reason for arm b.

## Work unit 2

### J13. The slice's generator is now one generator

Work unit 1's `mech/gen/generate.py` had its own facade emitter. Work unit 2 needed the
same facade, and two emitters for one facade is README R0's defect one level down from the
core. The facade, the pure-Python codec, work unit 1's no-core C shim and the new binding
are now emitted by one generator at `gen/`, which imports the shared core's `ir.py` and the
cpp slice's `cpp_header.py` read-only. `mech/` reads `gen/out/` like everything else.

The emitted facade and `_akcodec_gen.c` are byte-identical across the move, so work unit
1's columns still trace to the files that produced them. `pycodec.py` gained a decode half
and later two bounds fixes (J18), and its encode half is unchanged.

### J14. The composed arm: the design, finally, rather than one of its edges

`gen/py_binding.py` emits a CPython shim over the shared core at `poc/codec/`. Three
accessor backends over one traversal, the same three work unit 1 used, so the two are
comparable. **R0: nothing is copied.** `poc/codec/gen/one_core.sh` passes, the shim links
`libak_core.so` by path, and `build.sh` proves the boundary from the artifact -- 14
undefined `ak_*` imports and `libak_core.so` as a `NEEDED` entry, in both the measured and
the counting build.

**ABI v1 decision 9's sparse fill is what it emits**, from the first line rather than
retrofitted: the host memsets the element-group chunk once and assigns only what differs.
A memset to zero IS the group's default -- `tc == NULL` is absent, a zero scalar is the
proto zero, a zero presence word is no child -- so the sparse fill and the canonical form
agree by construction rather than by care.

Correctness first (R2): byte-identical to the validated manifest on P1.1, P1.2 and P1.3 on
3.10, 3.11, 3.12 and 3.13, encode and decode, with decode checked BOTH by re-encoding to
the same bytes and field-by-field against the incumbent. ABI version and all 380 group
layout facts checked at import (section 10).

### J15. Crossings: the ABI is not where Python's crossing problem is

Counted in both halves, because neither can count the other's
(`51-conformance-wu2.log`). Per element, P1.2:

| | shim -> CPython | core, forward | core, reverse |
|---|---|---|---|
| encode, C extension facade | **7.00** | 0.01 | 0.00 |
| encode, `PyObject_GetAttr` | 29.00 | 0.01 | 0.00 |
| decode, C extension facade | **7.00** | 0.00 | 0.01 |
| decode, `PyObject_GetAttr` | 22.00 | 0.00 | 0.01 |

**The core's own boundary costs one hundredth of a crossing per element** -- the batched
element run turns 1,000 elements into about ten entries -- while the shim makes 7 to 29
crossings into CPython for the same element. So in Python the crossing argument is not
about the ABI at all: the ABI's crossings are already negligible and every crossing that
matters is between the shim and the facade. That is a different statement from every other
slice's, and it is what README 9.1's three layers predict.

### J16. Decode inverts the encode verdict, and the floor arm is what explains it

P1.2, seven independent processes across 3.10 to 3.13, against upb on the R14 production
path:

| | encode | decode (the call) | decode + read every field |
|---|---|---|---|
| core-ffi / C ext type | **0.695 - 0.717** | 1.765 - 1.997 | **0.887 - 0.912** |
| core-ffi / `__slots__` | 1.49 - 1.59 | 3.70 - 3.75 | 1.026 - 1.056 |
| core-ffi / plain | 1.59 - 1.78 | 3.96 - 6.93 | 1.059 - 1.082 |
| core-ffi / pyacc | 4.91 - 5.63 | 11.29 - 11.74 | 1.688 - 1.747 |
| pycodec (no boundary) | 25.4 - 25.7 | 33.0 - 33.3 | 3.56 - 3.68 |

Taken at face value the middle column says the composed arm regresses on decode. **The
floor arm says the middle column is not comparing the same work.** Constructing 1,000 bare
facade objects and copying the input -- no parsing at all -- is **0.80 to 0.84 of upb's
entire decode**. A decode cannot cost less than producing what it produces, so upb is not
producing it: `FromString` parses into a upb arena and materialises a Python object only
when something reads it.

The right-hand column puts both on the same work and the composed arm is back under 1.

### J17. upb does not cache, and that is the sharpest thing in this slice

Measured directly rather than inferred. A second full read of the SAME upb message costs
**3.04 to 3.13 ms** against 3.44 for the first, so all but about 12 percent of the
materialisation is paid **again**. The facade's second read is **2.37 to 2.48 ms**, because
the values are Python objects and stay Python objects.

Two consequences, and the second is the one for the report:

- the like-for-like decode comparison above is if anything generous to upb, since it
  charges the materialisation once;
- **a caller that reads its response twice pays upb twice and the facade once.** For a
  control-plane client that lists results and walks them more than once, the gap widens
  rather than closes.

**This refutes the hypothesis the branch was carrying into decision 13.** The suggestion
was that upb may already be doing borrowed spans at the Python level, so the comparison is
fairer than it looks. It aliases strings into its input buffer *in C*
(`upb/wire/decode.h:29`), and a Python `str` is a fresh object every time, built on every
attribute read. There is nothing borrowed at the Python level at all. Decision 13's
borrowed-span idea is therefore **available to the facade and not already taken by the
incumbent**, which is the opposite of what the open question assumed -- though what a
borrowed Python `str` would even be (a `memoryview`, a lazy `str` subclass) is a facade
question nobody has drafted and this slice did not build.

### J18. The corpus's first consumer, and it found three defects

48 of 336 rows root at `ListResultsResponse`, which is what this slice's scope can reach.
Rule 0 is checked rather than asserted: the three messages are identical to `corpus.proto`
and the superset adds seven `u_*` fields this reader does not know, so the reader is the
reader.

**Two defects were mine, both in the generated pure-Python codec, both fixed:**

- **D5**: it bounds-checked a nested length against the whole buffer instead of against the
  enclosing message, so it accepted `X-nested-len-overrun` -- a well-formed outer frame
  whose inner length reaches into a neighbouring field. Exactly the failure the vector's own
  `why` predicts. The core rejects it correctly (`AK_ERR_TRUNCATED`).
- **D6**: it raised on an unknown field of the deprecated GROUP form (wire type 3) instead
  of skipping it. proto3 cannot express a group, so nothing generated from the schema
  contains one, and a conformant parser still has to skip it.

With both fixed the pure-Python arm passes **47/47 accept and 1/1 reject**.

**The third is in the shared core and is not mine to fix.** `ak_decode_*` returns
`AK_ERR_MALFORMED` on `U-root-group` and `U-nested-group`; upb accepts both, and I
confirmed that locally rather than taking the manifest's word for it. Every conformant
parser must skip an unknown group. It is a defect in `poc/codec/`, it affects every slice,
and R0 says a change to existing behaviour goes to the aggregating session. Reported, not
patched.

Decision 11, answered for python: **this slice drops unknown fields** (29 of 34
unknown-class rows re-encode to the `unknown-dropped` form; the other 3 have nothing to
drop at the re-encode). The core carries `ak_unk_f` vtable slots and this shim passes NULL
for every one, so the drop is the binding's choice and not a limit of the ABI.

### J19. One exploratory run disagreed with seven, and the seven win

An early single-process run put encode P1.2's C-extension arm at 0.999 of upb; the three
committed processes and four cross-interpreter ones put it at 0.695 to 0.717. That run's
upb row was also 15 percent slower than every later run's, so the whole process was slow
rather than that one arm being fast. Recorded because the discarded number was the
conservative one, and discarding a conservative outlier is exactly the move that needs to
be visible.

### J20. Two name collisions, both of the kind that measures the wrong thing silently

- `mech/arms.py` and `arms.py` are two modules called `arms`. Putting `mech/` on `sys.path`
  to reach the shared harness made `import arms` resolve to work unit 1's table. Caught by
  an `AttributeError` this time; the version of this that does not raise is the one to
  fear. The harness is now loaded by path and no directory that could shadow a module goes
  on the path.
- The measured core and the counting core are both `libak_core.so`, so the first one loaded
  satisfies the other shim's `NEEDED` entry and the counting build silently gets the
  non-counting core. That is how the first crossing counts came out as zeroes. The counting
  pass now runs in a subprocess of its own.

## Work unit 3: M2

### J21. The verdict moves on M2, and it moves in both directions

M1's element is a **leaf**, so the batching predicate admits it and a thousand elements
cross in about ten core calls. `TaskDetailed` is not a leaf. Everything M1 measured rested
on that property and this is the payload that tests it.

P2.2, the shape ArmoniK's control plane actually moves, against upb on the R14 production
path (`/tmp` single-process figures confirmed by the committed three-process run):

| | M1 / P1.2 | M2 / P2.2 |
|---|---|---|
| encode, C-extension facade | **0.700 - 0.719** | **1.408** |
| decode, the bare call | 1.773 - 1.804 | **2.416** |
| decode + read every field | 0.888 - 0.897 | **0.629** |
| re-read every field, facade against upb | 0.79 | **0.49** |

**Encode regresses and like-for-like decode improves**, and both have the same cause seen
from two sides. The non-leaf element costs crossings: 10.02 core crossings per element
against M1's 0.01, and 51.67 shim crossings against 7. That is what makes encode 1.41.
But M2 also carries far more content per element -- 17,500 strings and 2,000 map entries
in 540 KB -- and upb defers all of that materialisation, so the more content an element
has, the more the bare-call comparison flatters upb and the more the like-for-like one
does not.

So the answer to "does the M1 verdict survive an element that gains containers" is: **the
encode half does not, and the decode half gets better.** A report that quotes only one of
them is quoting half the slice.

### J22. The crossing counts reproduce the rust slice's to the digit, from another host

Counted in the core, in this process, through the python shim
(`logs/python/53-conformance-m1m2.log`):

| | this slice | ABI v1 section 6 / the rust slice |
|---|---|---|
| encode, per `TaskDetailed` | 5.02 forward + 5.00 reverse = **10.02** | **10.02** |
| decode, per `TaskDetailed` | **7.00** | **7.004** |
| encode, per `ResultRaw` | 0.01 | 9 per 1,000 elements |

Five loop slots on `TaskDetailed` -- four repeated string fields and a map -- and each is
one reverse call (the loop callback) plus one forward call (`ak_blob_run`, or
`ak_elem_TaskOptionsOptionsEntry`) per element. R5 says a crossing count is what makes a
result portable to a runtime nobody measured; here it is the same number in a second
language over the same core, which is the strongest form that claim has taken on this
branch.

### J23. Two defects, and one of them is ABI v1 decision 10 in the flesh

**D8.** Decode returned `options.options == {}` on every M2 payload while encode was
byte-perfect. Cause: `apply` constructed a fresh inlined `TaskOptions` and assigned it,
discarding the one the map's `add_` run had already created. Decision 10 says exactly
this -- "runs may arrive before the group fields, so a binding that constructed from the
group would discard them" -- and I built the binding that discards them anyway. Decode now
**gets-or-creates** every child instead of constructing one.

It is worth saying what caught it: **not** byte identity of the re-encode, which passed,
because the re-encode of a facade with an empty map is a legal encoding of a different
message. The field-by-field comparison against the incumbent caught it, and that
comparison is driven from the incumbent's own descriptor rather than from a list this
slice wrote, which is why it noticed a field the slice had lost.

**D9.** The like-for-like reader walked nested messages with `dir()`. On M1 that is two
Timestamps per element and it did not show; on M2 it is fourteen nested messages per
element, and it made the facade's re-read 2.8 times upb's on a payload where M1's was
*cheaper*. Entirely a fact about the reader. Both readers now follow one plan built from
the description and perform an identical number of reads -- 444,701 on P2.2, checked
rather than assumed -- and the facade's re-read comes out at 0.49 of upb's.

The first version of that table would have been published as "the composed arm's decode
gets worse on the shape that matters". It was wrong by a factor of four.

### J24. The map forces the incumbent's canonical form open, and both forms are legal

`SerializeToString` does not sort map entries; the canonical form in the manifest does. And
on P2.5 upb writes an empty map value as a present zero-length field where the canonical
form omits it. Neither is wrong -- `ffi/corpus/CONTRACT.md` C3 says so and lists 85 rows
with more than one accepted form.

So the conformance gate now distinguishes: this slice's own arms must produce the
canonical bytes, and the **incumbent** may produce any encoding that parses to the same
message, checked with the incumbent itself as the oracle. `deterministic=True` is carried
as a labelled second row and costs 1.021 to 1.025 of the production path on encode, which
is the price of the canonical form in the incumbent rather than in the core.

### J25. The map's sort is the shim's, and it is visible

The facade holds a `dict` because that is what a Python user expects; the wire wants
entries sorted by key. So the shim calls `PyDict_Keys` and `PyList_Sort` per element. It
is a cost the incumbent does not pay on its default path and this slice does not hide it:
it is inside the 1.408 encode figure, not beside it. A facade that held a sorted structure
would not pay it and would be less idiomatic; nothing here prices that trade.

### J26. One arm moved 1.8x between two runs of the same benchmark, and it was the allocator

Work unit 2 put the composed arm's P1.2 encode at **0.68-0.72 of the incumbent**. Work
unit 3, same machine, same core, same shim, same bytes, put it at **1.26**. The composed
arm's absolute had not moved at all (246-249 us then, 247-248 us now). The incumbent's
had: 344-358 us then, 195-197 us now. So the question was never "why did our arm get
slower", it was "why did upb get 1.8x faster", and until that was answered no P1.2 encode
ratio was reportable (R2).

**The first hypothesis was the fixture, and it is refuted.** Between the two runs
`build_upb` changed from a hand-written field-by-field copy to `FromString`, and then
`build_upb_native` was added to serialise a message built through protobuf's own setters
the way production does. A message out of the parser has an arena the parser laid out; a
message out of the setters has one the setters laid out; it is a reasonable suspect. All
three, in one process, in interleaved rounds, on P1.2:

| fixture | median |
|---|---|
| work unit 2's hand-written copy | 345,336 ns |
| `build_upb_native`, setters | 347,742 ns |
| `FromString` | 346,036 ns |

Within 1%, all three byte-identical to the manifest -- and all three at work unit 2's
*slow* number, in a process that does nothing but this. So the fixture is not it, and the
fast number is the one that needs explaining.

**It is the process's malloc state.** A single `bytes(1 << 20)` allocated and freed before
the measurement takes P1.2's upb encode from 341,453 ns to 194,588 ns and leaves it there.
A `bytearray` does it, serialising P2.4 once does it, and serialising *another* 218 KiB
message does not -- it has to be bigger. Building P2.4 without serialising it makes things
worse, not better, which rules out a warm cache or a clock ramp.

Which glibc knob, measured one at a time in fresh processes:

| | P1.2 upb encode |
|---|---|
| nothing | 349,589 ns |
| `mallopt(M_MMAP_THRESHOLD, 8 MiB)` | 347,019 ns |
| `mallopt(M_TRIM_THRESHOLD, 8 MiB)` | 296,498 ns |
| both | 195,281 ns |
| `mallopt(M_TOP_PAD, 8 MiB)` | 193,797 ns |

So it is not that the buffer is mmap'd -- raising the mmap threshold alone recovers
nothing. It is that glibc hands the buffer back to the OS when it is freed, by trimming
the top of the heap, and the next call faults it in again; `M_TOP_PAD` keeps enough slack
at the top that the trim never happens. Once a process has allocated and freed something
*larger*, glibc raises its own thresholds and the same thing happens by accident.

That is the whole story. Work unit 2's bench carried M1 only, so nothing in it ever
allocated past 218 KiB and the incumbent ran cold for the entire run. Work unit 3's bench
carries M2, `harness.calibrate` touches every case before the first round, and P2.4's
979 KiB output warms the allocator for everything after it.

**What it costs.** `allocator.py` is now the experiment, run as step 4 and logged to
`logs/python/55-allocator.log`. Cold against warm, in the same order, only the two arms
that matter:

| | payload | cold | warm | cold/warm |
|---|---|---|---|---|
| P1.2 encode, upb | 218 KiB | 389 us | 202 us | **1.92** |
| P1.2 encode, core-ffi / C ext | 218 KiB | 397 us | 258 us | **1.54** |
| P2.4 encode, upb | 979 KiB | 1,317 us | 345 us | **3.81** |
| P2.4 encode, core-ffi / C ext | 979 KiB | 1,738 us | 957 us | **1.82** |

Flagged only where the two spreads do not touch, so a 2 us row's jitter cannot qualify;
the four rows above are the only four that do, out of 32.

Everything else -- every payload under 20 KiB, and every decode at every size -- is inside
the noise. P2.2 and P2.3, at 540 and 647 KiB, are clean too, because by the time they run
P1.2 has already raised glibc's thresholds past them. That is the point: the figure is a
property of **what ran before it**, not of the codec.

**What I did about it.** `bench.py` now calls `mallopt(M_TOP_PAD, 8 MiB)` before it
imports `arms`, so every arm in every run is measured warm, and the header says so. Warm
because it is the state a long-lived gRPC server is actually in, and because it is the
state that helps the incumbent more than it helps us (1.92 against 1.54 on P1.2, 3.81
against 1.82 on P2.4) -- so it is the conservative choice as well as the realistic one.

**What it invalidates.** Work unit 2's headline, "the composed arm encodes P1.2 at 0.68 to
0.72 of the incumbent", was an artefact of a bench that only ever allocated 218 KiB.
Warm, it is ~1.26. `logs/python/60-composed-py3.11.log` and `61-composed-all.log` are kept
because they are the record, and this entry is why their P1.2 encode rows do not agree
with the current ones. The M1 *decode* figures in them are unaffected.

And a general one, for the other slices: **an incumbent that allocates one big output
buffer per call and frees it is measuring the allocator, not the serialiser.** Every slice
here has a large-payload encode arm. None of them, this one included, had checked.

### J27. M3 to M7, and a script that had stopped running

The scope widened one shape at a time and the walker's raise drove the order, which is
what `walk.py` is for. What each one cost:

| | shape | what it needed |
|---|---|---|
| **M4** | the plain adapter site | **nothing but the scope line.** `TaskSummary` is `TaskOptions` and a `Timestamp` over shapes M1 and M2 already carry, and the adapter site is a property of the payload's VALUES rather than of any field shape |
| **M7** | two repeated fields, interleaved | a run list per ROOT repeated field, and the permutation check |
| **M5** | bulk `bytes`, 36 B to 4 MB | a root with NO repeated field, and `V.bulk` threaded through the builder |
| **M3** | a oneof and explicit presence | a new cardinality in every backend, both directions |
| **M6** | packed scalars, and `double` | `ak_run_i32/i64/f64/u8`, and a packed run on the decode side |

**Two cardinalities rather than two flags.** `walk()` now yields a oneof member with
`c == "oneof"` and an `optional` scalar with `c == "optional"`, and nothing else changed
about how a backend consumes it. The alternative -- yield `m.plain` the way the IR does
and hand the oneof to the backends through a second accessor -- was rejected because a
backend that forgot to call the second accessor would emit a complete-looking codec that
silently dropped the oneof, which is the defect the walker exists because of. A
cardinality nothing has a case for is a GENERATION-time failure, and that is how every
one of the five emitters got its case: the generator refused to run until it had one.

The same reasoning applies to `optional`. An `optional int32` that goes through the
implicit-presence guard encodes absent and present-and-zero identically, and every payload
but the absent one passes. `payloads.py` puts one element in seven at present-and-zero
precisely so that shortcut fails loudly.

**What the by-value group already had.** Nothing in the ABI needed extending: `ak_efix_Probe`
carries `body_case` (the active member's TAG, 0 for unset) and a `presence` word with one
bit per explicit-presence field, and `ak_run_i32/i64/f64/u8` were already there for the
packed runs. So M3 and M6 are a binding exercise in Python and not an ABI question, which
is worth saying plainly because the branch has been treating oneof and explicit presence
as open.

**A root is not "one repeated field plus page and total".** That sentence was written into
four places -- the decode entry's single `h->list`, the like-for-like reader, the
conformance comparison and the bench's decode floor -- and M5 (no repeated field) and M7
(two) broke all four. The decode entry now carries one run list per root repeated field,
the reader walks a plan built for the ROOT rather than for an element, and the conformance
comparison is `_cmp_msg(root, root)` with no element loop at all, because `_cmp_msg`
already recursed through a repeated message field. The generalisations are all shorter
than what they replaced.

**And `corpus.py` had stopped running.** Work unit 2's binding took `decode(backend, buf,
types)`; the M2 commit gave it a root argument and renamed `arms.CEXT` to `arms.TY_CEXT`,
and `corpus.py` was not updated. Nothing noticed, because `run.sh` never ran it -- the
corpus pass was run by hand once, its log committed, and the log then outlived the code
that produced it. It runs as step 4 of `run.sh` now, which is the actual fix; the code
change is only what that exposed.

Its rule-0 check had the same shape of defect. It demanded that EVERY message in scope be
extended by `corpus_superset.proto`, which held while the scope was M1 -- all three of its
messages happen to gain `u_*` fields -- and produced eleven spurious failures the moment
the scope reached messages the superset does not extend. The obligation is that every
scoped message matches `corpus.proto` and that at least one is extended, so the
unknown-field skip is executed somewhere. A check that only holds for the scope it was
written against is not a check.

### J28. D11, and a first fix that was worse than the defect

The RPC arm's in-process control disagreed with `bench.py` on the same payload in the same
interpreter. The only thing between them was `gc.disable()`, which work unit 1's harness
does for the measured rounds -- right for a microbenchmark of a C-API primitive, and not
obviously right for a codec arm, because a facade decode of P2.2 builds on the order of
ten thousand GC-tracked objects and `FromString` builds an arena and one wrapper. The
collector's work is the facade's work.

**Measured in isolation, the discount is real and bounded** (`logs/python/57-gc-bias.log`,
GC off against GC on, same process, interleaved per arm):

| | on/off |
|---|---|
| every encode row, both arms, every payload | 0.99 - 1.02 |
| upb's decode, every payload | 0.97 - 1.02 |
| **the facade's decode** | **1.09 - 1.26**, P2.2 worst |

Four rows flagged, all of them the facade's decode. So the collector was subtracting from
one arm of one column and from nothing else.

**Then I enabled it in the bench, and that was wrong.** P2.2's decode came out at
**7.3x** the incumbent, against 1.26x for the identical call measured on its own. A
factor of six had to come from somewhere, and the obvious suspect -- the harness holding
all sixteen payloads' fixtures alive, so every collection walks a huge heap -- is
refuted: holding all sixteen alive on purpose makes the isolated figure slightly *faster*,
2.64 ms against 3.04 ms.

What is left is attribution. With the collector on, an interleaved run of about five
hundred cases fires it wherever the allocation threshold happens to trip, and the cost
lands on whichever case was running. **That is a figure that depends on what else is in
the run**, which is exactly the defect `allocator.py` exists for, one layer up, and this
bench already refuses that class.

So the collector stays OFF where a ratio is formed, and `gcbias.py` prices it where it can
be attributed: one payload, one direction, one process, nothing interleaved. The honest
sentence needs both halves measured:

> the ratios exclude the collector, which adds 1.09 to 1.26 to the facade's decode and
> nothing to any other row

**What this cost and what it is worth.** I published "this invalidates the decode columns"
and it did not: the GC-off tables were right all along, and the correction was to add a
number beside them rather than to replace them. The logs restored here are the ones taken
before the wrong fix, from the same tree and the same core. The reason to write this down
is that the sequence -- a control contradicts the bench, the bench is changed to match the
control, the change is worse than the thing it fixed -- is a plausible way to make a slice
worse while believing it is being made honest. The control was right about the cost and
wrong about where to charge it.

## Work unit 4: the 2026-09-24 review (FIX-PLAN WP4 and WP6)

Phase change first, because it changes what a result is here: the branch is in its setup
and design phase (README 1.1). A container timing is instrumentation. Nothing in this work
unit was timed; every entry below is settled by a run that checks behaviour.

**How this slice now builds while other agents edit the shared tree.** The rust agent is
changing `poc/codec` and the cpp agent `poc/cpp/gen/cpp_header.py` (which `gen/generate.py`
imports) at the same time. The first conformance run this session reported `R1: STALE,
regenerate: ak_abi.h` -- not a defect here, but the cpp agent's uncommitted R-D2 edit
showing through the read-only import. So `build.sh` and `gen/generate.py` take
`AK_UPSTREAM`, an `ffi/` tree to read the shared inputs from, and this session pointed it at
a `git archive 8864e4d ffi/poc/codec ffi/poc/cpp/gen ffi/schema` extraction. The core is
still the one core (never a fork, nothing writes to it); the build names a commit instead
of somebody's working tree. Cargo targets moved into `build/cargo/{plain,count,rpc}`, so
this slice no longer writes into `poc/codec/target*` at all.

### J29. R-D3: the RPC arm was ungated, and a failed RPC was timed as a cheap success

Confirmed, every sub-claim, first by reading and then by making the RPC fail on purpose
(`rpc_gate.py`, `logs/python/81-rpc-gate-before.log`):

| sub-claim | confirmed by |
|---|---|
| queue delivery decodes `c[2]` without looking at `c[1]` | `status` injection: queue cells returned a message with **0 tasks** |
| callback delivery appends `body` and drops `status` | same: callback cells returned 0 tasks |
| a failed completion is `b""` via `take_bytes`, and `b""` decodes | the core completes failures with `empty_ak_bytes()` (rpc.rs); `FromString(b"")` and the facade decode of `b""` both succeed |
| `measure_fn` stops a thread at its first exception and still divides by the full count | blocking cells: "errors counted 4" at 4 in flight, figure still produced |
| `_akffi_rpc` absent from `build.sh`'s R5 loop | read; the loop named two of the three shims |
| `conformance.py` never loads `_akffi_rpc` | read: `AK_FFI_MODULE` defaults to `_akffi` and `run.sh` never set it |

Before the fix, **68 of the 80 failure-injected timed rows produced a figure**. Only the
blocking delivery ever raised, and even there the harness counted the error and published
a figure divided by calls that never happened. Two things the review did not name:

- the `empty` and `short` injections (status OK, wrong body) fooled **every** cell,
  grpcio's cell A included. Checking status alone would not have closed this; the length
  check is what does;
- cell A's timed loop "aborted" under `status` and `closed` only because its warm-up call
  sat outside the `try`. A failure that began mid-run would have been counted and divided
  like the others. That it looked gated was an accident.

Fixed (`logs/python/83-rpc-gate.log`): every call in every cell checks status 0 and the
payload's exact length and raises `RpcFailed` otherwise (cell A through a gated
`response_deserializer`); the first exception in any thread aborts the measurement, which
prints `ABORTED, no figure` and makes `rpc.py` exit non-zero; each block runs
`gate_cells` before timing, which compares every delivery's bytes to P2.2 exactly and
re-encodes every decode. The binding now hands a failed completion's body over as
`None`, not `b""`, so a caller that forgets the status cannot decode a failure into an
empty message. After: **80 of 80 failure-injected rows aborted, 20 of 20 healthy rows
gated**, and the queue and callback deliveries return `status -1, body None` at the
binding.

`_akffi_rpc` is in the R5 loop, plus a check that it imports all six section 9 entry
points it binds and that its `libak_core.so` resolves to the rpc build, with a must-fail
control (the same source without `-DAK_RPC` imports 0 of 6 and is refused;
`logs/python/87-build-py3.12.log`). Conformance runs against it: **ALL CHECKS PASS**,
and it passed before the binding change too (`84-...-before.log`), so the rpc shim's codec
was right and simply unchecked. `conformance.py` now prints which shim it gated and which
`libak_core.so` the process mapped, because "passes" said nothing about which build had
passed until now (`85-conformance-rpc-shim.log`).

`rpc.py` smoke-ran end to end, gated, exit 0 (`86-rpc-smoke-gated.log`, every timing row
deleted from the log on purpose).

### J30. R-D4: the 3.7 floor, confirmed by reading and not fixed

`logs/python/82-floor-3.7.log`. `gen/out/binding.c` calls `Py_NewRef` (3.10) 164 times,
`PyObject_CallNoArgs` (3.9) 114 and `PyObject_CallOneArg` (3.9) 133, all emitted by
`gen/py_binding.py`, with no `PY_VERSION_HEX` anywhere. `gen/out/_akcodec_gen.c` (work unit
1's no-core shim) calls `PyObject_CallOneArg` 16 times. The PyO3 arm's abi3 feature is
`abi3-py310`. **One more than the review found**: the hand-written `native/binding.c` uses
`Py_NewRef` once and `PyModule_AddObjectRef` (3.10) once; it is not generated, so WP5's
conditionals will not reach it and it needs its own edit.

Not fixed, by instruction: WP5 moves the shim generator into `poc/codec/gen` and emits the
version conditionals there. And 3.7 is not obtainable in this container: the apt index
lists `3.7.17-1+noble2`, but the egress proxy refuses the deadsnakes PPA (403), python.org
and github, so neither the package nor a source tarball nor a standalone build can be
fetched.

### J31. R-F3 and R-F2: two STATE.md statements that no log supported

**R-F3, confirmed.** STATE.md's P2.2 row gave "10.02 / 7.00" in the shim -> CPython
column and "32" for the Python storages; `53-conformance-all-shapes.log` says 51.67 / 51.67
and 146.68 / 131.35. Both are right about different edges. 10.02 is **core** fwd 5.02 +
core rev 5.00 per `TaskDetailed` on encode and 7.00 is core rev on decode -- the number J22
matched against the rust slice -- and it had been copied into the shim column of two
tables. "32" matches nothing in any log (the nearest, `C-API value read=32170`, is a raw
count over the payload, not per element) and is deleted. The crossing table is now
regenerated from `85` (3.12), which is identical row for row to `53`'s 3.11 block, with
the three edges named and a rule that they are never summed.

**R-F2, confirmed.** STATE.md said in its header that the concurrency suite exists and
passes, and in "what is not measured" that concurrency is unanswered and 12.5 has no python
row. The second was a leftover from before work unit 3; deleted. `57-gc-bias.log`'s closing
line says `bench.py` no longer disables GC, but `bench.py` goes through `mech/harness.run`
with its default `gc_enabled=False`: the line was printed during J28's withdrawn first fix
and outlived it. `gcbias.py` now prints what the code does; the committed log is left as it
was produced and STATE.md says the line is stale.

**While checking both, a third statement with no log: U1.** The upb map-entry defect was
"isolated on a two-field message" by hand and never committed. `u1_map_unknown.py`
reproduces it on 3.12 / protobuf 7.36.2 (`88-u1-map-unknown.log`): upb returns `{}`, the
python backend, the core and the pure-Python control return `{'k': 'v'}`.

STATE.md was rewritten to the phase rule at the same time: no timing figure in it, timing
logs listed as instrumentation, no recommendation (the queue-as-default and the
outcome-2 sentences are gone), target 3.12 per owner decision D1 rather than 3.11.

### J32. Re-gate against the shared core at 6ede244 (R-D1 fixed)

Built from `git archive 6ede244` through `AK_UPSTREAM` (the cpp agent still had
`poc/cpp/gen` uncommitted). The build's core-tree hash and the library size both moved
(5ac4151c -> 8a5fbb83, 806,680 -> 808,520 bytes), which is the check that the new core is
the one in the build and not the old one surviving in a target directory.

**R-D1 through this slice** (`rd1_lenwrap.py`, `logs/python/89-rd1-lenwrap.log`). Three
wrapped inputs, each decoded in its own subprocess under a 10 s timeout. Against the OLD
core (8864e4d, forced over the shims' RUNPATH with `LD_LIBRARY_PATH`, the mapped core
printed): the finding's 11 bytes **hang**, a root-level wrapped length **aborts the Python
process** on a non-unwinding Rust panic, and a nested wrapped string length **segfaults the
Python process** -- the host being handed the bad span, which is the review's "4 GiB
out-of-bounds span to host" happening in a real host. Against 6ede244: all six rows return
`AK_ERR_TRUNCATED` promptly, on both shims; the P1.2 control decodes.

**Everything else holds**: conformance passes on both shims; crossing counts identical row
for row to 8864e4d (the fix moved no count); the RPC gate gives 80 of 80 aborted and 20 of
20 gated again.

**The corpus grew under this slice's feet and found two things.** 213 of 691 rows are now in
scope (was 126 of 336). Both core-ffi arms pass every obligation, the 42 in-scope
length-wrap vectors included. The pure-Python control fails 42 rows, exactly R-E5's three
classes: 27 `U-wire-*` (it reads a known field at whatever wire type arrives), 7 `S-neg-*`
(negative int32/int64 projected as unsigned 64-bit), 8 `X-tag-zero-*` (tag 0 accepted).
Not fixed: those are decode rules, and WP5 moves them into the shared generator; writing
them into `gen/py_codec.py` now is the defect WP5 removes.

The second thing is D12, in `corpus.py` itself: the first C1 failure ended the run with
`NameError: name 'UPSTREAM' is not defined`. Commit 7e0404a emptied the upstream-defect
table by deleting it and left the three places that read it. It could not show while every
row passed, which is the same shape as J27's script that had stopped running: code on a path
nothing exercised. Restored as an empty table, and the per-arm failure list is no longer
cut at eight, so the log carries every row a reader has to classify.
