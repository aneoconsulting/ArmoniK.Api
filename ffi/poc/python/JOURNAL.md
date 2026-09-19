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
