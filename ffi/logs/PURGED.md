# Raw logs removed from history (owner, 2026-10-02)

The branch `claude/rust-slice-optimization-sy1f4n` was rewritten on 2026-10-02 to remove
large raw artefacts from the 136 commits it alone carries (the commits after
`rust/native-core-ffi-poc` at 3cbf2216). No other file changed. Paths in STATE, JOURNAL,
tables and findings that name these files now point at files that are not in the
repository; the tables, summaries and syscall counts derived from them stay.

| Removed | Pattern | Size (uncompressed blob total) |
|---|---|---|
| raw system-call traces | `*.strace.gz` (incl. `*.st.strace.gz`) | 325 MiB, 465 files |
| perf binary dumps | `*.data.gz`, `*.data.server.gz`, `*.server.data.gz` | 12 MiB, 68 files |
| raw data of the container-era C++ optimisation snapshots | `logs/cpp/opt/{baseline,ref,s1..s9,s8b}/*.{jsonl,gbench.json.gz,tsv}` | 121 MiB, 228 files |

Kept: every strace summary (`*.syscalls.txt`, `*.strace` `-c` tables, per-thread
`.out`/`.threads.json`), every perf report in text, every table and STATE row. The C++
snapshot tables can no longer be regenerated from their raw samples.

The originals survive only where a clone made before the rewrite still exists (the campaign
machine's checkout, unless reset).

# Second purge: the whole branch history (owner, 2026-10-03)

`claude/poc-adversarial-review-38g5y8` was rewritten on 2026-10-03, after merging the
optimisation branch at 90171425, to remove raw benchmark output and build artefacts from
every commit of its history (2,840 commits; the commits of `main` it builds on carry none of
these paths and are unchanged). The packed repository went from 99 MiB to 23 MiB, and
`ffi/logs` in a checkout from 128 MB to 53 MB.

| Removed | Pattern |
|---|---|
| raw per-sample timing output (every slice, every campaign, smoke and optimisation run) | `ffi/logs/**/*.jsonl` |
| raw tables and compressed dumps | `ffi/logs/**/*.tsv`, `ffi/logs/**/*.gz` |
| raw framework output | `ffi/logs/**/*.gbench.json`, `ffi/logs/**/*.bdn.log`, `ffi/logs/**/*.criterion.log`, `ffi/logs/**/*.console` |
| committed C++ build directories (object files, a binary; `gen/upb_ab.sh` rebuilds them) | `ffi/poc/cpp/build-upbclang/`, `ffi/poc/cpp/build-upbft/` |

Kept: every gate and check log (`*.log`), every summary, table and report in text (`*.txt`,
`*.md`, `*.out`, `*.err`, `*.perfstat`, `*.syscalls.txt`, patches), and every committed count
file. None of the removed files was an input to a gate or a harness. Paths in STATE, JOURNAL,
findings and tables that name a removed file now point at nothing; what was derived from it
stays. Every figure in the removed files was container or physical-probe instrumentation,
not a campaign result.
