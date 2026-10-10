# s14 ladder, one-string sweep: ns per string, median of the per-process medians (n processes per cell in reference.md). CONTAINER INSTRUMENTATION.

R0 E0; R1 E0 through the generic transcoder path (identity copy); R2 E1R with the UTF-16 stub (bytes NOT checked); R2g R2 without the guard; R3 E1R (simdutf); R3g R3 without the guard. Steps: R1-R0 the generic path; R2-R1 E1R's frame, mark, patch and element calls against E0's staging (both through the generic path); R3-R2 the UTF-16 transcoder over the stub; R3-R3g and R2-R2g the guard. R2 copies `len` raw bytes: on Latin-1 its output is half the real UTF-8 (40 / 48 bytes for 80 / 96), so on those rows R2 also removes output bytes, not only the transcoder.

## DOTNET_TieredPGO=0

| content | units | R0 | R1 | R2 | R2g | R3 | R3g | R3-R0 | R1-R0 | R2-R1 | R3-R2 | R3-R3g | R2-R2g |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ascii | 40 | 84.78 | 86.3 | 106.4 | 104.4 | 123.7 | 115.5 | 38.9 | 1.525 | 20.08 | 17.3 | 8.15 | 2 |
| ascii | 48 | 82.65 | 84.75 | 104.7 | 104.1 | 120.5 | 115.9 | 37.87 | 2.1 | 19.92 | 15.85 | 4.6 | 0.575 |
| latin1 | 40 | 99.42 | 100.9 | 106.6 | 101.9 | 128.1 | 118.5 | 28.65 | 1.525 | 5.675 | 21.45 | 9.55 | 4.75 |
| latin1 | 48 | 104.2 | 104.6 | 103.7 | 103.2 | 125.5 | 117.1 | 21.32 | 0.425 | -0.875 | 21.77 | 8.4 | 0.500 |

## default JIT configuration, fast-mode processes

| content | units | R0 | R1 | R2 | R2g | R3 | R3g | R3-R0 | R1-R0 | R2-R1 | R3-R2 | R3-R3g | R2-R2g |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ascii | 40 | 77.48 | 81.58 | 93.52 | 92.68 | 104.1 | 105.7 | 26.65 | 4.1 | 11.95 | 10.6 | -1.6 | 0.850 |
| ascii | 48 | 76.75 | 79.92 | 92.8 | 92.85 | 105.7 | 104.1 | 28.93 | 3.175 | 12.88 | 12.88 | 1.625 | -0.050 |
| latin1 | 40 | 102.8 | 97.67 | 94.5 | 93.32 | 107.4 | 106.8 | 4.575 | -5.175 | -3.175 | 12.92 | 0.675 | 1.175 |
| latin1 | 48 | 109.4 | 101.7 | 91.4 | 91.85 | 107.1 | 106.7 | -2.3 | -7.65 | -10.32 | 15.67 | 0.375 | -0.450 |

## In-process differences (E1R - E0 inside each process, median over processes), ns per string

simd: R3-R0. noguard: R3g-R0 (NOGUARD also drops Go's end check on E0). stub: R2-R0. stubng: R2g-R0. generic: R3-R1. Derived: R1-R0 = simd - generic; R2-R1 = stub - (R1-R0); R3-R2 = simd - stub; guard = simd - noguard (and stub - stubng).

### DOTNET_TieredPGO=0

| content | units | simd R3-R0 | noguard R3g-R0 | generic R3-R1 | stub R2-R0 | stubng R2g-R0 | R1-R0 | R2-R1 | R3-R2 | guard (R3) | guard (R2) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ascii | 40 | 38.28 | 34.33 | 35.68 | 23.55 | 21.98 | 2.6 | 20.95 | 14.73 | 3.95 | 1.575 |
| ascii | 48 | 38.72 | 34.42 | 37.1 | 23.05 | 22.53 | 1.625 | 21.42 | 15.67 | 4.3 | 0.525 |
| latin1 | 40 | 22.6 | 22.5 | 23.75 | 9.85 | 5.475 | -1.15 | 11 | 12.75 | 0.100 | 4.375 |
| latin1 | 48 | 18.33 | 17.57 | 18.53 | 2.65 | 1.875 | -0.200 | 2.85 | 15.67 | 0.750 | 0.775 |

### default JIT configuration, fast-mode processes

| content | units | simd R3-R0 | noguard R3g-R0 | generic R3-R1 | stub R2-R0 | stubng R2g-R0 | R1-R0 | R2-R1 | R3-R2 | guard (R3) | guard (R2) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ascii | 40 | 26.73 | 26.32 | 25.23 | 16.08 | 14.42 | 1.5 | 14.58 | 10.65 | 0.400 | 1.65 |
| ascii | 48 | 28.8 | 26.55 | 25.8 | 15.38 | 15.45 | 3 | 12.38 | 13.42 | 2.25 | -0.075 |
| latin1 | 40 | 5.45 | 2.025 | 10.93 | -5.7 | -1.55 | -5.475 | -0.225 | 11.15 | 3.425 | -4.15 |
| latin1 | 48 | -3.6 | -3.55 | 8.2 | -14.95 | -5.5 | -11.8 | -3.15 | 11.35 | -0.050 | -9.45 |

