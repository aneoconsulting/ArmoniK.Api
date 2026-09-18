# Reading the rust slice

The aggregating session's reading of `poc/rust`, which is not the same document
as the slice's own `STATE.md`. What is here is what the slice's results mean for
the branch: what is now established, what the other four slices have to do
differently because of it, and what is still an argument.

**Covers stages 1 to 3 part 1**: the payload manifest validated against prost,
and four arms over M1 (P1.1 to P1.3) and M2 (P2.1 to P2.5). M3 to M7, the content
sets and the RPC arm are not built.

## Configuration, once, for everything below

4 vCPU Intel Xeon at 2.80 GHz, 15 GB, Ubuntu 24.04.4, Linux 6.18.44 x86_64, in a
container, no pinning and no governor control. rustc 1.94.1, release. prost
0.14.4, prost-build 0.14.4, protox 0.9.1. **Linkage: the core is a `cdylib`
resolved by the dynamic linker**, which section "The defect" below says is not a
detail. Accessor guard on. ASCII content set only. MSRV 1.88 is declared and
**not verified**: no 1.88 toolchain exists in the container.

Ratios are formed inside one process from interleaved rounds; three processes
were run and what is quoted is the range across them.

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

## Decode is allocation-bound, and it bounds the argument for every language

The largest result so far, and it is a limit rather than a win.

On M1 both core arms decode at 0.75 to 0.89 of prost. On **P2.2, the payload
SHAPES.md says to read first because it is the shape the control plane actually
moves**, they land at parity: `core-native` 0.89 to 1.07, `core-ffi-rust` 0.81 to
1.21.

**The crossings are not the reason, and that is what makes this portable rather
than a Rust result.** Seven crossings per element at 1.8 ns is 12.6 ns against
about 2,200 ns per element: 0.6 percent. The no-boundary control makes no
crossings at all and is at parity too. P2.2 carries 17,500 strings and 2,000 map
entries in 551 KB, so decode is dominated by `String` allocation and map
insertion, which every arm performs identically. On the shape that matters, the
codec is not what decode costs.

Three consequences, and the second is the one I would hold other slices to.

- The interface cost is still real and still small underneath: about 6 percent of
  decode on M2, about 2 percent on M1.
- **A decode win measured on a thinner payload may not survive P2.2.** The
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
- **ASCII only.** SHAPES.md's own rule is that a string-path number without a
  content set is half a number. The Latin-1 and above-U+00FF sets are not run.
- **One machine, one configuration, no concurrency, container, shared hardware.**
- **MSRV declared, not verified.**
- **One open defect in the slice, carried deliberately**: `ak_fail` casts its
  context to the encode context unconditionally, so a decode-side failure would
  corrupt the decode context. Unreachable today and scheduled with the decode
  error channel in stage 3. Worth watching, because it is the error channel ABI
  v1 section 5 calls the widest hole in the drafted interface.

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
