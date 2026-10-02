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
