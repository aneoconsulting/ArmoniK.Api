# s15 core-ffi codec grid (the core grid core-ffi directions, encode-core-hot and decode-read; drop and retain; E0 and the FSM as today): process CPU per op (us), median over every round of every rep (3 BDN host processes per variant), new = D26 code, base = the code before D26 (same session, interleaved). CONTAINER INSTRUMENTATION.

| payload | content | dir | mode | new | base |
|---|---|---|---|---:|---:|
| P2.2 | ascii | decode-read | drop | 1794 | 1661 |
| P2.2 | ascii | decode-read | retain | 1671 | 1674 |
| P2.2 | latin1 | decode-read | drop | 2112 | 2202 |
| P2.2 | latin1 | decode-read | retain | 2163 | 2205 |
| P2.2 | wide | decode-read | drop | 2838 | 2937 |
| P2.2 | wide | decode-read | retain | 2986 | 2856 |
