# Working in `ffi/`

This directory is an exploration branch that is never merged. The deliverable is
`REPORT.md`. Read [`README.md`](README.md) for the question, the rules a slice is
run under (R1 to R13) and the work items. This file is the operating contract:
who owns what, and how work survives the end of a session.

## Roles

| Role | Owns (writes) | Never writes |
|---|---|---|
| **Aggregating session** (the main agent) | `README.md`, `CLAUDE.md`, `design/**`, `findings/**`, `REPORT.md` | slice sources, logs |
| **Slice agent** (`ffi-slice`, one per language) | `poc/<lang>/**`, `logs/<lang>/**` | `REPORT.md`, `findings/**`, `design/**`, `README.md` |
| **Review agent** (`ffi-review`) | nothing at all | everything |

The separation is the point. A slice agent that writes the verdict on its own
slice writes the verdict its last measurement suggested; a review agent that can
build something confirms its own finding. Three of the most important results in
the existing reports are corrections of exactly those two failure modes.

## Resuming

A session ends and the context goes with it. `poc/<lang>/STATE.md` is what
survives, so:

- **Read `STATE.md` first.** It says what exists, what was measured, what the
  next step is and what is currently broken.
- **Rewrite it at the end of every work unit**, not at the end of the slice. A
  stale `STATE.md` is a defect, and it is the only one that costs a whole
  session.
- **Append to `JOURNAL.md` as you go**: what was tried, what it measured, what
  refuted it, in order. The journal is where a later reader finds out that an
  option was already refuted and by what.
- **Within one session, send the live agent another message** rather than
  spawning a fresh one; its context is intact. `ListAgents` shows the live ones.

## Committing

- Slice agents commit their own directory, message prefix `poc(<lang>): `.
- The aggregating session commits documents, prefix `docs(ffi): `.
- **Slice agents do not push.** The aggregating session pushes, so two agents
  never race on the same branch.
- Commit raw logs. A figure with no log behind it does not go in the report.

## Invariants

- **Nothing under `packages/` changes.** The Rust slice reads and measures
  `packages/rust`; it does not edit it.
- **The shapes are fixed by `design/SHAPES.md`.** A slice may add an arm. It may
  not change a shape, because a column covering different shapes is not a column.
- **The floor is a correctness gate, the target is where the clock runs**, and
  the two are allowed to be different code where that makes the target faster
  (`#if NET8_0_OR_GREATER`, a JDK 17 source tree, `if constexpr`). Conditions:
  one generator with a target level rather than a hand-maintained second tree;
  identical wire bytes across levels, checked by running the corpus on each; the
  public surface unchanged, or the divergence reported as a cost; and in C++
  nothing that changes the layout of an installed header type, because the
  consumer picks `-std` and we do not. The floor is measured as arms a, b and c
  of README section 5.2, and only arm a produces ratios.
- **Correctness before timing.** Byte identity across every arm, including the
  absent-field and unknown-field payloads, before any number is recorded.
- **Count crossings, do not infer them.** Every measured payload has a
  boundary-call count from a counting build.
- **Absolutes do not travel between machines, so every slice calibrates its
  own** (R13). Slices run in separate sessions on separate containers, and the
  cross-language crossing table of README section 2 is a table of absolutes. Each
  slice builds and runs the Rust slice's crossing benchmark on its own machine, as
  a build step, and quotes its own absolutes against that number as well as in
  nanoseconds. A slice on its own branch off the exploration branch pushes that
  branch itself; the aggregating session merges. Two agents still never race one
  branch.
- **Keep the slices small.** Cover the shapes and nothing else. What is not
  covered goes in "what is not measured", which every slice ends with.
