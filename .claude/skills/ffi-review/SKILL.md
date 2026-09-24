---
name: ffi-review
description: Run an adversarial review of the ffi/ native-core exploration. Fans out read-only ffi-review agents over one slice, design document or report section, dedupes what they find, and hands confirmed findings to the slice agent that owns the code. Use when the user asks to review a slice, a measurement, an ABI decision or a report draft.
---

# Adversarial review of an `ffi/` slice

Argument: a slice name (`rust`, `cpp`, `csharp`, `java`, `python`), a document
path, or nothing (review whatever changed since the last review).

## Procedure

1. **Establish the target.** Read `ffi/poc/<lang>/STATE.md` and `JOURNAL.md`, or
   the document named. Know what is claimed before anyone attacks it.

2. **Fan out `ffi-review` agents, one angle each**, in a single message so they
   run concurrently. Four angles, and each agent gets exactly one so that none of
   them dilutes into a general read:

   - **Measurement validity**: configurations, baselines, process boundaries,
     hazards, whether each figure is in a log.
   - **Implementation correctness**: does the arm do what the table says it does;
     is an arm silently not running; is the control a real control.
   - **Generator sweep**: a rule applied in one path and not another; a shape
     handled in the emitter but not in the binding.
   - **Comparability**: shape parity against `design/SHAPES.md`, payload
     identity across slices, and whether the conclusion is transferable to the
     runtime it is stated for.

   Tell each agent what is being claimed and where to look. They cannot build
   anything, so they need the paths.

   Tell every agent the phase (`ffi/README.md` section 1.1): container timings
   are instrumentation, so a finding against one matters when it is presented as
   a result or exposes a harness defect the campaign would repeat; and no
   document may recommend, because the decision is the owner's.

3. **Dedupe and rank.** Several agents will find the same thing from different
   angles; that is a signal about severity, not three findings. Drop anything
   that is a preference rather than a defect.

4. **Present to the user** before dispatching: the findings, most severe first,
   each with what would settle it. The user decides what gets fixed.

5. **Dispatch the confirmed set to the slice agent** that owns the code, one
   message with the full list. That agent confirms or refutes each one, because
   a review agent's finding is unconfirmed by construction.

## Rules

- A review agent never writes code, never builds, never benchmarks. If a finding
  needs an experiment to settle it, that experiment belongs to the slice agent.
- The same role never both raises and clears a finding.
- Findings do not go into `REPORT.md` directly. What lands in the report is the
  aggregating session's reading of a finding *after* the slice agent has answered
  it.
