# csharp slice: state

**Read this first. Rewrite it at the end of every work unit.** It says what exists in the tree
and what was checked. It carries no recommendation and no verdict (the decision is the owner's).
Every figure in this slice is container instrumentation (README 1.1), never a result; timing
waits for the campaign. The history of how each item got here is in `JOURNAL.md` (entries 1 to
85); this file states what is true now.

| | |
|---|---|
| **Status** | 2026-10-10 (latest): **s14**, measurement only, nothing changes a default: part 1, the E1R bimodality across processes under the default .NET 8 JIT configuration (distribution per JIT configuration, the JIT code and runtime samples of fast and slow processes, the BDN grid's spread); part 2, E1R's excess over E0 split by a ladder of rungs (additive measurement-only core features, default OFF; `logs/csharp/opt/s14/`; see **s14**). Before it, 2026-10-09/10: **s13**, the core's UTF-16 -> UTF-8 transcoder in scalar Rust (additive core features, default off) measured against simdutf under E1R (`logs/csharp/opt/s13-tc-scalar/`; see **s13**). Before it: **D25** (owner, FIX-PLAN d479b702): [SuppressGCTransition] dropped. The s12 attributed path (tested and timed, `logs/csharp/opt/s12-sgt/`) is removed; the code is back to D24's (generated C# identical to 357319f6), checks and counts unchanged (`logs/csharp/opt/d25/`; see **s12 / D25**). Before it: **D24**, the FSM is the target decode family: the `core-ffi` codec arm and every RPC cell where the core decodes use it; push is the labelled extra `core-ffi-push`, pull stays `core-ffi-pull` (see **D24**); the full gate at 1a5ccaea PASSED on both h2 variants (`logs/csharp/opt/d24/`). Before it: **s11**, the eight-arm decode table rerun on the core with the FSM's fixes A and B (core 081de788; no C# change: regenerated identical), checks passed (`logs/csharp/opt/s11/`; see **D23**, last item). Before it, 2026-10-09: **D23** (the FSM decode family's C# consumer, on the amended contract c2b95f62: begin/next return the op) built, checked (gen/s10_checks.sh) and timed in the eight-arm decode-read table (`logs/csharp/opt/s10/`; see **D23**). Before it, 2026-10-04: optimisation pass steps 1 to 4, 5 (D20), 5b (static decode vtable) 6 (D21, string encode paths E0/E1/E2/ETH selectable by AK_STR_ENC, default E0) and 7 (D21: E3, E3L, E1R, E1C, threshold and ASCII splits; kernels; attribution) implemented and measured; s8 (decode attribution, harness-only arms, no optimisation) measured; 9a (decoded runs pre-size their list or map) in; 9b (the owner's six-arm decode table) measured (not gated; net8.0 quick checks per step; see **Optimisation pass**), after the short baseline (see **Optimisation baseline**) and the single-CPU guard it made necessary. Before it: D18 done (CAMPAIGN section 4.0 as amended b58543f7b: `AK_CAMPAIGN_GRID=core|full`, default core; transport `armonik` in the core grid; see **Campaign grid**); before it FIX-PLAN WP13 done (TCP 127.0.0.1 with TCP_NODELAY read back, perf task-clock beside the process clock, softirq on the CLIENT CPUs, pools at AK_WORKERS, both h2 variants gated and labelled, D9 stated: see **WP13**). Before it: WP10 done (every RPC cell against the Rust slice's rpc_server; this slice's server removed), then req 22a as amended (e6c909630): BDN's default toolchain (one child process per case) for the campaign, InProcessEmit grouping a small-run switch. Gate and smoke: see **Gate** and **Smoke**. Findings are in scope only if they can change what the campaign measures (ffi/CLAUDE.md, "Scope of findings"). |
| **Levels** (FIX-PLAN D2) | target **net8.0** (.NET 8.0.31, SDK 8.0.131); floor **net6.0** (.NET 6.0.36 from the NuGet runtime pack, self-contained publish): gated; floor **.NET Framework 4.8**: compiled only (`src/HarnessFloor`), never run (needs Windows; the container has no Mono) |
| **Incumbent** | Google.Protobuf 3.32.0, Grpc.Tools 2.72.0, Grpc.Net.Client and Grpc.AspNetCore 2.71.0 (the versions `packages/csharp` ships) |
| **Core** | the one core, `ffi/poc/codec`, built from `git archive HEAD` by `gen/build_core.sh`, every build with `init-guard`: full `target-core` (`rpc`), `target-core-count` (`rpc,count`), `target-core-corpus` (`corpus`); no-unknown (ak-core `--no-default-features`) `target-core-nounk`, `target-core-count-nounk`, `target-core-corpus-nounk`, each in its own target dir; the same four transport cores against h2-batch (`poc/codec/h2-batch/`, D11 as amended) as `target-core[-count][-nounk]-h2b`; the h2 compiled into each is printed by build_core.sh |
| **Machine** | a container, 4 vCPU Intel Xeon, Linux 6.18.44; nothing in this file depends on it |

## What exists

### The shared C# backends (`ffi/poc/codec/gen/`, owned by this slice)

Each imports `plan` (and `cs_names`/`cs_types`) and nothing from the IR or a description.
Every one renders both variants: the full plan, and the plan relowered with `unknown="drop"`
(`plan.unknown_compiled_out`), from the same functions.

```
cs_names.py         C# spellings and the file banner
cs_types.py         facade types and the structural comparer; every class carries `UnknownFields` in the
                    full build and none in the no-unknown build (CAMPAIGN req 10, R-H22)
cs_managed.py       the managed codec (host-gen): one-pass and two-pass encode, decode; utf8 and the
                    depth limit are the plan's options; ONE unknown-field mode per codec ("both"
                    refused, R-H11): `Codec` from the drop plan (no capture code), `CodecRetain`
                    from the retain plan (the full build only)
cs_binding.py       the P/Invoke binding: groups, vtables, presence bits, every import as
                    LibraryImport under #if NET7_0_OR_GREATER and DllImport under #else, and
                    under AK_HOST_COUNT a wrapper counting its calls by name (req 19, WP7),
                    ak_init from plan.lifecycle, AbiLayout.Table()/Facts(), decision 11's
                    ak_dec_<Root>_opts and ak_dec_ctx_new_/ak_dec_reset_<Root> (full only),
                    AbiVariant (which variant, and a check that the loaded core is it); the RPC half
cs_host.py          CoreFfi_<Root>: encode / uencode, push and pull decode on one root-bound
                    context; full: retain through native options, one grow over
                    NativeMemory.Realloc (geometric, clamped to INT32_MAX), ONE reset per decode
                    (before it), bags taken into UnknownFields, UNDELIVERED check, per-position
                    discard helpers, UnkHost.Exact (an exact-size grow, a gate control only);
                    no-unknown: none of these
cs_layout_probe.py  the layout probe's Rust source, parsed out of ak-abi's Rust declaration text
                    (both variants, shapes and corpus); reads no plan (R-E6)
```

### This slice (`poc/csharp/`)

```
gen/generate.py [--check]   renders every generated file (both variants) through the shared
                            backends; --check = drift + the one-generator guard + its planted test
gen/glue.py, cs_values.py, cs_build.py, cs_arms.py, cs_proj.py, cs_registry.py, cs_campaign.py
                            harness glue only (values, payload builders, arm table, corpus
                            projection and dispatch, core-ffi registry, campaign per-root calls)
gen/build_core.sh           the core snapshot, the six core builds, the four layout probes
gen/gate.sh                 THE GATE (correctness only, nothing timed), both variants, net8.0 + net6.0
gen/crossings.txt           committed R5 counts, full build (22 rows: the CoreArms tally against the
                            core's own counters; P1.2, P2.2, P2.4 in three content sets)
gen/crossings-nounk.txt     the same, no-unknown build (22 rows)
gen/counts.txt              CAMPAIGN req 19 as amended: every exported entry point one call of each
                            timed core-ffi case calls, by name, resets and grows placed (1,051 cases: the full grid's 1,044 and the core grid's 7 U-* rows at end state ii)
gen/counts-nounk.txt        the same, no-unknown build (551 cases, the same 7 added)
gen/rpc-counts.txt          the same per call of every RPC cell A-F, framed twin, Cc copy cell,
                            direction and upload payload (105 rows)
gen/rpc-counts-nounk.txt    the same, no-unknown client (62 rows)
abi/                        the layout probe crate (features: unknown-fields default, corpus)
Directory.Build.props/.targets  build configurations: /p:AkNounk=true (the no-unknown build:
                            GeneratedNounk/ replaces the variant files, define AK_NO_UNKNOWN_FIELDS,
                            output bin-nounk/ obj-nounk/); /p:AkHostCount=true (the counting build,
                            AK_HOST_COUNT, bin-count[-nounk]/, never timed); /p:AkFloor=true (README
                            5.2 arm b, not gated)
src/Facade/                 the hand-written runtime (Wire.cs: error codes equal plan.FIXED's, R-H14;
                            OrderedMap.cs) + Generated/ (Types, Eq, Codec, CodecRetain, Values, Build)
                            and GeneratedNounk/ (Types and Eq without UnknownFields, Codec)
src/Harness/                harness: conformance | unknown | groups | utf8 | mapforms | layout |
                            coreffi | content | counts | bench; net8.0 + net6.0; Generated/,
                            GeneratedNounk/
src/Corpus/                 the corpus runner (net8.0 + net6.0): 4 arms full, 2 arms no-unknown,
                            each row in a child process; --unk-controls, --wrong-root, --variant
src/Rpc/                    akrpc (a client only, no server since WP10): `bench` (the timed RPC grid on
                            BenchmarkDotNet, RpcBench.cs), `campaign --suite calib`, `--suite rpc
                            --counts` and `--upload-check` (the counting build, the gate); cells A-F
                            against the campaign server's armonik.ffi.campaign.v1.Grid; the gate's
                            --layout and --error-path --sock; --shared-ctx
src/BenchDotNet/            the codec suite's engine: BenchmarkDotNet 0.15.8, InProcessEmit, one
                            process per arm:mode unit, process CPU per iteration (CpuClock), JIT tier
                            read back (JitTiers.cs), encode variants and pools (Cases.cs), the Grpc.Net
                            frame (GrpcFrame.cs), `--counts` (CountRun.cs, the counting build)
src/HarnessFloor/           net48, compile only (the binding; the host half is compiled out)
run_campaign.sh             --suite codec|rpc|calib|gate --out DIR (CAMPAIGN req 31)
```

## s14: the E1R bimodality and the E0 -> E1R ladder (2026-10-10; JOURNAL 85; container instrumentation)

The container restarted before the unit: the machine is now a 2.10 GHz Xeon with AVX-512
(Vector512 accelerated); s13 ran on a 2.80 GHz part without it. No absolute here compares with s13.

- **Part 1, distribution** (`dist/summary.md`; gen/s14_dist.sh, run from a worktree at be79f6a1,
  the s13 binding; 20 processes per configuration, one-string sweep, 48 units, ASCII and Latin-1,
  E0 and E1R interleaved; per-process medians, ns): default 2 of 20 processes slow (E1R 102-660
  ASCII, q1-q3 103-110; Latin-1 106-663); DOTNET_TieredPGO=0 0 of 20 (114-121 / 118-127);
  DOTNET_TieredCompilation=0 0 of 20 (116-149 / 118-152); DOTNET_TC_OnStackReplacement=0 0 of 20
  (102-112 / 106-120); DOTNET_ReadyToRun=0 6 of 20 (102-673 / 106-666). E0 is never slow. A slow
  process is slow in every round and for both contents. At a slow share near 10 %, 0 of 20 alone is
  not evidence that a configuration removes the mode.
- **Part 1, cause** (`jit/`): the hot methods reach the same final tiers in fast and slow
  processes (Tier1 with Dynamic PGO; the sweep loop Tier1-OSR) but with different code: the slow
  EncodeInto (1977 bytes, the same in both slow processes disassembled) calls the thread-static
  base helpers 13 times (9 non-GC + 4 GC) against 8 in the fast one; there is no inline TLS access
  in either. No syscall difference (strace). gdb stack samples with perf maps: the two slow
  processes 12 of 25 and 12 of 30 samples in libcoreclr, entered from EncodeInto right after its
  thread-static helper call sites, in a runtime function doing a hashed lookup (div, mfence);
  fast processes 0 to 1 of 30. libcoreclr carries no symbols here (the function is unnamed).
  Established at medium confidence: in a slow process the thread-static base helpers take a
  runtime slow path on every encode. Not established: why only some processes, and why the
  configurations move the share.
- **Part 1, BDN grid** (`grid-spread/`, P2.2 E1R retain, 6 host processes per configuration):
  no slow mode in these 12 processes; s13's 1,205 vs 1,819 us is not reproduced, so whether that
  spread was this effect is open. JitDisasmSummary in BDN's children not taken (one JitStdOutFile
  for host and children).
- **Part 2, rungs.** R0 E0; R1 E0 through the generic transcoder path (core feature
  `tc-measure-generic`: ak_tc_bytes returns an identity copy that is not the trusted passthrough);
  R2 E1R with a UTF-16 stub copying `len` raw bytes (core feature `tc-measure-u16stub`; output
  wrong by design, byte checks skipped and stated, the stub detected by its marker export
  `ak_measure_tc_stub`); R3 E1R; R3g E1R with `AK_STR_NOGUARD=1` (the frames skip the patch
  counters, Go skips the marks == patches check; Marked++ stays because it decides whether the
  frames run). Both core features default OFF; the default build byte-identical
  (7b8ed0b888940ab7, `cores2.log`). Every rung checked in the build (`checks-rungs/summary.txt`:
  gdb hits tc_copy_generic under R1 and tc_utf16_stub under R2; NOGUARD drops RootPinR_e's two
  thread-static helper calls). Byte identity of every rung but R2: `checks-ladder.log` and the
  pre-timing checks of every grid cell.
- **Defect found and fixed in the unit:** the first stub core folded tc_utf16_stub into
  tc_utf8_trusted (identical bodies, one address), so the encoder's fast path took the stub; its
  sweep processes were deleted and rerun on the fixed core (c6e29dbe), and gen/s14_cores.sh now
  refuses a measurement core whose getter returns the trusted passthrough's address.
- **Measured** (absolute times, per rung): `ladder-sweep/table.md` (ns per string, 8 processes per
  kind and configuration, TieredPGO=0 and default fast-mode processes, plus in-process E1R - E0
  differences; `reference.md` per process) and `grid/table.md` (us per op and per-string steps,
  3 host processes per variant, TieredPGO=0 and default with no slow-mode filter; `reference.md`
  min-max and per-rep medians). ASCII, per string: R3-R0 29 to 42 ns (TieredPGO=0) and 20 to 36
  (default); R1-R0 2 to 6 / 3 to 8; R2-R1 13 to 30 / 4 to 23 (largest on P2.4); R3-R2 3 to 16 /
  4 to 13; the guard (R3-R3g) 0 to 6 / 0 to 9, not resolved from zero (per-rep medians differ by
  about 3 to 5 ns per string on P2.2). R2-R1 is E1R's structure net of E0's own UTF-8 encode, not
  the structure alone. Latin-1 and wide: the stub writes fewer bytes than the real output, so
  their R2 is not comparable and their steps are not attributed beyond R3-R2.

## s13: scalar Rust UTF-16 -> UTF-8 transcoders under E1R (2026-10-09/10; JOURNAL 84; container instrumentation)

- **Core (cb20ae6e, poc/codec, additive, default OFF):** ak-core features `tc-scalar-naive`
  (`utf16_to_utf8_into` writes with the pre-D19 `utf16_write_replacing`; under the worst-case
  buffer, the scalar count first) and `tc-scalar-word` (`utf16_write_word`: 8 units tested as two
  u64 against 0xFF80FF80FF80FF80 and written as 8 bytes, then 4, then one unit at a time as
  utf16_write_replacing); at most one (compile_error). They change `ak_tc_utf16` and the E3
  export `ak_utf16_to_utf8` (one function); lengths, validation and decode unchanged. The
  default build is byte-identical (rebuilt from the snapshot: 7b8ed0b888940ab7 = target-core).
  LLVM SLP-vectorised the word writer's 8-unit path (one SSE2 16-byte load, the pack in SSE2);
  the naive path has no vector instruction (`s13-tc-scalar/vectorisation.txt`).
- **Cores** (`gen/s13_cores.sh`, `cores.log`): target-core-tcnaive, -tcword (rpc,init-guard),
  target-core-corpus-tcnaive, -tcword.
- **Harness:** `AK_CORE_LIB=<path>` (src/Harness/CoreLib.cs, a DllImport resolver; the
  generated AbiVariant check loads the same file): BDN's default toolchain rebuilds each child
  from the project, which copies target-core's library, so a copy beside the host reached the
  host only; each child now records the core files it maps (`core_maps` on its rows; checked:
  only the variant core). `gen/opt_ab.sh` variants take `AK_AB_CORE=<dir>`.
- **Checks:** the differential (`differential.log`: cargo test, release and debug, the three
  builds: the word writer and each build's utf16_to_utf8_into against utf16_write_replacing on
  lengths 0-300 x 4 mixes x 6 and 1023-65537, lone and reversed surrogates, caps 3n, 3n-1, need,
  need-1, need+1, 0; the D19 tests); `checks.log` (gen/s13_checks.sh): Cases.Verify (every
  string path) under E0, E1R, E1R:128 and the corpus under E1R (retain strict), each core: PASSED.
- **JIT finding:** under the default .NET 8 JIT configuration the one-process sweep's E1R
  figure is bimodal ACROSS processes: about 120-150 ns or about 700 ns for the same 48-unit
  ASCII string and core (3 runs: 125, 152, 711 ns; `sweep-default-jit-BIMODAL/`: the default
  core's two processes 702 ns, the scalar cores' 122-169 ns); with DOTNET_TieredPGO=0 139-150 ns
  over 3 runs, with DOTNET_TieredCompilation=0 133, DOTNET_TC_QuickJitForLoops=0 119. The sweep
  was therefore run with DOTNET_TieredPGO=0 (`sweep/`). The BDN grid runs with the default
  configuration (as the campaign). In it, E1R's two per-process medians differ by up to about
  50 % on some rows (simdutf core: P2.2/latin1 1,205 / 1,819 us, P2.4 drop 1,522 / 1,873 us) where
  E0's and E1R:128's stay close: consistent with the same per-process JIT effect, not shown to be
  it. Not attributed further (perf does not run in this container).
- **Measured:** `sweep/table.md` (ns per string; E0 from the default core's process, E1R per
  core; `reference.md` per process, each with its in-process E0) and `grid/table.md` (us per op,
  encode-core-hot, core-ffi drop and retain, E0 / E1R / E1R:128 per core; `reference.md`).
  The grid spans two container restarts: the first whole-grid run (opt_ab.sh) was cut in rep 1
  (`grid-INTERRUPTED/`, not used); the second completed 7 rep-1 cells before the next restart;
  `gen/s13_grid.sh` then resumed cell by cell (a cell = one variant x one rep, one process with its
  own quiet wait; complete = rc 0 and 363 jsonl lines; the cut cell moved to
  grid-INTERRUPTED/ as -cut2; each cell's line in `grid/header.txt` carries the boot time) and
  committed after each rep. Every cell's rows record the core its child processes mapped
  (`core_maps`): each the intended core. E1R:128 takes E0's path on every grid row (no grid
  string reaches 128 units), so its columns do not involve the transcoder.

## s12 / D25: [SuppressGCTransition] on the FSM's begin / next: tested, then dropped (2026-10-09; JOURNAL 82, 83)

**D25 (owner, FIX-PLAN d479b702): dropped.** The attributed path described below was removed
in the D25 commit: cs_binding's `_sgt` imports, `TryFsmSgt` and the attributed loop (FsmRun is
the plain loop again), the arm `core-ffi-sgt`, `AK_BDN_S12`, `AK_FSM_SGT`, `--sgtbench`
(SgtBench.cs) and gen/s12_checks.sh. Regenerated: every generated C# file identical to
357319f6 (D24). Kept: `gen/s12_tables.py` (re-renders `s12-sgt/ab-s12/table.md` byte-identical
from the jsonl) and the logs `logs/csharp/opt/s12-sgt/` as the record. After the removal
(`logs/csharp/opt/d25/`): gen/s10_checks.sh PASSED (verify-fsm both builds, corpus both builds,
four plants, codec counts equal to gen/counts.txt and counts-nounk.txt), and the four RPC count
files reproduced identical (`d25/rpc-counts.log`). What follows describes the s12 test as it was
run (commits 3a90feca, e7cd5d6d); none of it exists in the tree now.

- **Built:** cs_binding `emit_import_sgt` / `fsm_sgt_imports`: `ak_fsm_begin_<R>_sgt`,
  `ak_fsm_next_<R>_sgt`, the same exports with `[SuppressGCTransition]`, `#if NET5_0_OR_GREATER`
  (net48 has no attribute), LibraryImport on net7+, DllImport otherwise, counted under their own
  names in the counting build. cs_host: `TryFsmSgt` beside `TryFsm`, one `FsmRun` with two loops
  (attributed / plain); nothing shared with push or pull beyond what was. Harness: arm
  `core-ffi-sgt` (RootOps `DecFfiSgt`), `AK_BDN_S12=1` (full build: core-ffi drop, core-ffi-sgt
  drop, core-ffi retain; no-unknown build: core-ffi, core-ffi-sgt), `AK_FSM_SGT=1` routes the FSM
  checks (--verify-fsm, the corpus's ffi arms, the plants) through TryFsmSgt, `--sgtbench`.
- **Where the attributed imports are used (validity rules of SuppressGCTransitionAttribute):**
  drop mode and the no-unknown build only. Retain always takes the plain imports: next can call
  the host's grow (fsm_unk_put, an UnmanagedCallersOnly reverse call). No retain-without-grow
  variant was built. A context's first FSM decode takes the plain imports (`_fsmWarm`): that
  begin allocates the FSM state (fsm_of: the Box, the 32 KB arena; grp.resize). From
  ak-core src/fsm.rs, nothing else in begin / next allocates (grp.resize only when a root needs
  more group words than the context's first decode sized, and a context is bound to one root), no
  lock is taken (the counters are plain fields; the count build's bumps are not locks), no syscall
  is made, and drop mode calls no grow. Not checked with a malloc interposer.
- **Duration** (`s12-sgt/sgtbench/longest-call.md`; per event the minimum over 30 decodes, then
  the row's longest; two processes per build; plain import): calls above 1 us on P1.2 (7 of 8
  calls per decode; longest 11.3 us drop, 17.5-17.7 us no-unknown), P3.1 (one call, 6.0-6.1 us),
  P1.3 (one call, 1.09-1.25 us); every other grid row's longest call is 0.05-0.84 us (P2.4's
  0.79-0.84 the highest). The attribute's "under 1 us" rule does not hold on those three rows.
- **Checks** (`s12-sgt/checks.log`, gen/s12_checks.sh): gen/s10_checks.sh on the plain path
  PASSED and with AK_FSM_SGT=1 PASSED (verify-fsm both builds, corpus FSM arms both builds, four
  plants, and a fifth planted in the attributed loop only, caught); counting builds with
  AK_FSM_SGT=1: 468,694 of 953,057 FSM calls (full; retain and first decodes plain) and 468,505 of
  484,165 (no-unknown) through the attributed imports, 0 failures.
- **Timings** (`s12-sgt/ab-s12/table.md` times only; `reference.md`; `sgt-vs-plain.md` per-rep
  medians): BDN campaign harness, core-grid settings, client CPUs 0,1, 2 reps, quiet before each.
  `s12-sgt/sgtbench/*.md`: the crossing microbench (raw begin + next loop, P7.1 and an empty
  element, interleaved 40 ms blocks, 6 rounds, CPUs 0,1, two processes per build).

## D24: the FSM is the target decode family (2026-10-09; JOURNAL 81; correctness only, nothing timed)

FIX-PLAN D24 / CAMPAIGN 4.0, 4.1 req 8 (28a8dba3). Commit ff1bf0c2 (code, counts) and the
unit's last commit (gate logs, STATE, JOURNAL).

| arm / cell | decode family |
|---|---|
| codec `core-ffi` (retain, drop, no-unknown): decode, decode-read, decode-reencode, and the graph an encode row of a U-* row encodes (FromWire 3/4) | FSM (`TryFsm`) |
| codec `core-ffi-push` (labelled extra; full grid and S9/S10 sets; decode, decode-read) | push (`TryDecode`) |
| codec `core-ffi-pull` (labelled extra) | pull |
| RPC: every cell whose response the core decodes: C (blocking) and its delivery cells (callback, callback-inline, queue; the framed Cf cells' a+read is C's), D (the Grpc.Net marshaller); E and F decode with host-gen in C#, unchanged | FSM (4 call sites in src/Rpc/Campaign.cs); no push twin (the runner had no decode-family switch) |
| corpus `ffi-drop` / `ffi-retain` | FSM, and every row also decoded by push and pull: same code, same re-encoding, or the row fails; `AK_CORPUS_PUSH=1` adds `ffi-push-drop` / `ffi-push-retain` |
| corpus `--unk-controls` | push is still the zeroing reference (TryDecodeZeroing is push's); pull == push and FSM == push |
| gate CoreGate (`harness coreffi`) | `dec` / `val` / `ret` = FSM (RT, value, no reverse call; retain with FsmU and DecodeU); `push` and `pull` columns = extras; R5 compares FSM and push with the core's counters |
| BenchDotNet `--decattr` (s8 attribution), Harness `bench` (pre-campaign) | push, named so (unchanged meaning) |

- **P7.1 in the gate:** CoreGate now decodes the committed interleaved vector (as the codec
  suite has since D23), so its P7.1 push reverse count is 7 (was 3).
- **Counts regenerated** (before copies in `logs/csharp/opt/d24/counts/`): gen/counts*.txt and
  every gen/counts-str-*.txt: only core-ffi decode, decode-read and decode-reencode rows change
  (FSM: fwd = reset + events, rev 0), plus the new core-ffi-push rows, each equal to the old
  core-ffi row (88 full, 44 no-unknown); gen/crossings*.txt: eight counts per row (encode, FSM,
  push, pull), the push and pull columns unchanged except P7.1; gen/rpc-counts*.txt and
  gen/rpc-delivery-counts*.txt: only the rows that had a push decode change (C, D and the
  delivery cells), none still names ak_decode_.
- **Two defects found by the first full gate (ff1bf0c2, `d24/gate-stock-FAILED-ff1bf0c2.log`),
  both fixed (1a5ccaea):** (1) the net48 floor did not compile since step 9a:
  `OrderedMap.EnsureCapacity` used Dictionary/List EnsureCapacity under `#if !NETSTANDARD2_0`,
  and HarnessFloor compiles the facade for net48; now `#if NET6_0_OR_GREATER`, else the order
  list's capacity only. (2) the delivery-cell counts were not reproducible: the FSM mask was set on
  a context's first FSM decode, which in the callback / queue cells lands on whichever thread
  decodes. The mask is now copied once in EnsureDec when the context is created (one call, never
  per decode; TryFsm no longer tests it), and the delivery counts drop `ak_fsm_set_pvt_*` by name
  with the other context-creation calls they already dropped (ak_dec_ctx_new_*,
  ak_dec_set_pvt_*). Delivery counts then identical over three runs per build.
- **Gate:** step 7 runs the corpus also with the push arms (net8.0); new step 10: BenchDotNet
  `--verify` both builds, `--verify-fsm` both builds (Rust events compared when the Rust log is
  present), four defects planted in the generated FSM consumer (each must fail), then the
  generated file restored.
- **Full gate at 1a5ccaea, from a clean worktree, both h2 variants: GATE PASSED**
  (`logs/csharp/opt/d24/gate-stock.log`, `gate-h2-batch.log`; net8.0 and net6.0, net48 compiled,
  both builds, every count file, the 4 FSM plants and every earlier control failing as required).
  This is also the open item "a full C# gate at the end of the optimisation pass". The first
  attempt at ff1bf0c2 failed (`gate-stock-FAILED-ff1bf0c2.log`, the two defects above).

## D23: the FSM consumer (2026-10-09; JOURNAL 79; container instrumentation, NOT gated)

Core: c2b95f62 (D23 as amended: `ak_fsm_begin_<R>` / `ak_fsm_next_<R>` RETURN the event's op,
AK_BDR_NEW/ADD/APPLY_ELEM/APPLY > 0, APPLY = the root group, always last, the end; < 0 an error;
`ak_fsm_ev { u32 slot; u32 n; i64 token; void* data; u32 bytes }`, 32 B). Cores rebuilt at it
(`s10/build-core-op.log`). Commits 7c5a2c0b (work in progress), f2797456 (the consumer on the
amended contract and its checks), and this unit's last commit (table, STATE, JOURNAL).

- **Binding** (`cs_binding.py`): `fsm_imports` renders the three entry points per root from
  `plan.fsm_entry_points` (counting wrappers under AK_HOST_COUNT like every import); `_consts`
  renders plan.FIXED's 32-bit constants (the AK_BDR_* ops). Abi.cs and the by-name probe
  (`abi/src/main.rs`) regenerated: ak_fsm_ev slot, n, token, data, bytes.
- **Consumer** (`cs_host.py`, `_emit_fsm`, per root): `TryFsm(src, len, retain, out R)`,
  `Fsm` / `FsmU`. Shares NO code with the push callbacks or the pull Replay (owner's rule): its
  own entry path (TryFsm), its own native D20 mask `FsmPvt` (every bit; copied by
  `ak_fsm_set_pvt_<R>` once per context), its own dispatch (`FsmDispatch(t, b, op, ev)`: switch on
  the returned op, then on `slot >> 16` for NEW / APPLY_ELEM and on the whole slot for ADD), its own
  run appends (`_fsm_append`, a separate renderer, EnsureCapacity first as the others). Shared,
  as push and pull already share them: the facade side (group readers `G.D_<Msg>`, `G.Str` strict,
  `G.Bytes`, the unknown-bag hand-over `G.Take` / `G.Drop` and the arena), the context, and
  decision 11's arming (EnsureDec, ArmFor, Disarm). Loop: `op = begin(..., &ev); while (op > 0)
  { if (op == APPLY) { root group; break; } FsmDispatch; op = next(..., &ev); }`; ONE `fixed`
  spans begin and every next; each payload consumed before the next call; `_fwd` += 1 per call;
  DecoderFallbackException -> AK_ERR_TRANSCODE; on any error the partial object is dropped;
  UNDELIVERED as pull.
- **[SuppressGCTransition] on begin/next:** legal in drop only. In retain the core calls the
  host's grow (`UnkHost`, UnmanagedCallersOnly) from inside begin/next: a reverse call, which the
  attribute forbids. In drop the FSM makes no reverse call; a call runs for at most one 32 KB arena
  of elements (on the large rows tens of microseconds), during which a GC would wait for the
  thread. Not rendered and not measured.
- **Harness:** RootOps `DecFfiFsm`, `DecFfiFsmGraph`, `DecFfiPullGraph`, `TryFsmRc` /
  `TryPullRc` / `TryPushRc`, `FfiForward`, `ReEncHost` (managed re-encoding of a graph, the value
  comparison that does not go through the core); arm `core-ffi-fsm` (Cases.Build);
  `AK_BDN_S10=1` = the eight arms (S9's six + core-ffi-fsm retain / drop, decode rows only);
  `BenchDotNet --verify-fsm`; corpus `AK_CORPUS_FSM=1` (arms ffi-fsm-drop / ffi-fsm-retain, each
  also requiring push's and pull's code and re-encoding on the row); corpus registry `DecodeFam`;
  `gen/s10_checks.sh`, `gen/s10_tables.py`.
- **P7.1's decode input (defect found and fixed, D48):** until this unit every C# decode row of
  P7.1 decoded the incumbent's CONTIGUOUS re-encoding of the graph, not the committed interleaved
  vector (SHAPES.md: P7.1 is two repeated fields interleaved; same 98 bytes, a permutation of the
  same triples). Found because the FSM's call count (3) differed from the Rust slice's events (7).
  Now `Cases.DecodeWire`: P7.1's decode rows decode `schema/generated/payloads/P7_1.bin` (checked
  to be a non-identical permutation of the incumbent's bytes); Cases.Verify checks every decode
  arm's graph of it against the incumbent's bytes. gen/counts.txt / counts-nounk.txt re-committed:
  only the six P7.1 core-ffi decode rows changed (push rev 3 -> 7). P7.1 decode figures before
  this unit (s9 and earlier) are of the contiguous form.
- **Checks** (`s10/checks/checks.log`, gen/s10_checks.sh at f2797456's tree): generators current;
  layout by name both ways + section 10, both builds, shapes and corpus cores (the plant fails);
  Cases.Verify both builds (4,525 pre-timing checks in the full build); `--verify-fsm` both builds:
  114 inputs (16 shapes, 6 content-set rows, 92 U-* rows), push = pull = FSM graphs (managed
  re-encoding, retained form, and the read pass) on 228 / 114 input x mode rows, FSM calls = the
  Rust slice's events (`logs/rust/opt/d23-fsm/checks/events-counting.txt`) on all 342, reverse
  calls FSM = pull; malformed variants (truncations and byte flips at 48 evenly spaced positions
  per input, ASCII set and U-* rows): 20,676 / 10,338 decodes per family, 18,626 / 9,313 refused by
  pull with the FSM's code equal on every one, 2,050 / 1,025 accepted by all with equal graphs,
  push's code = pull's on all; corpus with the FSM arms: ffi-fsm-drop and ffi-fsm-retain pass 680,
  fail 0, both builds (CORPUS PASSES on all 6 / 3 arms); default corpus and --unk-controls pass;
  four plants in the generated consumer caught (token ignored, a run's last element lost, the root
  group not applied, an error read as the end). The counts check failed on its first run (the
  P7.1 change above); re-committed and re-run identical (`s10/checks/counts-recommit.log`).
  `s10/checks-v0/`: verify-fsm on the first contract (0/1 return), superseded.
- **Counts** (`s10/checks/counts-s10.txt`, counting build, the eight-arm grid; and
  `events-full.txt` / `events-nounk.txt`): per decode, FSM fwd = 1 reset + its calls (= events):
  P1.1 2, P1.2 8, P1.3 3, P2.2 3,501, P2.3 876, P2.4 561, P6.1 1,401, P7.1 7, P5.1 1; pull fwd =
  reset + parse + ak_bdr_ptr; push fwd = reset + decode, its reverse calls = the events.
- **Eight-arm decode-read table** (`s10/ab-s10/table.md`, times only; spreads, B/op, gen0 and
  minflt in `reference.md`): campaign harness (BDN per-case children, core-grid settings, client
  CPUs 0,1, warm-up 25 x 40 ms, 6 rounds x 40 ms), full build, 2 reps (384 s, 381 s), quiet before
  each (load1 0.07 / 0.48). Allocation per op is identical across push, pull and FSM on every row.

- **s11 (FSM fixes A and B in the core; JOURNAL 80):** core 081de788 (fix A 55c2771c: packed
  bodies in a tight loop; fix B 01c73821: the sub-slice reader; fix C withdrawn). generate.py
  rewrote nothing (`s11/generate.log`); cores rebuilt (`s11/build-core.log`, target-core
  7b8ed0b888940ab7); gen/s10_checks.sh PASSED (`s11/checks/`; counts and FSM event files
  identical to s10's). Eight-arm table at 37697a69, 2 reps (381 s, 380 s), quiet before each:
  `s11/ab-s11/table.md` (times only), `reference.md`, and `fsm-above.txt` (rows where the FSM's
  lowest per-rep median exceeds the highest per-rep median of both push and pull in the same
  mode, s10's list beside it).

## Optimisation pass (2026-10-04; JOURNAL 74; container instrumentation, NOT gated)

- **Steps** (owner decisions; each its own commit, checks and A/B log):
  1. D1 (`6883426d`, `48b3e4a5`): core-ffi and host-gen encode rows of the core grid at the form
     the CORE's transport receives (`encode-core-hot`, enc_end transport-core); the Grpc.Net frame
     form a labelled extra. `logs/csharp/opt/s1/`.
  2. a1 (`4c4e0496`): native string staging keeps its blocks, grows geometrically, commits only
     the bytes written. `s2/`.
  3. a2 (`49839a04`): push decode without ak_dec_err_reset / ak_dec_err (harness `hostfail`
     check), one GCHandle per instance, options rewritten on a mode change only, retain buffers
     from a per-context arena. `s3/`.
  4. D7 (`07bbafa1`): delivery cells <cell>.callback / .callback-inline / .queue for Bf,
     Cf-retain, Ef-retain (src/Rpc/Deliveries.cs), `akrpc --delivery-semantics` in the gate,
     gen/rpc-delivery-counts*.txt, `csw` per RPC row. `s4/`.
  5. D20 (`75d48a23`, `a79c14be`): core-ffi sets every utf8_skip bit (push vtables; pull
     through ak_dec_set_pvt_<Root>) and decodes strings with a strict UTF8Encoding(false, true):
     DecoderFallbackException -> AK_ERR_TRANSCODE (ak_fail in the callbacks, the pull replay's
     return). Gate control: the corpus's T-dec-* rows with a lossy decoder planted
     (AK_GATE_PLANT_LOSSY) must fail. Counts unchanged (the delivery-cell filter drops the
     setter's first-use call). `s5/`.
  5b. (`3ef0c330`): the push decode vtable and the pull pvt are built once per root in native
     memory (NativeMemory.AllocZeroed, `static readonly ak_dvt_<Root>*`), not a stack local per
     decode; a struct static field would live in a boxed object on the GC heap, which
     compaction may move, so its address cannot be handed to the core. Counts unchanged. `s5b/`.
  6. D21 (`44f4f304`, `df00f287`, `032b819a`): the core-ffi string encode path is chosen per
     process by AK_STR_ENC (cs_host.py Stage): **E0** (default, unchanged) .NET UTF-8 into the
     native staging + ak_tc_bytes; **E1** the string pinned (a GCHandle per string, freed when
     the codec call returns) + ak_tc_utf16 (simdutf); **E2** ak_str.data = 0x10000 + an index
     into a thread-static string table and `tc` = TcManaged ([UnmanagedCallersOnly]: UTF-8
     written by .NET into the core's buffer, grow when the worst case does not fit; one reverse
     call per non-empty string); **ETH:<n>** E1 for a string of at least n UTF-16 code units,
     E0 below. Byte identity: Cases.Verify runs every payload, content set and U-* row under
     E0, E1, E2 and ETH:16 in every BDN process (2716 / 1944 pre-timing checks); the corpus
     passes under E1, E2 and ETH:16 (`s6/corpus-strpaths.log`); a planted one-unit / one-byte
     short string (AK_GATE_PLANT_STR) fails the corpus under E1 and under E2 (424 rows each),
     and the gate now runs these. Counts: gen/counts.txt and counts-nounk.txt unchanged under
     E0; gen/counts-str-{e1,e2,eth256}[-nounk].txt: E2's rows are the base rows with `tc N`
     added to rev (641 full / 324 no-unknown rows with tc > 0); E1's and ETH's carry `pin N`
     (strings handed pinned, no boundary call). Length census (`s6/strlen-census.txt`): every
     string of the step-6 grid rows is under 48 UTF-16 code units, so ETH:256 pins none
     (`pin 0` on every row of both builds). Sweep (`s6/sweep/`, `s6/sweep-fine/`, one process,
     one string per encode): E1 under E0 from about 96 code units on wide (CJK) content and
     from about 224 on Latin-1; on ASCII and astral (surrogate-pair) content E1 is above E0 at
     every length swept (4 to 16 Ki); E2 is at or above E0 (0 to about 40 ns) up to 1 Ki on
     every content and below it at 16 Ki on every content (above E1 there on Latin-1 and wide).
     Grid rows (`s6/ab/compact.md`): E1 adds about 100 to 135 ns per string on the large
     payloads and 55 to 85 on the U-* rows, E2 about 13 to 29, ETH:256 within E0's spread. The threshold run as ETH is 256 (`s6/` JOURNAL 75 states how it was read
     from the sweep); it is a length test only and does not see content.
  7. D21 continued (`a0e3c779` .. this unit's last commit; JOURNAL 76): more string paths under
     AK_STR_ENC: **E3** (E2's callback, `fixed` + the core's additive `ak_utf16_to_utf8`,
     worst-case grow), **E3L** (sized by `ak_utf16_utf8_len` first), **E1R** (E1 without
     GCHandles: the fill marks the string, generated `fixed` frames pin it around the call that
     reads it: the root group in one nested `fixed` scope; an element chunk by bounded
     recursion, ONE FRAME PER ELEMENT with every singular string of the element and its inlined
     children; repeated strings and nested maps one frame per string / entry; chunks of K =
     AK_STR_PINK elements, default 64, refused above 256: worst frame 1,152 B, s7/frames/),
     **E1C** (the same chunks pinned by GCHandles freed per chunk: the attribution control),
     and `<MODE>:<n>[:na]` (the mode from n code units, E0 below; `na` sends ASCII strings to
     E0). An encode whose fill marked nothing runs no frame. Default unchanged (E0; the default
     encode measured within spread of 1818d178 after the new code was kept off its path).
     Checks: gen/s7_checks.sh (Cases.Verify under 14 path settings, `--verify-mt` 8 threads
     every path both builds, the corpus under each path both builds, pin stress with a
     compacting GC per chunk, planted controls: short string per path, early unpin, K 257;
     counts per path) passed (`s7/checks4.log`); gate levels 8 passed (`s7/quick-checks.log`).
     Two concurrency defects found by the RPC run and fixed (thread-static mark counters; E1C
     releasing only its own chunk's handles), JOURNAL 76. Kernel facts (s7/bench/): Cascade
     Lake class CPU without VBMI2, simdutf active kernel haswell (AVX2), .NET 8
     Vector512.IsHardwareAccelerated false. The threshold run as the final variant is E1R:128
     (JOURNAL 76 states how it was read). Logs `s7/`.
- **s8 (decode attribution, JOURNAL 77):** harness-only arms in one process (`BenchDotNet
  --decattr`; generated seams VtParse / VtNoop / VtParseValidate, DecodeVt, ParseOnly; RootOps
  DecFfiVt / DecFfiParse / DecFfiGraph / DecIncGraph / TouchF; Dec.SkipStrings for host-gen)
  and per-arm crossing counts (`--decattr-counts`). The split of the core-ffi decode per
  payload into core parse, crossings, managed build, strings (with the GC pause), pull, and
  host-gen's parse + build vs strings is in `logs/csharp/opt/s8/tables.md`. One product-path
  change: host-gen's Dec.StrReject tests Dec.SkipStrings (one static read per string, the twin
  of core-ffi's G.Str test); everything else is harness-only.
- **Step 9a (JOURNAL 78):** every push `Add_*` callback and every pull-replay ADD calls
  `EnsureCapacity(Count + n)` before appending its run (cs_host.py `_add_body`);
  `OrderedMap.EnsureCapacity` (the facade's map). Quick checks passed, counts unchanged.
- **Step 9b (JOURNAL 78):** `AK_BDN_S9=1` runs the six decode arms (Google.Protobuf retaining
  and discarding unknown fields, core push retain / drop, core pull retain / drop); the
  table is `logs/csharp/opt/s9/ab-9b/table.md` (times only), reference.md beside it.
- **Final combined run:** `logs/csharp/opt/s1-s4/` (gen/opt_bench.sh with OPT_DROP=1: the core
  grid plus the drop units and the RPC no-unknown client, labelled extras), tables.md.
- **Harness defect fixed on the way (`2610f847`):** under BDN's default toolchain the children of
  a no-unknown host were built as the FULL build (no /p:AkNounk=true): every "no-unknown" codec
  and RPC row timed under the default toolchain before it (the baselines of JOURNAL 73 included)
  ran the full build's code in drop mode. Every process now checks its build and core against
  the host's (BuildCheck) and the no-unknown job passes the property.
- **Counts regenerated:** gen/counts*.txt (step 1: encode-core rows added; step 3: fwd -2 per
  core-ffi decode), gen/rpc-counts*.txt (step 3), gen/rpc-delivery-counts*.txt (step 4, new);
  steps 5, 5b, 6, 7: unchanged; steps 6 and 7 add gen/counts-str-*.txt (per string path:
  e1, e2, eth256, e3, e3l, e1r, e1c, e1r128, each with -nounk).
- **Quick checks:** `AK_GATE_LEVELS=8 AK_GATE_KEEP_CORE=1 gen/gate.sh` (net8.0 only, cores not
  rebuilt; prints "CHECKS PASSED ... NOT the gate"). The full gate ran again at D24 (1a5ccaea, passed); before that it had not been run since
  step 1 (owner: no gate yet). Step 6's quick checks ran at `44f4f304`; the gate's new D21 lines
  (corpus under E1/E2, the planted controls, the per-path counts) were added after and run by
  hand with the same commands (`s6/corpus-strpaths.log`, the count files), not yet through
  gate.sh itself.
- **Harness:** gen/opt_ab.sh and gen/opt_ab_rpc.sh take per-variant environment
  (`NAME=DIR@K=V,...`) and wait for a quiet machine (load1 < 0.5, no other process above 10 %
  CPU, the RPC server excluded) before every timed process; `BenchDotNet --strsweep` (one-process
  length x content sweep of the string paths).

## Optimisation baseline (2026-10-04; JOURNAL 73; container instrumentation, NOT gated)

- **Where:** `logs/csharp/opt/baseline/` (the valid run: header.txt, tables.md, codec.tsv,
  rpc.tsv, the raw jsonl and BDN logs, timing.txt), at `24a9294`. Driver `gen/opt_bench.sh`
  (the core grid as run_campaign.sh runs it: BDN default toolchain, one child per case, merged
  runs; codec 25 x 40 ms warm-up, 6 rounds of 40 ms; RPC 10 x 100 ms warm-up, 6 rounds of
  100 ms; server warm-up 500; MemoryDiagnoser on through `AK_BDN_MEMORY=1`), tables by
  `gen/opt_tables.py`. Client CPUs 0,1, server 2,3, AK_WORKERS 8. Nothing of the core grid
  dropped; benchmark wall 12.4 min (codec 474 s, RPC 271 s), over the 5-10 min asked because each
  child needs ~1 s of warm-up to reach tier 1 (below). Every codec process ran Cases.Verify;
  every RPC call checked; 0 failed cases.
- **VOID for the managed arms:** `logs/csharp/opt/baseline-1cpu-VOID/` (client on ONE CPU, 4 x 40 ms
  warm-up): .NET 8 delays tier-up 10x on a one-CPU affinity mask, and those rows measured tier-0
  code. Kept, marked in its header, jsonl and tables.md.
- **Tier check:** `logs/csharp/opt/tier-check/` (gen/tier_check.sh, gen/tier_table.py): on 1 CPU
  managed rows 3-7x slower than with DOTNET_TC_CallCountingDelayMs=0 or TieredCompilation=0; on 2
  CPUs with 4 x 40 ms they tier up during the actual stage; 10 x 100 ms and 25 x 40 ms settle.
- **Guard:** every timed .NET process (BDN host and each child, both suites) reads its affinity mask
  and refuses one CPU unless AK_ALLOW_SINGLE_CPU=1 (CpuGuard in src/BenchDotNet/CpuClock.cs);
  `cpus_affinity`, `cpus_runtime`, `single_cpu_override` on each case's first row; the header
  states it. run_campaign.sh's smoke defaults are now client 0,1 / server 2,3.
- **Exploration switch:** `AK_BDN_MEMORY=1` adds BDN's MemoryDiagnoser (one extra workload
  iteration after the actual stage, outside the job's clock; `mem_alloc_bytes_per_op`, `mem_gen`,
  `mem_ops` on each case's first row). Off in the campaign.
- **Toolchain in this container:** .NET from Ubuntu noble-updates (`dotnet-sdk-8.0`
  8.0.131-0ubuntu1~24.04.1: SDK 8.0.131, runtime 8.0.31); dotnet-install's host is refused by the proxy.

## Gate (D18): clean checkout, one per h2 variant

`logs/csharp/wp13-core-gate-stock.log` and `wp13-core-gate-h2-batch.log`: **GATE PASSED** for
each at `ebbf1f6`, run by the runner from a fresh worktree (one gate per variant in one tree);
net8.0 and net6.0, both builds; 30 planted controls failing as required in each. The gate runs
with AK_CAMPAIGN_GRID unset, so it checks the full grid; `gen/build_core.sh` now also builds
ArmoniK.Api.Client and ArmoniK.Api.Common from packages/csharp at HEAD (cell A's channel).
Before D18: `wp13-gate-stock.log`, `wp13-gate-h2-batch.log` at `ea02da5`.

**Smoke, core grid** (`logs/csharp/campaign/wp13-core-smoke/`, stripped; grouped switch on, the
smoke default; container instrumentation): codec, 5 units, 245 rows, 0 failed; rpc over the
`armonik` configuration: stock 4 units, 32 rows (A, Bf + B a+read, Cf-retain + C-retain a+read,
Ef-retain + E-retain a+read at k = 1 and 8), h2-batch Cf-retain 4 rows, pinned A and Cf-retain
4 rows (`alloc: pinned`), 0 failed; A: one socket to the server after every case, TCP_NODELAY
on, SO_KEEPALIVE and SO_REUSEPORT 0.

## WP13 (FIX-PLAN, 2026-10-03)

- **TCP (D10, req 17 as amended).** Every timed cell runs over TCP 127.0.0.1 against the shared
  server's TCP listener (`AK_SERVER_TCP=0`; serve.sh prints `tcp 127.0.0.1:PORT`), in the runner
  and in the gate's RPC steps (`--sock tcp:127.0.0.1:PORT`). Nagle off: Grpc.Net `Socket.NoDelay`
  in the connect callback; the core `ak_client_opts.tcp_nagle = 0`. Read back in each case's
  setup after one untimed call (`src/Rpc/NoDelay.cs`: every ESTABLISHED socket of the process
  to the port, getsockopt TCP_NODELAY; none found or one without it fails the case) and in the
  upload check. The listener runs the PINNED server configuration only, so `shipped` and
  `pinned` differ on the client side only (stated in the runner and unit headers). Control
  `AK_CAMPAIGN_PLANT=nagle`: in the gate (upload check) and in `--plant` (A and B).
- **Client CPU (req 21 as amended).** `cpu_ns` = perf task-clock of the whole process (one
  perf_event_open SOFTWARE/TASK_CLOCK counter per thread, new threads picked up at each read),
  `proc_cpu_ns` = CLOCK_PROCESS_CPUTIME_ID beside it, both per BDN iteration at the same
  boundaries; `client_softirq_ticks` / `client_irq_ticks` from /proc/stat over AK_CPU_CLIENT
  per case (round-1 row). Both toolchains.
- **Pools (D8, D14).** AK_WORKERS (campaign.machine, default 8): the core runtime
  (`ak_runtime_new`), the .NET thread pool worker min and max, the server's tokio workers
  (AK_SERVER_THREADS defaults to it). Caller threads = the in-flight level k (not a worker
  pool); grpc-core not used. In every header.
- **h2 (D11 as amended).** Stock and h2-batch cores built and gated (`AK_H2=stock|h2-batch
  gen/gate.sh`); the runner loops the rpc suite over `AK_H2_VARIANTS` (default both), one gate
  each (`gate.log`, `gate.h2-batch.log`); every akrpc process reads the loaded core's h2, puts
  `h2` on every row and aborts if it differs from AK_H2. Codec rows carry `h2: stock`.
- **D9 / req 25 as amended (ad1a15be5).** The rule covers this slice: the shared core's buffers
  and transport allocate through glibc malloc inside the .NET process (no shim of its own;
  managed objects are on the GC heap). Runner switch `AK_CAMPAIGN_ALLOC`: `default` (the main figures,
  GLIBC_TUNABLES unset, as production) or `pinned` (the labelled diagnostic pass,
  GLIBC_TUNABLES with trim_threshold 256 MiB and mmap_threshold 32 MiB; files suffixed
  `.alloc-pinned`), both suites. Every row carries `alloc` and `minflt` (the process's minor
  page faults over the iteration, read by the job's clock beside the CPU clock, both
  toolchains; per call = minflt / iters). Each process reads back the mode at start (one
  16 MiB malloc and mallinfo2's mmapped-block count: default "mmapped", pinned "heap"), states
  it in the header, and refuses to run if AK_CAMPAIGN_ALLOC disagrees with its GLIBC_TUNABLES or if the
  pinned readback is not "heap". The RPC server runs the default allocator in both passes.
- **Probe, no pre-grow (owner decisions 2026-10-03).** The probe runs once per process (the
  host's Main and the first GlobalSetup of each BDN child; later setups return) and keeps its
  16 MiB block mapped (never freed: freeing it raised glibc's dynamic mmap threshold, java
  2892e207b); every case's first row carries `alloc_probe` (the child's own under the default
  toolchain). There is NO heap pre-grow: it was built and then reverted by the owner, because
  a pre-grow on one thread cannot reach the other threads' malloc arenas (C++ found the first
  benchmark still faulting through the core's worker threads); BDN's warm-up runs the real
  call path on every thread, and `minflt` on every row shows whether it sufficed.
- **Runner settings for small tests:** `AK_RPC_TRANSPORTS`, `AK_RPC_BUILDS`, `AK_H2_VARIANTS`; the allocator pass `AK_CAMPAIGN_ALLOC`.

## Register H (WP6) and WP7

The register H findings for this slice (R-H2, R-H3, R-H6, R-H9, R-H11, R-H14, R-H15, R-H18,
R-H19, R-H22, R-H23) were confirmed and fixed in WP6; evidence and dispositions per finding in
JOURNAL 57. WP7's ten items and what each built are in JOURNAL 58 and in the checklist below.

**Rows C4 does not code-check.** In the conformance corpus: none; every one of its 146 reject
rows states a `reject.reason` and each maps to a code (malformed, truncated, depth or transcode,
per plan.py's DECODE RULES), so C4 checks the code on every reject row there. In the
oracle-probe manifest (`poc/rust/gen/probe_corpus.py`, run by the gate): the **3** reject rows
`P-field-maxplus1`, `P-field-2p32plus2`, `P-field-maxplus1-in-group`, which carry no `reject`
object, so no expected code exists to compare with. Their refusal is still required (an
acceptance or an exception fails the row), and the code each arm returned (-2) is printed in
the row's form, so the skip is visible, not silent; what is not checked is only a code the row
itself does not specify.

## Open defects

| # | Where | What |
|---|---|---|
| D4 | net48 | compiled only; no gate on .NET Framework (needs Windows); the core-ffi host half has no net48 form (needs delegate thunks rooted for the vtable's lifetime) |
| D42 | (closed, WP10) | the pre-campaign timing modes of `akrpc` (in-process server) are removed |
| D47 | (closed, 2610f847) | BDN default-toolchain children of the no-unknown host built as the full build; the baselines' no-unknown columns are the full build in drop mode (JOURNAL 74) |
| D46 | BenchDotNet JitTiers (grouped mode) | the JIT check counts only methods compiled inside a case's span and promoted later; code first compiled before the case (Cases.Verify runs every arm first) and never promoted is not seen: on one CPU, tier-0-speed rows passed `jit check: PASS` (JOURNAL 73, logs/csharp/opt/tier-check/1cpu-grouped.*). Not fixed. Under the default toolchain no tier readback exists (JOURNAL 64); the one-CPU guard and the warm-up length are what stand in for it there |
| D48 | (closed, D23) | every C# decode row of P7.1 decoded the incumbent's contiguous re-encoding instead of the committed interleaved vector; now Cases.DecodeWire (JOURNAL 79) |
| D45 | (closed, D18) | the per-unit `# build ...` header line said "Unix socket ... (req 17: UDS)" over TCP; rewritten with the transport line in ebbf1f6 |

## Campaign readiness (design/CAMPAIGN.md at 3210f28; section 10 checklist)

Assessed against CAMPAIGN.md as amended through 3210f28 (the owner's 2026-09-26 decisions,
R-H22 to R-H36). The runner is `poc/csharp/run_campaign.sh`. The codec suite runs under
BenchmarkDotNet (req 22a), and so does the RPC grid since WP9 (req 22a as amended 2026-09-27):
`akrpc bench`, one pinned process per cell, the launch's one server started and warmed by the
runner first. **The server is the Rust slice's tonic rpc_server** (FIX-PLAN WP10, CAMPAIGN req
13 as amended; poc/rust/SERVER.md): one process per launch through poc/rust/serve.sh, pinned
to AK_CPU_SERVER, its tokio worker count in its log; `shipped` and `pinned` are this client's
configuration against its two sockets (tonic's server defaults; 4 MiB windows, adaptive off). The calib suite is `akrpc campaign --suite calib`. Each suite runs both builds
(full and no-unknown) per launch, in an order alternated by launch.

**Toolchain (CAMPAIGN req 22a as amended e6c909630).** The campaign runs BDN's default
toolchain: a generated project and ONE CHILD PROCESS PER CASE, in both suites. Grouping every
case of a unit in one process (InProcessEmit) is the switch `AK_BDN_GROUPED=1`, on by default
under `--smoke` and for small exploration runs only; the header says which. Under the default
toolchain the process CPU per iteration comes from the child's own clock reads (written at its
GlobalCleanup, paired in the host and checked against BDN's wall measurement of each
iteration); the JIT tier is **not** read back in that mode (the listener sees the host only;
open, JOURNAL 64), so the codec suite's JIT check applies to grouped runs only.

**What the framework forces or what differs from the hand-written sampler (WP9), stated:**
- a runner call per unit: one unit = one cell (21 in the full build, 10 no-unknown, per transport),
  its cases = its directions, payloads and in-flight levels; three benchmark classes (RpcK1,
  RpcK8, RpcK16) because OperationsPerInvoke is an attribute constant; one invocation = one
  batch of k calls in flight, k operations (`iters` = calls, `invocations` = batches);
- the channels live for the process (opened before BDN starts, one channel per cell per
  benchmark process), no longer for the whole launch across cells;
- BDN picks the invocations per iteration (pilot, iteration time 100 ms campaign, 20 ms smoke;
  unroll factor 1) where the sampler had a fixed number of calls per sample;
- the warm-up is BDN's (jitting stage, pilot, 10 warm-up iterations campaign / 1 smoke), not a
  JIT-settled loop; the JIT tier is read back per case and reported (`hot_tier0`), not fatal
  for RPC cases; order: units a seeded shuffle per launch, cases a seeded shuffle per process.

**Custom code on top of BDN, each for a requirement** (WP9 addendum, bc7cf94b1): starting and
warming the shared server through serve.sh (req 13); CpuClock, the job's clock reading process CPU at each
iteration boundary (req 21: BDN has no CPU per iteration); the seeded IOrderer (req 22: BDN has
no random order); the JSON-lines exporters with every label, writing no sample when any case
failed (req 28, 18); the runner's discard of a failed launch's output (req 18/22a); the JIT tier
read back (req 24's "recorded"); in the codec suite, the two prime cases (BDN cases, not
exported): without them the first case of every process measured BDN's own first-touched
runtime helpers at tier 0 and the JIT check failed (JOURNAL 51, 62). The codec suite's
hand-written pre-warm loop, its settle wait and knobs are removed (JOURNAL 62).

| # | Requirement | Status |
|---|---|---|
| 1 | one machine, slices sequential | not applicable in the container: the machine is the owner's; the runner runs one measured process at a time |
| 2 | governor, turbo, SMT | not applicable in the container: set by the owner; recorded in every header (sysfs) |
| 3 | isolation | not applicable in the container: set by the owner; isolcpus/nohz_full and the cgroup cpuset recorded |
| 4 | three disjoint CPU sets, fixed sizes, thread counts | met (D8, D14: every pool at AK_WORKERS, see WP13): `AK_CPU_CLIENT` / `AK_CPU_SERVER` from the environment (ffi/campaign.sh exports ffi/campaign.machine's) or, run alone outside a smoke, read from ffi/campaign.machine; a set whose size is not `AK_SET_SIZE` is refused; `taskset` per process; the header names the source. Worker thread counts in every header: .NET thread pool min/max and current, caller threads, the core runtime's workers (the codec suite creates none), the server's |
| 5 | floors gated for correctness | net6.0: met (the gate, both builds). .NET Framework 4.8: **not met**: compiled only, needs a Windows machine (owner decision: provide one, or record that the floor was not run) |
| 6 | build flags printed | met: Release, net8.0, core features (init-guard on), shared `libak_core.so`, build variant, JIT and GC settings, in every header; the core's cargo profile (`lto = false`) is stated here, not printed |
| 7 | payloads | met as amended: 16 payloads; Latin-1 and wide on P1.2, P2.2 and P2.4 (R-H26); the 92 accepted, non-disputed U-* rows at the shapes core's 7 ABI roots through the timed shapes core in encode (`encode-hot`: the arm's own decode of the row, untimed, re-encoded), decode and decode-read, every arm including incumbent-best (R-H27); decode-reencode a labelled extra |
| 8 | arms | met: incumbent-prod (Grpc.Tools' marshaller shape: CalculateSize + WriteTo(IBufferWriter), ParseFrom(ReadOnlySequence)), incumbent-best (WriteTo(IBufferWriter) / ParseFrom(ReadOnlySpan)), core-ffi (the FSM since D24, 2026-10-09; push as `core-ffi-push`, pull as `core-ffi-pull`, labelled extras), host-gen |
| 9 | directions | met: encode (4 variants, req 11), decode, decode-read (a generated visitor reads every field) |
| 10 | unknown fields, three modes | met: core-ffi and host-gen in retain and drop (full build; `Codec` and `CodecRetain`) and no-unknown (its own build and core, no facade member, R-H22); core-ffi-pull in drop and no-unknown; incumbent default (retains); `unknown_mode` and `build` on every sample |
| 11 | serialised once per iteration; encode variants | met as amended (R-H29): `encode` (pool + reused buffer), `encode-hot` (one graph + reused buffer), `encode-transport` (pool + the Grpc.Net form), `encode-transport-hot`; rows carry `enc_end`, `enc_input`, `pool_graphs`, `pool_bytes`. Reused buffer: the incumbent's BufWriter, host-gen's Enc, the core's encode buffer. Transport form: the serializer cells A, F, D run (shared code) into a frame built as Grpc.Net.Client 2.71's GrpcCallSerializationContext builds it (checked by reflection); on the core's transport (C, E) the form is the buffer row, stated; incumbent-best has no transport row. Pool: retained heap >= 2 x AK_LLC_BYTES (13.75 MB default), measured and topped up; a hot input is a pool of one (same per-call step); graph construction always in the case's setup. Google.Protobuf keeps no size memo |
| 12 | cells A-F, modes | met as amended (R-H35): full client A, B, C-retain, C-drop, D-retain, D-drop, E-retain, E-drop, F-retain, F-drop (+ labelled B/C callback and queue rows); no-unknown client A, B, C-nounk, D-nounk, E-nounk, F-nounk |
| 13 | server: separate, pre-serialised, one per launch, warmed; one channel per cell | met as amended (WP10, WP13): the Rust slice's tonic rpc_server through poc/rust/serve.sh, one process per launch pinned to AK_CPU_SERVER, AK_SERVER_THREADS = AK_WORKERS, its TCP listener serving every h2 variant, build and client configuration; warmed by serve.sh warm (2,000 campaign / 100 smoke checked calls per direction, on its sockets and its TCP listener) before any client; one channel per cell per benchmark process; P2.2 pre-serialised |
| 14 | directions a, a+read, b, c, d | met as amended (R-H36; 2026-09-27): a and a+read, b; c = unary upload of P5.3 and P5.4; d = the streamed upload (M5 messages of 2 MiB, ids on the first only, 4 MiB and 16 MiB, the server checking the count on every call, count and SHA-256 once per cell before timing), every cell and mode, at 1 and 8 in flight; B/C/E through the core's client streaming (ak_call_open, ak_call_send / ak_call_send_enc, ak_call_recv), A/D/F through Grpc.Net's AsyncClientStreamingCall; the core's framed send path beside its reference as Bf, Cf-*, Ef-* on b, c and d (a has an empty request). C runs the MOVE path (ak_call_unary_enc on b and c, ak_call_send_enc on d), its copy path kept as the labelled extra Cc-*; D copies (Grpc.Net's serializer can only write into the call's own buffer, so ak_enc_take_owned would not remove the copy; stated); ak_call_opts not used |
| 15 | 1/8/16 in flight | met |
| 16 | delivery | met as amended (R-H30): B, C, E the core's blocking call on caller threads created before the warm-up; A, D, F Grpc.Net's idiomatic `await CallInvoker.AsyncUnaryCall` (as Grpc.Tools' generated client does), k in flight = k async loops on the thread pool, stated in the header; callback and queue rows labelled extras, awaited |
| 17 | shipped and pinned, TCP 127.0.0.1 | met as amended (D10, WP13): TCP 127.0.0.1 against the server's TCP listener, which runs the pinned server configuration only; shipped and pinned are the client's configuration (shipped: Grpc.Net DisableDynamicWindowSizing and no window, the core's windows at 0; pinned: 4 MiB windows on both client transports); Nagle off on every client socket, read back per case |
| 18 | every call checked, abort | met: status (a non-OK gRPC status is AK_ERR_RPC_STATUS on the core's transport, an RpcException on Grpc.Net) and length or count on every call, the server warm-up's included; a retained decode that leaves a buffer undelivered fails its call; the `--plant` controls, per build and transport, a wrong expected length on a, c and d and a wrong SHA-256 on d, each on A, B, Bf and D one cell at a time, all abort with no sample |
| 19 | crossing counts gate | met as amended (R-H31, 2026-09-26 geometric grow): every exported entry point the timed code calls, counted by name in a counting build, resets included and placed (one per decode, before it: decision 11 rule 7 as amended), retain with no pre-placed buffer and the timed build's geometric grow; per codec case (`gen/counts*.txt`) and per call of the RPC cells (`gen/rpc-counts*.txt`, B to E and the framed twins, A and F listed, directions a to d); gated in step 9 with a must-differ control (exact-size grow). The R5 counts (`gen/crossings*.txt`) are gated too and checked before calib |
| 20 | crossing cost fwd/rev, perf stat | **not met here**: calib has a forward row (ak_noop) and a forward-and-reverse row; `perf stat` runs when installed and is not installed in this container (owner: install perf on the campaign machine) |
| 21 | CPU is process CPU per round | met as amended (R-H25): codec, CLOCK_PROCESS_CPUTIME_ID per BDN iteration (the job's clock, read at the same iteration boundaries as the wall time; a case without one value per iteration fails); rpc (as amended, WP13), perf task-clock of the whole client process per BDN iteration (`cpu_ns`), CLOCK_PROCESS_CPUTIME_ID beside it (`proc_cpu_ns`), softirq and irq ticks on the CLIENT CPUs per case; calib (the crossing benchmark, req 20) keeps CLOCK_THREAD_CPUTIME_ID of its one loop thread |
| 22 | order randomised where the framework allows | met: codec, the unit order of a launch and the case order in each BDN process are seeded shuffles, seeds in the headers; builds alternate by launch; rpc, the cell order of every round a seeded shuffle; transports and builds alternated by launch |
| 22a | benchmark engine | met as amended 2026-09-27: BenchmarkDotNet for the codec suite and the RPC grid (InProcessEmit, pinned by the runner, StopOnFirstError for RPC, raw measurements exported, warm-up and tier recorded; what the framework forces and the custom pieces are listed above) |
| 23 | 5 rounds x 3 launches | met (defaults) |
| 24 | warm-up stated, identical; GC/JIT defaults stated; every warm-up a runner parameter | met (amended 85cfd4826): every warm-up is a runner parameter with the campaign default in the header and a short smoke default (run_campaign.sh's AK_RPC_WARM_*, AK_RPC_SERVER_WARM, AK_BDN_WARMUP / _ROUNDS / _ITERATION_MS / _PREWARM_*; JOURNAL 61). codec, per BDN process a pre-warm to JIT quiescence (the job's clock included) and 2 unexported prime cases, then per case BDN's jitting, pilot and a fixed warm-up count; the JIT tier read back per case, `jit check: FAIL` fails the unit. rpc: warm-up rounds of 64 calls per cell, direction and level until a round compiles nothing (at most 10); `jit_in_window` per sample. GC and JIT between blocks at the framework defaults, stated Req 24 as amended 8c02e7c58 (>= 20 calls per calling thread before the first measured value): met for B, C, E and their twins at the campaign default, unchanged (10 warm-up iterations x BDN's minimum 4 invocations x 1 call per caller thread per invocation = >= 40, plus 1 jitting and >= 4 pilot invocations; the caller threads are created in the case's process before BDN's first stage; checked in BDN child mode on d/16 MiB at k = 8: 45 per thread, `logs/csharp/wp13-req24-warmup-count.log`), stated per unit in the `# warm-up/thread` header line; A, D, F and the callback/queue extras: >= 40 calls per async loop on the fixed thread pool, the per-thread count not controlled (stated) |
| 25 | allocator/GC warm, GC stated | met as amended 2026-10-03 (D9): warm-up per arm, workstation concurrent GC stated, GC counts and pause per BDN case (summary row); main figures on glibc's default allocator, `AK_CAMPAIGN_ALLOC=pinned` the labelled GLIBC_TUNABLES pass, `alloc` and `minflt` on every row, the mode read back per process (see WP13) |
| 26 | correctness before timing | met: the runner requires the gate passed at identical content (both builds, counts included); every BDN process re-checks byte identity of every encode arm and variant (transport frames and pooled graphs included) and every U-* row's encode and re-encode forms before timing |
| 27 | header | met: commit (dirty tree refused), machine, CPU sets and their source, runtime and incumbent versions, build flags and variant, core features, transport, threads, warm-up and repeats |
| 28 | JSON lines, raw | met: one line per BDN iteration and per rpc/calib sample, section 7's fields plus `build` and the encode-variant fields; the per-case BDN summary row carries `row: case-summary` and no `cpu_ns`/`wall_ns` |
| 29 | logs in ffi/logs/csharp/campaign/ | met |
| 30 | summaries | not applicable: this slice produces none; any ratio is to come from per-launch medians (stated in every header) |
| 31 | runner interface | met for the slice; the top-level `ffi/campaign.sh` is the aggregating session's |
| 32 | smoke run | see **Smoke** below |

**Smoke** (the owner's small-test rule; figures stripped; container instrumentation):
- `logs/csharp/campaign/wp13-smoke/` at `ea02da5`, through the runner from the gate's worktree,
  grouped switch on (the smoke default): TCP 127.0.0.1, client configuration `pinned` only,
  full build only (AK_RPC_TRANSPORTS=pinned, AK_RPC_BUILDS=full), both h2 variants, one launch,
  BDN 1 round, 1 warm-up, 20 ms iterations; the Rust server started with AK_SERVER_TCP=0 and 8
  workers and warmed (100) by serve.sh: per variant all 21 units, 283 samples, 0 failed cases;
  every row `net: tcp`, `h2` equal to the variant, with `cpu_ns` and `proc_cpu_ns`, and
  `client_softirq_ticks`. `plant-controls.log` (stock, pinned, full): 21 controls (WP10's 19 and
  Nagle on, on A and B), every one aborting with 0 samples. The raw .jsonl and .bdn.log are not
  committed (logs/PURGED.md); `*.stripped.log` keep every header line and per-cell counts.
- Not run in the smoke: `shipped`, the no-unknown RPC client, calib, the codec suite, the
  default toolchain (checked on one RPC unit, Bf, with task-clock: 10 samples, JOURNAL 65).
- `logs/csharp/campaign/wp10b-smoke/` at `d1a3a3b`, through the runner, with the grouped switch
  on (the smoke default): full build, `shipped` only (AK_RPC_TRANSPORTS=shipped,
  AK_RPC_BUILDS=full), one launch, BDN 1 round, 1 warm-up, 2 ms iterations; the Rust server
  started and warmed (100) by serve.sh: all 21 units, 283 samples, 0 failed cases; `plant/`: 19
  controls (wrong length on a, c, d and wrong SHA-256 on d, on A, B, Bf, C-drop, D-drop), every
  one aborting with 0 samples.
- `logs/csharp/campaign/wp10-smoke/` at `76ac71e` (WP10 before the switch): the same, the same
  result.
- The default toolchain (one child process per case) was checked on one RPC unit and one codec
  unit (JOURNAL 64), not in these smokes. Not run: the pinned transport, the no-unknown RPC
  client, calib, the full codec suite.

**Engine cost, container instrumentation** (`logs/csharp/bdn-default-job-unit/`, before WP7):
one unit of 336 cases at the default BDN job (10 warm-up, 5 x 100 ms) ran 17 to 18 minutes.
WP7 roughly triples the encode cases (four variants, U-* encode) and adds pool setups; the
campaign's codec suite is correspondingly longer.

## Campaign grid (D18, CAMPAIGN section 4.0 as amended b58543f7b)

`AK_CAMPAIGN_GRID=core|full` (runner default `core`; the processes read it, unset = full, so
the gate, which the runner runs with it unset, checks the full grid unchanged).
- **codec, core:** units incumbent-prod:default, core-ffi:retain (FSM decode since D24), host-gen:retain (full build)
  and core-ffi:no-unknown, host-gen:no-unknown (no-unknown build); directions
  encode-transport-hot (end state ii, one hot graph) and decode-read; the 16 shapes (P7.1
  decode only), Latin-1 and wide on P2.2 only; the 7 named U-* rows, encode-transport-hot and
  decode-read. 49 cases per unit, 245 per launch (+2 primes per unit). The U-* rows'
  transport form is new: its byte identity is checked before timing in every process
  (Cases.Verify, all arms, both builds), and its crossing counts were taken with the counting
  build (`logs/csharp/wp13-core-grid-counts/`: 49 core-ffi cases per build, 42 identical to
  the gated `gen/counts*.txt` rows, the 7 new U-* transport rows equal to their gated
  encode-hot rows); those 7 rows per build are not in the gate's committed count files.
- **rpc, core:** units A, Bf, Cf-retain, Ef-retain (full build), a+read and b (P2.2), c at
  P5.4, d at 16 MiB, k = 1 and 8 (8 cases per unit). The framed cells' a+read is the
  reference cell's (B, C-retain, E-retain; a has an empty request, so the send path is the
  same), run in the framed cell's process under its own name. Plus Cf-retain on h2-batch for c
  and d at k = 1 and 8 (4 cases), and the pinned-allocator subset, A and Cf-retain on c and d
  at k = 1 (4 cases, GLIBC_TUNABLES pinned for those processes only, files `.alloc-pinned`).
  `AK_RPC_ONLY_DIRS` / `AK_RPC_ONLY_K` narrow a process; the runner sets them.
- **transport, core: `armonik`.** Cell A's channel is ArmoniK's: packages/csharp
  `GrpcChannelFactory.CreateChannel(new GrpcClient { Endpoint })`, called directly, every other
  option at its package default (HttpClientHandler in its logging DelegatingHandler, retry
  ServiceConfig 5 attempts 1 s / 5 s / 1.5 on Unavailable, Aborted, Unknown, DisposeHttpClient,
  ServicePoint settings, Grpc.Net's default message limits: receive 4 MiB). The assemblies are
  built from `git archive HEAD packages/csharp Protos` by `gen/build_core.sh` into
  `target-armonik-client/` (nothing written under packages/). The process-wide
  `Http2FlowControl.DisableDynamicWindowSizing` is NOT set under `armonik` (it is the worker
  channel provider's). Observed in the container: the system proxy bypasses the loopback
  endpoint; Nagle is off on A's socket by SocketsHttpHandler's own default; SO_KEEPALIVE and
  SO_REUSEPORT are 0 on the live socket (ArmoniK's ServicePoint settings do not reach .NET 8's
  SocketsHttpHandler); one HTTP/2 connection per cell at k = 1 and 8. The core cells keep the
  core's `shipped` client configuration (windows at the stack's defaults, adaptive off,
  tcp_nagle 0). Every case's first row: tcp_sockets_after, tcp_nodelay_after,
  so_keepalive_after, so_reuseport_after. The req-18 controls run on `shipped` (the Nagle plant
  reaches Grpc.Net's socket there).
- **full:** today's grid, with `shipped` and `pinned`, every extra.

## Campaign duration estimate, core grid (2026-10-03, computed, nothing run)

BDN's per-case child processes at the campaign defaults (3 launches, 5 rounds, 10 warm-up
iterations of 100 ms). Container figures, for sizing only: RPC per case from
`wp13-req24-warmup-count.log` (about 1.25 s per case of child start and setup; a fast case
about 2.8 s; c at P5.4, k = 8 about 4.2 s; d at 16 MiB, k = 8 about 11 s at BDN's
4-invocation floor; BDN's project build 26.7 s per unit process); codec 3.25 to 4.35 s per case
(2.0 to 3.1 s of BDN stages, hot inputs only, plus the 1.25 s child overhead).

| Item | Per launch | 3 launches |
|---|---|---|
| codec: 5 units, 245 cases + 10 primes | 255 x 3.25 to 4.35 s + 5 x 27 s build: 16 to 21 min | 48 to 62 min |
| rpc: 4 stock units x 8 cases, Cf h2-batch 4 cases, pinned A + Cf 4 cases: 40 cases, 7 unit processes | ~161 s of cases + 7 x 27 s build + server start and 2,000-call warm (~2 min): ~8 min | ~24 min |
| calib | < 1 min | ~3 min |
| **timed total** | | **~75 to 90 min with one BDN run per unit; ~60 to 75 min with the merged runs (about 5 min saved per launch, measured)** |
| gates (stock and h2-batch, before the run; reused while the content is unchanged) | | ~45 min |

**BDN runs merged (owner, 2026-10-03; done).** Under the core grid the runner makes one BDN run
per codec build (2 per launch) and one per RPC run kind (stock, h2-batch, pinned: 3 per launch),
5 instead of 12. Every case still runs in its own child process (default toolchain, same
warm-up, pilot and iteration settings); what differs per process (build, core library,
GLIBC_TUNABLES) stays a separate run. The RPC child builds only its case's cell from the case
key (the host builds the case list without opening a channel); each case's setup records the
cell, direction, payload and channels it ran, and the exporter refuses a row whose label differs
or whose process opened another cell's channel (control AK_CAMPAIGN_PLANT=cell: refused, no
sample). The full grid keeps one run per unit. Measured (container, `logs/csharp/wp13-merge-timing.log`,
default toolchain, short settings; the same rows checked before and after): RPC stock 4 runs
237 s against 1 run 123 s, pinned 2 runs 81 s against 1 run 43 s, codec 5 runs 381 s against 2 runs
228 s: about 5 min saved per launch (~15 min for 3 launches), mostly BDN's project builds
(26 to 37 s each) and the codec suite's 2 prime cases per run. Estimate above, after the merge:
~60 to 75 min timed. In the grouped (smoke) mode a merged run holds every cell's channels in one
process, so `tcp_sockets_after` counts them all there; in the campaign's child mode it is per cell.

## What is not measured or not established

- **No timing in this slice is a result.** Every figure is container instrumentation.
- s14: why only some .NET 8 processes take the thread-static slow path (the runtime function is
  unnamed: no libcoreclr symbols here; perf does not run); whether s13's BDN grid spread was it;
  E1R's structure alone (R2-R1 is net of E0's UTF-8 encode, which no rung isolates); the guard
  below about 5 ns per string; Latin-1 and wide steps below R3 (the stub's output is shorter).
- D23: the FSM with `[SuppressGCTransition]` in drop (legal there, not rendered); the FSM in the
  RPC grid; the no-unknown build's FSM timings; a C# FSM on net6.0 (net8.0 only checked); the
  cause of the FSM's higher figures on P6.1, P4.1 and U-wire-ListMetrics (not attributed); D23's
  checks run through gen/s10_checks.sh, not through gate.sh.
- Whether the RPC cells (cell A's Grpc.Net path in particular) are at tier 1 when measured: the
  tier check covered codec rows only (JOURNAL 73); the RPC warm-up is the campaign's 10 x 100 ms.
- Anything on .NET Framework 4.8 (compiled only); no floor runs the RPC suite or BDN.
- `perf stat` cycles and instructions (not installed here), so req 20's per-iteration counts.
- The JIT tier under the default toolchain (open since JOURNAL 64).
- The runner end to end with `AK_CAMPAIGN_ALLOC=pinned` (the switch was smoked on units directly, `wp13-d9-alloc-smoke/`); the server's allocator is not switched.
- Thread CPU time under BenchmarkDotNet (the campaign's figure is process CPU, req 21).
- Cell B's encode form (incumbent into a span for the core's transport) as a codec-suite row.
- U-* rows with a pool input or a transport end state (encode-hot only).
- A lossy-UTF-8 codec (the plan's alternative option) is not generated.
- D21 step 7: E1R and E1C at payload scale with LONG strings (no grid payload has a string
  of 48 units or more; the long-string figures are one-string sweeps); E1R's per-string cost
  is not split below the frame/guard/transcoder sum; the two E1R outliers of s7/ab-fixed
  (U-wire-DualResponse retain, ListMetrics retain) are not investigated; E2/E3 for the pull
  path; an AVX-512 simdutf kernel (the CPU lacks VBMI2); the ASCII split's scan beyond 1 Ki.
- D21: no grid payload carries a string of 48 code units or more, so the ETH arm's E1 side is
  exercised by the sweep and the corpus only, not by a timed grid row; E1 measured with many
  strings pinned at once costs 2 to 3 times its one-string sweep figure (JOURNAL 75), not
  explained. Not built: a UTF-16 copy into the native staging (the existing AK_UTF16=1 Stage
  form) as a no-pin E1 variant; a content-aware split (needs a scan of the string); E2 for
  pull (it is push encode only).
- Unknown fields inside a map entry, in either codec: the facade map has no bag.
- Decision 11's placement paths other than grow through this host: pre-allocated pools,
  in-place refill between deliveries, the oneof buffer move, AK_ERR_CAPACITY with no grow.
- Google.Protobuf's message-size limit (ABI v1 decision 8) in the managed codec.
- ABI v1 decisions 4, 6, 10, 12 on .NET; decision 13 (borrowed strings) bounded only by a
  no-string ceiling (`G.SkipStrings`).
- Server-streaming and bidirectional calls (reserved in the ABI, not built); `ak_call_opts`
  (deadline, metadata) is passed NULL; the callback/queue deliveries of a stream (not built).
- The framed send path on direction a (an empty request); D without a copy (not possible
  through Grpc.Net's serializer).
- `packages/csharp`'s own object model; ReadyToRun; GC under load; content sets beyond P1.2,
  P2.2 and P2.4.

## Next step

000000. s14: the aggregating session reads `logs/csharp/opt/s14/`; the measurement features (`tc-measure-generic`, `tc-measure-u16stub`, AK_STR_NOGUARD) stay default OFF / unset; the slow mode's runtime function is unnamed (no libcoreclr symbols here) and why only some processes enter it is open.
00000. s13: the aggregating session reads `logs/csharp/opt/s13-tc-scalar/`; the scalar features stay default OFF.
0000. D25: nothing open; [SuppressGCTransition] was tested (s12) and dropped by the owner; the record is `logs/csharp/opt/s12-sgt/`.
000. D24: the aggregating session reads `logs/csharp/opt/d24/` (gate logs, counts before/after);
   the push extra arm `core-ffi-push` exists in the full grid and the S9/S10 sets.
00. D23: the aggregating session reads `logs/csharp/opt/s10/` and `s11/` (checks, counts, the eight-arm
   table); nothing further assigned in this slice.
0. Optimisation pass: steps 1 to 7 (with 5b) are in; the aggregating session reads
   `logs/csharp/opt/s1-s4/` and the per-step logs (`s5/`, `s5b/`, `s6/`, `s7/`). The string path's
   default stays E0 until the owner decides; gate.sh's new D21 lines run with the next gate. The full gate (both h2 variants) last ran at 1a5ccaea (D24): passed. D46
   (the grouped-mode JIT check's blind spot) is open.
1. The aggregating session reads WP13 (JOURNAL 65) and pushes; this slice changes nothing further
   unless a finding in scope (ffi/CLAUDE.md, "Scope of findings") comes back.
2. The net48 gate on a Windows machine (D4), which first needs a net48 host half.
3. The campaign itself is the owner's: `run_campaign.sh` (through `ffi/campaign.sh`) on the
   campaign machine; it runs one gate per h2 variant and the rpc suite once per variant.

## Log index

| Log | What it establishes |
|---|---|
| `opt/s1/`, `opt/s2/`, `opt/s3/`, `opt/s4/` | the optimisation steps: net8 quick checks and narrowed A/B (codec `ab/`, RPC `ab-rpc/`, deliveries `s4/deliveries/`) |
| `opt/s5/`, `opt/s5b/` | D20 (utf8_skip all bits + strict host decode) and the static decode vtable: checks (`checks.log`, the lossy-decoder control) and decode-read A/B (`ab/`, before/after, retain/drop/no-unknown) |
| `opt/s6/` | D21 string encode paths: `sweep/`, `sweep-fine/` (one-process length x content sweep), `strlen-census.txt`, `checks.log` (quick checks at 44f4f304), `corpus-strpaths.log` (corpus under E1/E2/ETH:16 and the planted controls), `ab/` (codec encode-core-hot E0/E1/E2/ETH:256, table.md, compact.md), `ab-rpc/` (Cf-retain b, k 1 and 8, E0/ETH:256/E1) |
| `opt/s14/` | s14: dist/ (part 1 distribution, summary.md), dist-run.out, jit/ (JIT summaries sum-*, disassembly dis-* and per-method fast101/fast102/slow107/slow110, strace-*, gdb-* stack samples with perf-*.map, sw-* the sweeps they ran), grid-spread/ (P2.2 E1R retain, 6 hosts x default / TieredPGO=0); cores.log (first measurement cores; the stub core folded), cores2.log (rebuilt at c6e29dbe, fold check), checks-ladder.log, checks-rungs/ (each rung in the build), ladder-sweep/ (table.md, reference.md, tsv per process, cut/ the slow-mode processes set aside), grid/ (table.md, reference.md, jsonl, BDN logs, header), grid-INTERRUPTED/ (empty: no cell cut) |
| `opt/s13-tc-scalar/` | s13: cores.log, differential.log, vectorisation.txt, checks.log, sweep/ (DOTNET_TieredPGO=0; table.md, reference.md), sweep-default-jit-BIMODAL/ (kept), grid/ (table.md, reference.md, jsonl, BDN logs), grid-INTERRUPTED/ (not used) |
| `opt/d25/` | D25: checks.log and checks/ (gen/s10_checks.sh after removing the attributed path: PASSED, counts unchanged), rpc-counts.log (the four RPC count files reproduced identical) |
| `opt/s12-sgt/` | s12: checks.log and checks/ (gen/s12_checks.sh: plain and AK_FSM_SGT=1 runs of gen/s10_checks.sh, the SGT share), ab-s12/ (table.md, reference.md, sgt-vs-plain.md, jsonl, BDN logs, header), sgtbench/ (microbench and longest call per row, two processes per build; longest-call.md) |
| `opt/d24/` | D24: gate-stock.log and gate-h2-batch.log (the full gate at 1a5ccaea, clean worktree: PASSED), gate-stock-FAILED-ff1bf0c2.log (the first attempt: net48 build, delivery counts), counts/ (every count and crossings file before D24, and the counting core-ffi tables) |
| `opt/s11/` | the s10 table rerun on core 081de788 (FSM fixes A and B): generate.log (no C# change), build-core.log, checks/ (gen/s10_checks.sh PASSED), ab-s11/ (table.md, reference.md, fsm-above.txt, jsonl, BDN logs, header) |
| `opt/s10/` | D23: build-core.log (cores at 8030b7f9, superseded), build-core-op.log (cores at c2b95f62); checks/ (gen/s10_checks.sh: checks.log, verify-fsm-*.log, events-*.txt, corpus-*.log, counts-s10.txt, counts-recommit.log); checks-v0/ (verify-fsm on the first contract, superseded); ab-s10/ (eight arms, decode-read: table.md times only, reference.md spreads / B/op / gen0 / minflt, jsonl, BDN logs, header) |
| `opt/s9/` | step 9a: checks-9a.log, ab-9a/ (before/after, core push retain decode-read); step 9b: ab-9b/table.md (six arms, times only), reference.md (spreads, B/op, gen0, minflt), the jsonl and BDN logs |
| `opt/s8/` | decode attribution: tables.md (arms, split, allocation / GC / faults), *.census.txt (graph census), counts-full.txt / counts-nounk.txt (crossings per arm), full-r*/nounk-r* tsv and logs; run1-superseded/ (the first run, without hskip and the GC pause) |
| `opt/s7/` | D21 step 7: `checks*.log` (gen/s7_checks.sh; checks4 is the final), `quick-checks.log`, `verify-mt.log` (the concurrency defects before / after), `frames/` (E1R stack per frame), `bench/` (sweep over every path, kernel probe, CPU flags, vector-width and simdutf switches, pin microbench), `e0-overhead/` (default path vs 1818d178), `threshold/` (sweep, sweep2), `ab/` (E0pre E0 E1 E1C* E1R E3; *E1C defective), `ab-fixed/` (E0 E1C E1R E1R:128, corrected build), `ab-final/`, `ab-final2/` (E1R:128 before/after the inline test), `ab-rpc/` (Cf-retain b, E0 E1R:128 E1R); `ab-INTERRUPTED/`, `ab-rpc-FAILED-E1R/` kept, not used |
| `opt/s1-s4-decode-quiet/` | the quiet re-measure of decode-read after steps 1 to 4 (core rebuilt with D19, load checked per process) |
| `opt/s1-s4/` | the combined core-grid run after step 4 (with the drop and no-unknown RPC extras), tables.md |
| `opt/baseline/` | the optimisation pass's short baseline, core grid, client on 2 CPUs (not gated; tables.md, codec.tsv, rpc.tsv) |
| `opt/baseline-1cpu-VOID/` | the first baseline run, client on 1 CPU: VOID for the managed arms (tier 0), kept |
| `opt/tier-check/` | the tier-0 confirmation: 1 vs 2 CPUs, delay 0, tiering off, grouped, warm-up length, the guard's refusal |
| `wp13-core-grid-counts/` | the counting build over the core codec grid, both builds (see Campaign grid) |
| `campaign/wp13-core-smoke/`, `wp13-core-gate-stock.log`, `wp13-core-gate-h2-batch.log` | the core-grid smoke through the runner from a clean worktree at `ebbf1f6`, and its two gates |
| `wp13-merge-timing.log`, `campaign/wp13-merge-smoke/`, `wp13-merge-gate-stock.log`, `wp13-merge-gate-h2-batch.log` | the BDN merge: timings per launch before and after, the label control; the core-grid smoke after the merge and its two gates at `beae3d7` |
| `wp13-req24-warmup-count.log` | BDN's stage counts per case at the campaign warm-up, child mode, C-drop at k = 8 (req 24 as amended) |
| `wp13-alloc-probe-smoke/` | after the pre-grow's revert: C-drop per mode (grouped), Bf under the default toolchain (pinned, each child its own probe), a codec unit (default) |
| `wp13-pregrow-smoke/` | the glibc pre-grow, since reverted (JOURNAL 67, 68) |
| `wp13-d9-alloc-smoke/` | the allocator switch (D9 as amended): an RPC and a codec unit in both modes, the readback, a mismatch control, minflt under the default toolchain |
| `wp13-gate-stock.log`, `wp13-gate-h2-batch.log` | the clean-checkout gates of WP13 at `ea02da5`, one per h2 variant (see Gate) |
| `wp13-gate-h2-batch-FAILED-2f9ce48.log` | the second gate in one tree failing on the floor build before the fix (JOURNAL 65) |
| `campaign/wp13-smoke/` | the WP13 minimal smoke: TCP, pinned, full build, both h2 variants, stripped; the plant controls with Nagle on |
| `campaign/wp8c-warmup-knobs/` | the RPC client's warm-up knobs in smoke mode, one transport, full build (req 24 as amended) |
| `wp10b-gate.log`, `wp10-gate.log` | the clean-checkout gates after the toolchain switch (`d1a3a3b`) and WP10 (`76ac71e`) |
| `campaign/wp10b-smoke/`, `campaign/wp10-smoke/` | the minimal smokes against the Rust server, grouped, one transport, full build, with plant controls |
| `wp9-gate.log` | the clean-checkout gate of WP9 at `6a7c0cf`, both builds, net8.0 and net6.0 (see Gate) |
| `campaign/wp9-smoke/` | the minimal WP9 smoke: the RPC grid on BDN, one transport, full build; one codec unit (see Smoke) |
| `wp8b-gate.log` | the clean-checkout gate after the WP8 parity items, at `9114d6b`, both builds, net8.0 and net6.0 (see Gate) |
| `campaign/wp8b-smoke/` | the smoke after the WP8 parity items (see Smoke) |
| `wp8-gate.log` | the clean-checkout gate of WP8 at `d97ea52`, both builds, net8.0 and net6.0, counts and upload check included (see Gate) |
| `campaign/wp8-smoke/` | the WP8 smoke runs of every suite and the plant controls (see Smoke) |
| `wp7-gate.log` | the clean-checkout gate of WP7 at `c35bd22`, both builds, net8.0 and net6.0, counts included (see Gate) |
| `wp7-grpcnet-context-reflection.log` | what Grpc.Net.Client 2.71.0's internal serialization context calls (req 11's transport form) |
| `campaign/wp7-smoke/` | the WP7 smoke runs of every suite (see Smoke) |
| `wp6h-gate.log` | the clean-checkout gate after the register H fixes, at `b758b27` |
| `wp6s1-gate.log` | the clean-checkout gate of WP6 step 1, at `ee94026` |
| `wp5s10-gate.log` | the step-10 gate (no-unknown variant added as step 8), at `2410125` |
| `wp5s9-gate.log` | the step-9 gate (decision 11 port), at `8d2e7ac` |
| `gate-repro/` | the gate at 253f487 and ef00211, three runs each, each log under its own name: 6 of 6 passed (the once-seen failure at 253f487 did not reproduce; cause not found, JOURNAL 54) |
| `campaign/` | the smoke runs of `run_campaign.sh` and the runner's gate logs |
| `bdn-default-job-unit/` | a trial of one BDN unit at the default job, for the JIT check and the engine cost (instrumentation) |
| `wp5-tail-gate.log`, `wp5-step4-gate.log`, `wp5-step4-before-after.log`, `wp4-rd9-bytes-free.log`, `wp3-17-net6-floor-build.log`, `stage21-wp4-regate.log` | earlier gates and checks, superseded by the gates above |
| `stage1` ... `stage20`, `bdn-results/`, `calibration-rust-crossing.log` | pre-campaign stages; every timing in them is instrumentation and none is used. **Stages 18 and 19 were built against a core not on the branch (FIX-PLAN R-C9)**: relabelled in their headers, no figure from them is usable |
