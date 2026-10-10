# s15 core-ffi codec grid (the core grid core-ffi directions, encode-core-hot and decode-read; drop and retain; E0 and the FSM as today): process CPU per op (us), median over every round of every rep (3 BDN host processes per variant), new = D26 code, base = the code before D26 (same session, interleaved). CONTAINER INSTRUMENTATION.

| payload | content | dir | mode | new | base |
|---|---|---|---|---:|---:|
| P1.1 | ascii | encode-core-hot | drop | 0.769 | 0.678 |
| P1.1 | ascii | encode-core-hot | retain | 1.085 | 1.042 |
| P5.1 | ascii | encode-core-hot | drop | 0.102 | 0.105 |
| P5.1 | ascii | encode-core-hot | retain | 0.104 | 0.100 |
