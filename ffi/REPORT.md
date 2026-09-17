# One native core, N bindings: the report

**Not written yet.** This is the deliverable of the `rust/native-core-ffi-poc`
branch and the only artifact taken into account at the end. It is written last,
by the aggregating session, from the slice journals and the per-slice findings.

Until then, the state of the work is:

- [`README.md`](README.md): the question, the rules, the work items W1 to W9.
- [`design/SHAPES.md`](design/SHAPES.md): what every slice implements.
- [`design/ABI.md`](design/ABI.md): the reconciled ABI. **W1, not written.**
- `poc/<lang>/STATE.md`: where each slice actually is.

## What this report will have to answer

1. Which of the three outcomes in README section 13 the evidence supports, and
   what rules out the other two.
2. Per language: what adopting the core costs against what that language ships
   today, on the target configuration, decomposed into interface cost and runtime
   tax using the Rust slice.
3. Whether the design constraints hold: C++11, netstandard2.0 or .NET Framework
   4.8, Java 8. Compiles and passes correctness, not timings.
4. What it does not establish. Every slice ends with that list and this report
   inherits all of them.
