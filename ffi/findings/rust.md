# Reading the rust slice

The aggregating session's reading of `poc/rust`, which is not the same document
as the slice's own `STATE.md`. What is here is what the slice's results mean for
the branch: what is now established, what the other four slices have to do
differently because of it, and what is still an argument.

**W3 is done.** Four arms over every message and every payload of `SHAPES.md`,
all byte-identical to the validated manifest, plus the three content sets, the
unknown-field vectors and the RPC arm. The manifest was re-validated against prost
after this session's adapter fix (16 of 16) and M2 and M4 were re-measured rather
than assumed: the mechanisms held to the digit (10.024 crossings per task on
encode, 7.004 on decode, decision 5 unchanged), most ratios did not move, and the
new runs are tighter, so they supersede.

**What that re-run also demonstrated, which nothing had yet**: the three facade
arms matched the new hashes as soon as the generator was re-run, and the `prost`
arm did not, because its objects come from a separately hand-written builder. Two
construction routes were kept apart for exactly that reason and this is the first
time the separation fired.

## SUSPENDED: the ratio tables below do not reproduce, and the re-run is pending

**Read this before quoting any number in this document.** The unknown-field arm's
first timing run looked like a large regression; it was not. Rebuilding **the same
commit** in a fresh worktree on the same machine, the same session and the same
day reproduces neither the published ratios nor their spread:

| P1.2 encode, / prost | published | HEAD `4afffd9b` | `7fb30be5` |
|---|---|---|---|
| `armonik` | 0.967 – 1.032 | 1.150 | 1.146 |
| `core-native` | 0.425 – 0.438 | 0.564 | 0.544 |
| `core-ffi-rust` | 0.706 – 0.716 | 1.011 | 0.992 |
| P1.3 decode `core-ffi-rust` | 1.313 – 1.393 | 1.725 | — |

**Every arm moved the same way, `armonik` included** — and `armonik` is prost's
own codec over the facade types, which nothing in this slice has touched since.
The drift is present at a commit predating both the zeroed-group work and the
unknown-field arm, so neither caused it. The container is shared, unpinned and
was measured on a different day; that is the likeliest cause and it is **not
established**.

**What this suspends, and it is not small.** `core-ffi-rust` on P1.2 encode is the
difference between 0.71 (a 30 percent win over prost) and 1.01 (parity). The
headline that the core is *faster* than prost through the C ABI on encode rests on
the published column, and the published column does not currently reproduce. Until
a controlled re-run says which column is right:

- **Treat every absolute ratio in this document as provisional**, including the
  encode win, the decode parity and the P1.3 inversion's magnitude.
- **What survives is what was measured as a delta inside one process**, between
  arms in the same interleaved rounds with prost as the control in that same
  process: the inlining audit's three-term split, the zeroed-group deltas, the
  unknown-bag deltas, the crossing counts (which are counts, not times), and the
  byte-identity results. Those are differences taken in one build and do not depend
  on the absolutes.
- **The large-effect qualitative findings are likely but not confirmed**: the P1.3
  inversion's *sign*, UTF-8 validation at 2.0–2.6, decode converging to parity as
  containers grow. Each needs re-confirming rather than assuming.

**What R4 actually says, and what this sharpens it to.** R4 already held that
absolutes do not travel between runs on shared hardware and that ratios within one
process do. This says the second half is too generous: a ratio formed in one
process is reproducible *within that build*, and **not necessarily across builds
of the same source** on this container. Every comparison must therefore be taken
in one build and one session, and a table assembled from two is not a table.

A re-run of the M1 and M2 tables under those conditions is the outstanding item.

## Configuration, once, for everything below

4 vCPU Intel Xeon at 2.80 GHz, 15 GB, Ubuntu 24.04.4, Linux 6.18.44 x86_64, in a
container, no pinning and no governor control. rustc 1.94.1, release. prost
0.14.4, prost-build 0.14.4, protox 0.9.1. **Linkage: the core is a `cdylib`
resolved by the dynamic linker**, which section "The defect" below says is not a
detail. Accessor guard on. ASCII content set only. MSRV 1.88 is declared and
**not verified**: no 1.88 toolchain exists in the container.

Ratios are formed inside one process from interleaved rounds; three processes
were run and what is quoted is the range across them.

## The RPC half: two crossings per call, zero per field

The number no other slice can get from its own measurements, and the one the
"adopt the RPC layer, generate the codec" fallback rests on.

**It is a property of the code rather than a measurement**, which is the stronger
form: neither the slice's RPC crate nor the core's RPC module mentions a message
type anywhere, verified here by reading both. The half dispatches on a path string
and moves opaque bytes, so there is no place a per-field cost could enter. If the
count were a function of field count, one of those files would have to know about
fields.

**The transferable form is the arithmetic, not the ratio.** Two crossings of 1.8
ns against a call costing about 1.5 ms of CPU is roughly four parts in a million.
A host paying 98 ns per crossing through JNI pays 196 ns on the same call: 0.013
percent. End to end the Rust arm is 0.91 to 1.10 of tonic at 1, 8 and 16 in
flight, which on four shared vCPUs is no measurable difference rather than a win,
and is the less useful half of the result.

**A hazard every slice's RPC arm will hit**, now rule R9: P2.2 is 540 KB and the
default HTTP/2 stream window is 64 KB, so a single call in flight spends most of
its wall-clock idle waiting for `WINDOW_UPDATE`. The slice's first version would
have reported 33 ms per call for a path costing 1.5 ms of CPU. Measure CPU.

**The concurrency defect, and what it says about obligation 12.5.** The client
handle was built taking a mutable reference, so two host threads mutated shared
state. It worked at 1 call in flight and failed outright at 8 — the good failure
mode, since the bad one is a wrong byte under contention. It was found *by
accident*, because stage 4 asked for 8 in flight. The conformance obligation that
exists to catch exactly this class still has no implementation in any slice, and
this is the best argument for it so far.

**And the error channel discarded the cause.** The failure surfaced as
`AK_ERR_HOST` with nothing attached, and the slice's code was correct against ABI
v1 as written: the specification asks for a code and a message and gives no
channel for a source chain. An error channel that discards the error is not an
error channel. That is now part of open decision 12 rather than a slice's bug.

## What is now established

**The manifest is an oracle.** Every payload but P7.1 is byte-identical to prost
and to a second encoder that shares no code with it; P7.1 cannot be produced by
any canonical writer and is validated by decode. A slice that disagrees with a
hash now has a defect in itself. This is what W2 was waiting for, and it is the
single most reusable thing the slice has produced.

**A crossing costs 1.8 ns in Rust through a shared library**, forward and
forward-plus-reverse alike, stable to 0.1 ns across three runs and measured in the
same process and the same build as the arms. That is the number section 4.1 of the
README exists to obtain: every other slice's result splits into this plus its own
runtime's tax.

**Nine crossings encode a thousand rows, six decode them**, counted rather than
inferred. The drafted ABI spent 15,137 on the same shape. The amended ABI's
central claim survives contact with a second language.

**The accessor guard is free here**, inside the run-to-run spread. The mechanism
is the one ABI v1 section 5 predicts rather than anything Rust-specific: with
strings riding in the group, the guard lands on one to five reverse calls per
message instead of one per string. The section 5 worry was about the *number* of
accessors, and the amendment has already removed most of them.

## The defect every other slice has to protect against

With the core in the crate graph as an rlib, rustc inlined every `extern "C"`
entry point into the host: the release binary contained **zero** call sites to
the encode, decode and element entry points. The FFI arm was the no-boundary
control with extra struct copies, and every ratio from it would have been a
figure about the optimiser.

**The counting build did not catch it. It reported 3, 9 and 6 crossings** as
usual, because the counting code was inlined along with the function bodies.

That is the finding: **a counter is not evidence that a call happened**, and R5
as written ("count the crossings, do not infer them") is necessary and not
sufficient. R5 now also requires a slice to show from the built artifact that the
entry points are unresolved imports, as a build step rather than a claim. Checked
independently here: `nm -D --undefined-only` on the harness binary lists the
thirteen ABI entry points as undefined.

Who is exposed: **Rust and C++**, the two that can compile host and core
together. In C++ it is spelled `-flto` over a statically linked core, and a
non-LTO build and an LTO build of the same sources will disagree. The managed
hosts cannot inline across the boundary and are safe from this one.

The inverse matters too and is now in R7: **1.8 ns is a shared-library crossing.**
A C++ host that statically links gets a direct call and pays less. A C++ column
and this one are not measuring the same mechanism unless both say which.

## The string path: validation is the largest effect in the branch

The ASCII pass priced UTF-8 validation at 25 to 30 percent of an encode. **That
was not the cost.** `core::str::from_utf8` consumes a `usize` at a time on ASCII
and one byte at a time otherwise, so its cost tracks *non-ASCII bytes*, not bytes.
On the two non-ASCII content sets the scalar validator costs 2.2 to 3.0 times its
own ASCII cost, and turns a 0.72 to 0.81 win against prost into a **2.0 to 2.6
loss**. It is the largest single effect measured anywhere in this branch, and an
ASCII-only pass cannot see any of it. This is why SHAPES.md's rule that a
string-path number without a content set is half a number is a rule and not a
formality.

**Settled, and the answer is that the old arrangement was strictly dominated.**
Everything below stands as measurement; none of it stands as a reason to validate
on encode. The encoder does not need a `string`'s bytes to be valid; the decoder
cannot trust them whatever the encoder did; proto3 puts the obligation on parsers.
So encode became a memcpy, and **decode became rejecting at no cost at all**:
validate-and-reject measures 0.54 to 0.75 of the lossy conversion it replaces on
ASCII, and 0.36 to 0.71 with the SIMD validator on every content set, because a
lossy conversion already validates and its recovery path is slower than failing.

It also **improves** the comparison against prost rather than costing it, since
prost rejects too: P1.2 ASCII moves from 0.87-0.91 to 0.73-0.79, and P1.2 wide from
0.78-0.80 to 0.49-0.51. The decode column this slice has been quoting was
pessimistic, not flattering.

So the design the slice started with paid for a slower validator to get a weaker
guarantee, on both sides of the boundary at once, and the fix is cheaper in both
directions. That is the cleanest result in the slice and it came from a question
about the specification rather than from a benchmark.

**Two defects the re-measurement produced, both worth the rule they carry.** The
decode entry point returned the sticky error slot and nothing ever cleared it, so
one rejected decode poisoned every later decode in that context — now a stated
requirement in ABI v1 section 5, because "the first error wins" is incomplete
without saying when the slate is wiped. And the first driver ran the policies in a
fixed order, where the always-first build read 0.778 in one invocation and 0.88 in
the next; round robin with a rotating order fixed it. An ordering artifact that
large would have been invisible in any single run.

**Superseded, kept for the record: the question was on the wrong side of the
boundary.** Everything
below stands as measurement and none of it stands as a reason to validate on
encode. The encoder does not need a `string`'s bytes to be valid, since it writes
a length and copies; the decoder cannot trust them whatever the encoder did; and
proto3 requires parsers to validate. So the encode-side check is redundant with
one that has to happen anyway, `ak_tc_utf8` collapses into `ak_tc_bytes`, and the
whole 2.0-to-2.6 penalty below is a cost paid for nothing. ABI v1 decision 3
carries the rewritten form. **The slice's own arrangement is the inversion in
miniature**: it validates on encode at 25 to 30 percent of ASCII cost, and decodes
through `String::from_utf8_lossy` at 37 sites while rejecting at none, so the only
place protobuf actually requires a check is the place it silently substitutes.
Nobody chose that; it is what a facade written the obvious way does.

**The slice answered it with an arm rather than an argument**, which is the right
instinct and worth recording as such. `ak_tc_utf8_simd` has the identical
contract, verified here in the source: the same refusal of malformed input, the
same grow and capacity handling, `simdutf8::basic` in place of the scalar DFA. It
recovers half to two thirds of the penalty (1.27 to 1.50 of prost) and is not
faster on ASCII, because the scalar ASCII path is already eight bytes an
iteration. The accept and reject sets being identical is what makes it admissible
here: a validator that differed on any input would make the core's bytes depend on
which one a build chose.

**What it did to the ruling on trusting the host: nothing, and that ruling still
stands on its own terms.** A *trusted* transcoder makes validity a contract a host
can be wrong about, picked per host type, and that is refused. The later proposal
to drop encode-side validation outright is a different thing and the objection
does not reach it: it extends trust to nobody, no generator chooses per type,
every host gets the same passthrough, and the parser checks. There is no contract,
so there is nothing to be wrong about.

**And the measurement for it already exists**: `ak_tc_utf8_trusted` is that
proposal, so 0.59 to 0.72 on M1 ASCII, 0.80 to 0.85 on M2 and 0.75 on both
non-ASCII sets are the figures, with no new run needed. The SIMD validator is not
wasted by this, but it moves: validation on decode is mandatory, and that is where
a fast validator earns its place.

**What it does not settle**, and the slice says so itself: one x86-64 machine with
AVX2; runtime CPU dispatch with a fallback, which makes it a floor question in C++
as much as in Rust; and a dependency inside the core rather than in a binding,
which is an unpriced packaging cost.

**Decode is unaffected**, and that is worth stating rather than passing over.
Every arm validates on decode, so all three sets cost every arm 1.2 to 1.7 times
its ASCII self and no ordering moves. That is exactly what ABI v1's asymmetry
predicts, the core transcoding on encode and the host on decode, and it also holds
the allocation-bound decode result up under a second content set.

**One thing this slice cannot reach at all.** A Rust `String` cannot hold an
unpaired surrogate, so the transcode pair of README section 10 item 4 is
*unreachable* here rather than unbuilt. The corpus has to carry those vectors as
raw bytes produced by a non-Rust host, and C# and Java are the slices that have to
run that pair. Correctly logged as unreachable rather than quietly omitted.

## The adapter was unreachable from the payload set, at both sites

M4 exists for one reason: one facade type, two wire forms, a map that is not
injective, and a defect only a byte corpus catches. **The payload set could not
reach that shape at either site**, and the slice found it by checking the adapter
by state rather than by running the payload, which is what I had asked for and is
the only way it surfaces.

At the nested site, `emit/payloads.py` filled `success` and `error`
independently. Over 200 elements the only combinations occurring were
(true, non-empty) and (false, non-empty). **(true, non-empty) is a state
`TaskDetailed.Output`'s own comment forbids** ("the error message, only set if
task have failed") and that no adapter over {Ok, Error(details)} can represent;
the success state (true, empty) never occurred at all.

At the plain site the finding is sharper and is a property of the schema rather
than of the generator: **Ok and Invalid both flatten to the empty string, so one
of them must come back wrong whatever the adapter author chooses.** Return Invalid
and lose Ok; return Ok and silently claim success for a task that reported no
outcome. The nested form round-trips all three states. That is the concrete defect
SHAPES.md is abstract about, and no payload reached either state.

Fixed in the schema directory, which this session owns: the generator now cycles
the three states per element. Verified independently off the wire bytes: 167 Ok,
167 Error, 166 absent at P2.2's nested site, zero occurrences of the impossible
state, and P4.1's plain site carrying the collision at 67 non-empty against 133
empty-or-absent. It moved P2.1 to P2.4 and P4.1.

**Worth noting about the real schema**, which the slice's work surfaced: both wire
forms are real and both are called `Output`. `TaskDetailed.Output` is
`{bool success, string error}`; `objects.proto`'s `Output` is a
`oneof {Empty ok, Error error}`. The facade unifies two messages that a reader of
either `.proto` alone would not connect.

## A control turned a twelvefold win into a bounded claim

P5.4 decode came out at 0.08 of prost. The slice did not report it; it added a raw
4 MB copy as a floor arm, and found every core arm sitting on that floor
(`core-native` 0.084, `core-ffi-rust` 0.080, raw `copy_from_slice` 0.082, raw
`to_vec` 0.080).

So the claim is **"a 4 MB bulk decode costs one copy in the core and twelve in
prost"**, which bounds both sides, rather than "the core is twelve times faster
than prost", which bounds neither. Why prost sits twelve times above the floor is
recorded as a labelled suspicion and not chased, because it is a finding about
prost rather than about the ABI.

This is now rule R2's second half. It also cuts against the branch's own case in
one place: ABI v1 section 8's direct-argument path is justified by a JVM figure of
0.16 to 0.34, and since the Rust *no-boundary* control is already on the memcpy
floor, the Rust slice can say nothing in support of it.

**A second harness defect in the same pass would have been published as a
finding.** `core-native` on P5.4 encode measured 1.412 of prost because the arm
allocated a fresh `Vec` per call and grew it by doubling from 4 KB to 4 MB, while
every other arm reused a warm buffer and prost's `encode_to_vec` sizes once. A
growth-policy comparison wearing a codec's name; 1.412 became 1.018 when fixed.
Caught only because a 4 MB payload made it large enough to disbelieve, and at M1
sizes it would have looked like a plausible regression. That is the same lesson as
the inlining defect from a different direction: the arm you are proud of and the
arm you distrust both need a floor.

## Unknown fields: the branch's largest unpriced behaviour change

The slice set out to cover a shape and turned up a migration question instead.

**At the wire level there is no such thing as an unknown oneof member.** The
grouping exists only in the descriptor, so a parser cannot tell an unrecognised
oneof tag from any other unknown field: the case stays at the last known member
and the payload is dropped. Seven hand-built vectors, all four arms agreeing on
the decoded value *and* the re-encoded bytes. The contrast the slice draws is the
useful part: an unknown *enum value* round-trips losslessly, because the field is
known and only the value is not, while an unknown *field* cannot round-trip at all
in any arm. SHAPES.md had these as one row and now has two.

**What the slice did not draw, and this session did.** "Nothing retains unknown
fields" is not a neutral property of the design, because four of the five
languages retain them today. proto3 has preserved unknown fields since protobuf
3.5, so `Google.Protobuf`, protobuf-java, protobuf C++ and upb all carry an
unrecognised field from decode through to re-encode. prost does not, and the core
follows prost. **So adopting the core removes a protobuf guarantee from every
language except the one whose incumbent already lacked it** — and Rust, the
language that loses nothing here, is the one the slice was written in, which is
exactly how a divergence like this stays invisible.

It is now ABI v1 open decision 11. Two things about it worth keeping:

- **Who it bites is specific.** A client that decodes and never re-encodes loses
  nothing. A proxy, a worker forwarding a `ProcessRequest`, anything round-tripping
  between two schema versions loses the field silently. That is the same seam as
  conformance obligation 12.4, seen from the other side.
- **Retention is not obviously cheap.** Keeping unknown bytes means storing them
  somewhere the host can hold, which is a host-visible allocation on a path this
  design works to keep allocation-free. Nobody has priced it.

This is the kind of finding the branch exists for: not a number, and not
discoverable by a benchmark. A shape-coverage vector found it.

## The audit: the inlining objection, and why the decomposition survived it

The per-element interface costs quoted in this document were obtained by
subtracting `core-native` from `core-ffi-rust`. The objection, raised against it
rather than by it: `core-native` is compiled into the harness, so rustc could fuse
the traversal into the benchmark loop while the FFI arm cannot be inlined at all,
and the subtraction would then charge an inlining advantage to the interface.

**The premise turned out to be false for these binaries, and the artifact says so.
Verified here independently rather than taken from the slice**, because the
finding conveniently exonerates a claim this document had already made:

- `encode_into_list_results_response` is **20 bytes** (`0x14`) — two stores and a
  tail `jmp`. The traversal it jumps to, `enc_list_results_response`, is **4,299
  bytes** (`0x10cb`); the decode traversal is **11,311 bytes** (`0x2c2f`).
- The largest `bench::main::{{closure}}` is **472 bytes** (`0x1d8`). A 472-byte
  closure cannot contain an 11,311-byte traversal.
- The call sites inside those closures are `call *0x…(%rip)` — **indirect calls
  through the GOT**, the same shape as the call into the cdylib.

The mechanism, stated narrowly because the broad version of it is false: the
profile carries no `[profile.release]` section, so cargo's default applies and
**LTO is off**; and the two entry points the benchmark actually calls are
**non-generic `pub fn` with no `#[inline]`**, so their MIR is not exported and
their bodies cannot cross into `harness` at all. That is the load-bearing fact.

It is *not* true that nothing in `facade` can cross. `core_native.rs` carries 38
`#[inline]` functions, `enc_list_results_response` among them, and an `#[inline]`
function's MIR **is** exported cross-crate with LTO off. That traversal was not
inlined on cost grounds — 4,299 bytes into a 472-byte closure — and the benchmark
never calls it directly anyway; it calls the 20-byte entry thunk, which is the one
that genuinely cannot cross. (The symbol table shows this from the other side:
the traversal is a *local* symbol, instantiated into the binary, while the entry
point is a global from the rlib.)

**The distinction names a failure mode rather than splitting hairs.** If the
generator ever put `#[inline]` on a per-message entry point, or made one generic,
the objection this audit refutes would become true again **with LTO still off**,
and the published ratios would quietly begin carrying an inlining advantage.
`gen/inline_check.sh` catches it — the closure would grow past the traversal —
which is why that check is a script run every build rather than a paragraph in a
log. So both arms were already paying an indirect call, and there was no inlining
advantage to subtract.

Two arms confirm it by measurement rather than by reading the disassembly:
`core-native-noinline` (`#[inline(never)]`, a lower bound, since IPO survives it)
and `core-native-opaque` (a `black_box`ed function pointer: no inlining, no
devirtualisation, no constant propagation). The three-term split:

| | inlining term, ns/element | group term, ns/element |
|---|---|---|
| P1.3 encode | −0.10 to +0.01 | 11.29 to 11.42 |
| P1.3 decode | −1.03 to −0.82 | 27.63 to 28.36 |
| P1.2 encode | +0.19 to +0.60 | 43.23 to 45.13 |

At nine crossings per thousand elements the dynamic call itself is about 0.02
ns/element, so the second column is group materialisation with a rounding error
attached. **The absent-path inversion is not an inlining artifact.**

**Two things not to carry away from that table.** P1.1 is deliberately absent from
it: with four elements its per-element column is a per-*message* cost divided by
four, so it is not comparable with P1.2's, and it moved from about 40 ns to about
30 between the two binaries for that reason. And a per-element interface cost is
worth quoting only where it exceeds the run-to-run spread, which on M1 and M2
decode it does not.

**The audit's own limit**, stated in its log: one toolchain, one link, no LTO.
With `lto = "fat"`, or where a traversal is small enough to inline cross-crate,
the premise this refutes could become true. What is established is that it is
false for the binaries these figures came from.

**But the audit creates a caveat of its own, and it cuts the other way.**
`core-native` is therefore *not* the fully-inlined no-boundary control its name
suggests — it is "no boundary, but still an indirect call through the GOT". That
makes it a cleaner isolation of the group than intended, and it means **a genuinely
inlined native Rust codec is unmeasured**: with LTO on, or the entry points marked
`#[inline]`, the no-boundary arm could be faster than anything in this document,
and every ratio quoted against `core-native` would widen. Nothing here bounds that.

## The zeroed-group fill answers open decision 9

The candidate: the host memsets the element-group chunk once and assigns only the
fields that differ from the default, instead of section 6's total fill. Built as an
arm beside the default path, with the generator emitting both and nothing the
default path uses changed.

| payload | what it is | ns/element | zeroed / total |
|---|---|---|---|
| P1.2 | M1, every field present | −1.66 to +1.70 | 0.986 to 1.014 |
| P1.3 | M1, the absent path | −4.98 to −4.06 | **0.719 to 0.766** |
| P2.2 | M2, the deciding shape | −26.18 to −13.02 | 0.970 to 0.985 |
| P2.5 | M2, the absent path | −0.42 to +0.17 | 0.983 to 1.007 |

**P1.3 encode goes from 1.108–1.188 of prost to 0.815–0.857: the M1 absent-path
inversion disappears.** P2.2, the row set up to decide *against* the candidate,
shows a small consistent saving instead. The condition — wins on the absent path,
costs less than 5.4 ns per element elsewhere — is met on every payload measured.

**A correction to this session's framing, which the slice was right to make.** I
put this as "reversing a trade ABI v1 already made and priced at 5.4/24.4 ns". It
is not: the array is the *host's* chunk buffer, and the codec still never resets
anything, so section 6's no-reset property is untouched. What changes is only the
host's fill — an unconditional store per field becomes a bulk memset plus a
conditional store. 5.4/24.4 is the right threshold to judge the cost against, not
a cost being paid back.

**Three deliberate-break positive controls** guard the arm, because a silent
fallback to the total fill would have passed byte identity *and* measured the
same: dropping `ResultRaw.name` breaks M1's zeroed rows only; dropping
`TaskDetailed.owner_pod_id` breaks all five M2 rows and incidentally shows P2.5 is
not fully absent; and turning the presence test into a value test on
`Probe.opt_count` breaks P3.1, which is why M3 is in the conformance list at all —
present-and-zero is load-bearing.

**What this does not settle**: whether the same trade holds in a managed host,
where a bulk clear of a struct array and a conditional store cost something quite
different. The C# and Java slices decide that, not this one.

## Decode is bounded by container construction, not by allocation in general

Stage 3 part 1 read this as "decode is allocation-bound". M3 sharpens it, and the
sharper version is more useful to the other slices because it tells them which of
their messages to expect parity on.

`core-native` decode against prost, by element shape:

| payload | what an element costs the host | decode |
|---|---|---|
| P3.1 | flat, 5 fields, 1 to 2 strings | 0.81 to 0.82 |
| P1.2 | 6 blobs, 2 optional children, no container | 0.83 to 0.86 |
| P2.2 | 4 `Vec<String>`, a `BTreeMap`, 27 fields | 0.89 to 0.96 |

**Decode converges to parity in proportion to host-side container construction**,
not to bytes and not to strings: P3.1 and P1.2 allocate plenty of `String`s and
still show the win. A map insert and four vector growths are work every arm does
identically and no codec can avoid.

**The crossings are not the reason**, which is what makes this portable rather
than a Rust result: seven per element at 1.8 ns is 12.6 ns against about 2,200 ns,
0.6 percent, and the no-boundary control converges too.

Three consequences, and the second is the one I would hold other slices to.

- **Retracted.** This bullet previously read "the interface cost is still real and
  still small underneath: about 6 percent of decode on M2, about 2 percent on M1".
  The audit below found that on M1 and M2 *decode* the no-boundary and FFI arms sit
  inside each other's run-to-run spread and the sign flips between builds, so **no
  per-element interface cost should be quoted for those rows in either
  direction**. The absent-path rows (P1.3) are outside the spread and stand.
- **A decode win measured on a container-light message may not survive P2.2.** The
  published managed decode figures deserve re-reading on that basis before the
  report quotes them, because if they came from less string-dense payloads the
  win shrinks on the shape ArmoniK actually sends. This does not overturn them; it
  says they are not yet comparable.
- What is left to win on decode is allocation, not crossings, which is what ABI v1
  open decision 10 is now about.

**Hazard, and it is the slice's own (R9):** the M2 decode ratios have a much wider
run-to-run spread than anything in stage 2, `core-ffi-rust` on P2.3 ranging 0.819
to 0.953 across three processes, because each decode builds a 15,000-`String`
object graph and allocator state varies. Read the ranges, not the medians. The
encode ratios are tight.

## The two shapes .NET could not measure cost nothing

`Probe`'s oneof and its three `optional` scalars ride in the group entirely: **3
crossings for 200 elements, in both directions**, and every arm byte-identical.
Explicit presence was exercised rather than merely present, with all three cases
occurring on all three fields (absent, present-and-zero, present-and-nonzero), and
the payload-free oneof member reached 40 times in 200 elements.

The mechanism is the part for the C# slice to copy rather than the counts: an
explicit field's encode branches on **the presence bit**, never on the value or
the length, which is what lets a present-and-empty string be written as present.
A by-value group reports absent and empty identically unless it is built this way.

One layout decision the slice made and flagged, now in ABI v1 section 6: a oneof
is a discriminant plus every member inlined flat, **not a union**, because a union
makes the group's layout depend on which member is largest and section 10 already
requires layouts to be reproducible by hand. It costs group size on a shape the
schema has 19 of, and the union is recorded as an unmeasured alternative rather
than an equivalent.

## Encode survives the harder shape

0.46 to 0.57 of prost natively and 0.83 to 0.92 through the C ABI on every uniform
M2 payload, against 0.99 to 1.00 for the `armonik` arm. The M1 encode result holds
on nested, mapped, string-dense messages. P2.4 is the exception at 1.03 through
the ABI, and that is decision 5's worst case rather than a shape effect.

## ABI v1 decision 5, answered

Zero warm prefix misses and zero bytes moved on every uniform payload; on P2.4,
one miss per element and 980,938 bytes moved of a 981,222-byte output. Isolated
against two size-matched uniform arms rather than attributed, and floored against
prost's own two-pass construction, **the mechanism costs 1 to 3 percentage points
of an encode on the payload built to defeat it and nothing elsewhere.** Zero
grow-callback invocations anywhere.

The slice's closing argument is the part worth keeping: the worst case cannot be
engineered away. Over-reserving needs a non-minimal varint, which the ABI refuses;
under-reserving needs the move; the alternatives cost every payload to spare P2.4.
So the learned width is right at a bounded worst case, not a bet that the worst
case is rare.

## The defect that byte identity could not catch

`ak_elemu_TaskDetailed` read the open field's tag and site from the context at
entry, and the element body overwrote them, so from the second chunk onwards a
run read whatever the previous element left behind. The visible symptom was a
length-prefix site that would not converge (448 misses in 500 identical elements).
The actual defect is that **a chunking host writes every chunk after the first
under the inner field's tag**: silent wire corruption.

**Byte identity passed throughout, on a tag collision.**
`ListTasksDetailedResponse.tasks` is tag 1 and `TaskOptions.options` is tag 1, so
the wrong tag was the right tag. No message in `design/SHAPES.md` has a root whose
repeated field carries a tag different from a repeated or map field inside its
element, which means **no slice built against this corpus can catch this class of
defect**. That is now corpus requirement 5 in README section 10, and the ABI
carries the rule it implies: an element or run entry point leaves the open-field
state as it found it.

It is not Rust-specific. Any implementation holding "which field is open" in the
context has it, and rule 2 makes that the natural design.

## Crossings on M2

Decode costs **7.004 crossings per `TaskDetailed`**, which is exactly what ABI v1
7.2 predicted against the drafted ABI's 43. The specification's figure was an
argument; it is now a measurement.

Encode costs **10.02 per element** on the same payload, which nobody had. The
batching predicate saves decode and not encode, because on encode the host drives
every one of its own containers, so each loop slot is a crossing whatever the
predicate says. Section 6 now says so.

## The result that changes what a slice may quote

On P1.1 and P1.2 the core beats prost in both directions through the C ABI
(encode 0.59 to 0.72, decode 0.78 to 0.89). **On P1.3, where every element
encodes to nothing, it loses in both directions** (1.16 to 1.39) while the
no-boundary control stays at 0.47 to 0.84.

The whole gap is the by-value group. The fill is unconditional by specification
(ABI v1 section 6), so the binding fills a ~200-byte element group and the core
materialises a ~128-byte one whatever the wire holds, and on the absent path there
is nothing for that fixed cost to amortise against.

This is the first time the group's cost has been charged rather than assumed, and
two things follow.

- **Every slice reports P1.3 and P2.5 as their own rows and does not fold them
  into an average.** A page of results where most fields are unset is ordinary
  control-plane traffic, not a pathological input, and a slice quoting "the group
  is worth X" from a full payload is quoting a number that reverses.
- It opens a real design question, now **ABI v1 open decision 9**: a presence-word
  fast path for an entirely empty element, an element run that can hand over a
  count of empties, or accept the cost and say so. Nothing is decided on one slice
  and one message. M2's P2.5 is the nested case, and the managed slices pay a
  different price for the same fill because theirs crosses a runtime boundary.

## The open decision that turned out to have a price

ABI v1 decision 3 (UTF-8 passthrough) was framed as pure semantics. It is not:
validation is **25 to 30 percent of an encode** here, the largest single knob on
the encode path measured anywhere so far.

The slice's proposal, recorded in decision 3 and not accepted: carry both
transcoders and let the generator pick from the host type. This session's caveat,
which is the reason it is not accepted yet: a trusted transcoder is a correctness
contract a host can be wrong about, and that is precisely the failure mode v1
already removed once when it dropped `max_bytes_per_unit`, where being wrong was
measured as silent wire corruption. The mitigation is real (a generator reading a
type is not a human making a promise) and it makes the question narrow: is there a
host type in any of the five languages where the generator would emit `trusted`
and the invariant does not hold? **C++ answers it for `std::string`, Python for
`str`.** Until one of them does, the default is unchanged.

## What is not established, and what to distrust

- **M1 and M2 only.** Oneofs, explicit presence, the adapter site, bulk bytes,
  packed fields and interleaved repeated fields are unmeasured in Rust.
- **Nothing is measured past 4 levels of nesting.** The description reaches 4 and
  the real schema reaches 6, and the two levels missing are the filter and request
  message family, which this payload set carries none of. ABI v1 open decision 7,
  the decode recursion limit, is therefore unexercised.
- **The packed enum shape was falsely marked covered** until this slice checked
  it: `design/SHAPES.md` attributed it to M2, which has no packed field at all. It
  now lives on M6 and is unmeasured until that message is built.
- **The `armonik` column does not price `packages/rust`.** It reproduces that
  crate's *pattern* (hand-written-shaped types, generated `prost::Message` impls,
  no conversion layer) with this slice's generator, and does not use
  `armonik-macros`. The slice's own defect log is why that caveat has teeth: its
  first measurement of this arm read 1.16 to 1.40 and would have supported "hand
  written types cost something against generated structs", a false finding about
  the bet `packages/rust` actually made; two statements of emitted code closed it
  to 0.94 to 1.03. A verdict on the in-repo crate needs its own measurement,
  quoted standalone and never as a ratio against these columns.
- **The main decode tables in this document were taken with the lossy policy**
  and are not restated. If a rejecting decode is adopted they improve by roughly
  0.08 to 0.12 on M1 and 0.03 to 0.08 on M2, in this slice's favour.
- **The content sets are run on P1.2 and P2.2 only**, and on one x86-64 machine
  with AVX2. The SIMD validator's behaviour where those features are absent is
  not measured, and it is a floor question rather than a target one.
- **The transcode pair is unreachable from this slice** (above), so nothing here
  bears on the Java/.NET substitution disagreement.
- **One machine, one configuration, no concurrency, container, shared hardware.**
- **MSRV declared, not verified.**
- **One open defect in the slice, carried deliberately**: `ak_fail` casts its
  context to the encode context unconditionally, so a decode-side failure would
  corrupt the decode context. Unreachable today and scheduled with the decode
  error channel in stage 3. Worth watching, because it is the error channel ABI
  v1 section 5 calls the widest hole in the drafted interface.

## The RPC half's case is behavioural, and none of the behaviour is measured

This is the most important sentence in this document and it should survive into
the report unsoftened. The argument for putting the RPC layer on the C ABI is that
five implementations currently disagree about which status codes are retried, what
`AllowUnsafeConnection` disables, and the write-then-notify ordering. **Stage 4
measured none of that.** It measured the call path, whose cost was never the
question, and found it free.

Unmeasured on the RPC side: the callback and completion-queue delivery modes,
metadata, deadlines, the gRPC status code as a number, cancellation, retry,
backoff, TLS, streaming, a real network, failure injection and the server seam.

## The specified surface that is not built

From the slice's completeness pass, and it belongs in the report rather than in a
footnote, because a specification is not evidence:

- **`ak_init` and the whole lifecycle of ABI v1 section 3 are not built at all**,
  so "every entry point requires `ak_init`" is unexercised, as are the crypto
  provider, the log and tracing bridges and the panic hook.
- **The codec's rollback of a half-written field is written and never triggered.**
  Section 6 calls that the widest hole in the drafted interface; nothing in this
  slice makes a host fail mid-run deliberately, so the fix for it is untested.
- **Group-layout assertions, the `coder` hint, size and recursion limits, the pull
  decode family and the whole RPC half** are specified and unbuilt here.
- **Malformed wire is unexercised**, and `ak_fail` is reachable only through a
  panic.

## What I would do next, if this slice is reopened

In the order the slice itself proposes, which I agree with:

1. **A concurrency suite** per conformance obligation 12.5. It is the only item on
   this list with a defect already found by accident and nothing looking for the
   class on purpose.
2. **`ak_init` and the lifecycle**, so that section 3 stops being unexercised
   specification.
3. **The pull decode family**, to turn "push is the right default at a 1.8 ns
   crossing" from an argument into a measurement. Rust is the cheapest place to
   learn whether one traversal emitter can really serve both families, which is
   ABI v1 open decision 2 and a condition the whole decode design rests on.
4. **The remaining content sets, and the SIMD validator on a machine without
   AVX2**, which is the floor question behind the reframed decision 3.

None of it blocks another slice. Everything another slice needs from Rust exists.

## What the slice's own defect log says about method

Four defects, each found before timing and each swept across the generator rather
than patched where it surfaced. Two would have produced a publishable wrong
number: the inlining above, and the `armonik` emitter. One (`all_absent`
precedence, which left a Timestamp alive in a payload where everything should be
absent) was invisible to P1.1 and P1.2 and visible only on P1.3.

That is three separate defects in two stages whose detection depended on the
absent path or on the standing question "is this arm actually running". Both are
already rules (R6, and the slice agents' brief); the slice is evidence they earn
their place rather than decoration.

**The slice's largest methodological contribution is one observation seen twice,
in opposite directions**, and it is what both halves of R5 now say:

- an arm that **claims a boundary and has none** — the core as an rlib, every
  entry point inlined, and the boundary counters still reporting the right counts
  because the counting code inlined with them;
- an arm that **claims no boundary and might have one** — a control that could be
  fused into the benchmark loop, which would make the subtraction that isolates
  the interface flatter it by whatever the optimiser found.

Neither is visible to R7's configuration discipline, because LTO, `#[inline]` and
genericity do not appear in a configuration line. Both are visible only from the
built artifact. **An arm is not what its name says until the artifact agrees**,
and both directions are now checked as a build step of the slice's own suite
rather than by an audit that runs when someone goes looking. The C++ slice inherits
the inverse hazard, where the question is `-flto` over the *control* rather than
over the core, and can lift the check rather than re-derive it.
