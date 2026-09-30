# Foreign processes during the stability campaign (read from the machine facts in each runner.log)

Source: the `busiest_processes_pcpu_psr_comm` field of every `machine_start`, `machine_after_round N` and `machine_end`
line. It is `ps -eo pcpu,psr,comm --sort=-pcpu`, top 8, taken between two client processes. `pcpu` is ps's average
over the process's lifetime (not an instantaneous load); `psr` is the CPU the process last ran on.

**python3 and ps (both runs).** These are this slice's own `gen/machine_facts.py` (a python3 process a few
milliseconds old that runs `ps` itself, so ps reports it near 60-100 %). They ran between client processes, not
during a timed loop, and every one was on the OS set: run1 python3 on CPUs 0 (3 times), 9 (3), 10 (6), 19 (5); run2
python3 on 0 (10), 9 (4), 10 (3), 19 (6), ps once on 0.

**run1.** `codec_suite-dda` at 99.2 % on CPU 9 (machine_start, 19:05Z) and `codec_suite-129` at 99.3 % on CPU 19
(machine_after_round 1, 19:06:06Z) are NOT this slice's: the C++ slice has no binary of that name (it is the Rust
slice's codec benchmark), and nothing in gen/stability.sh starts one. `rpc_semantics` (9.8 %, CPU 19, after round 2)
is the Rust slice's semantics binary, not this slice's. `htop` (5-6 %, OS set) ran through round 9. So run1's first
rounds overlapped with another slice's work on the OS set; run1 also has the server re-pinned from round 11 on (see
tables-rounds1-10.md, tables-rounds11-14.md). run2 is the reference.

**run2.** No foreign process above 5 % in any of the 21 samples other than `.kwin_wayland-w` (the compositor, 9.1-9.2
% lifetime average, unchanged across the run). Its last CPU was in the SERVER set (15-19 range: 15, 16, 17, 18) in 18
of the 21 samples and 19 in 3: its threads are allowed on 0-19 (listed in every header as non-kernel threads allowed
on the measured CPUs, with sddm-helper and fusermount3). No codec_suite, rpc_semantics or build appears in run2's
samples.
