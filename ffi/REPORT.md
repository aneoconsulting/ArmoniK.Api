# One native core, N bindings: the report

**Not written yet.** This is the deliverable of the `rust/native-core-ffi-poc`
branch and the only artifact taken into account at the end. It is written last,
by the aggregating session, from the slice journals, the per-slice findings and
the campaign's logs (W13).

**It is a factual record, not a recommendation.** The decision is the project
owner's. The report states what is established for each option, what is not,
and on what evidence; it does not choose between the options, rank them, or set a
threshold for what counts as a material cost.

Until then, the state of the work is:

- [`README.md`](README.md): the question, the phase (section 1.1), the rules, the
  work items W1 to W14, the options (section 13).
- [`design/SHAPES.md`](design/SHAPES.md): what every slice implements.
- [`design/ABI-v1.md`](design/ABI-v1.md): ABI v1. **W1, drafted, not yet agreed.**
- [`design/FIX-PLAN.md`](design/FIX-PLAN.md): the plan after the 2026-09-24
  review, and its findings register.
- `poc/<lang>/STATE.md`: where each slice actually is.

## What this report will have to answer

1. **Per option of README section 13**: the facts established, and what is not
   established.
2. **Per language**: what adopting the core costs against what that language
   ships today, on the target configuration, per direction and at both the codec
   and the RPC level, from the campaign. Where the campaign measured it on one
   machine, the decomposition into interface cost and runtime tax using the Rust
   slice (README section 4.1).
3. **Whether the design constraints hold**: C++11; C# net6.0 and .NET Framework
   4.8; Java 8; Python 3.7. Compiles and passes correctness, not timings.
4. **Unknown-field retention** (ABI v1 decision 11): what each behaviour costs per
   language, so the owner can decide.
5. **The maintenance side, as facts**: where the five packages diverge today
   (W12), and whether the generator is one implementation (W14), with the
   evidence.
6. **What it does not establish.** Every slice ends with that list and this report
   inherits all of them, including that the payload set is not weighted by real
   traffic.
