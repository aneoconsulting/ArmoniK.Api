# runtime probe: commit cd4edbb0 + UNCOMMITTED; 2026-09-28T20:50:53Z; Intel(R) Xeon(R) Processor @ 2.80GHz; client CPU 1, server CPUs 2,3 (4 workers), pinned; cells A,Df,Df-chan,Cf-cb; iterations 5 (variant order rotating), block order, no /proc; rounds 12 calls 8 warm 4; attribution pass rounds 0 calls 4 with /proc and the allocation shim; variants: base=main: h1=main:AK_HOST_WORKERS=1 c1=main:AK_CORE_WORKERS=1 b1=main:AK_HOST_WORKERS=1,AK_CORE_WORKERS=1
# run time 201 s

Runtimes as each variant's probe process recorded them:

- `base`: runtimes: host (cell-rt) mt2 (AK_HOST_WORKERS), core ak_runtime_new(2) (AK_CORE_WORKERS); Df-chan mpsc depth 1 (AK_CHAN_DEPTH); AK_CORE_CHAN_DEPTH=unset (honoured only by the patched core, which prints its depth on stderr)
- `h1`: runtimes: host (cell-rt) mt1 (AK_HOST_WORKERS), core ak_runtime_new(2) (AK_CORE_WORKERS); Df-chan mpsc depth 1 (AK_CHAN_DEPTH); AK_CORE_CHAN_DEPTH=unset (honoured only by the patched core, which prints its depth on stderr)
- `c1`: runtimes: host (cell-rt) mt2 (AK_HOST_WORKERS), core ak_runtime_new(1) (AK_CORE_WORKERS); Df-chan mpsc depth 1 (AK_CHAN_DEPTH); AK_CORE_CHAN_DEPTH=unset (honoured only by the patched core, which prints its depth on stderr)
- `b1`: runtimes: host (cell-rt) mt1 (AK_HOST_WORKERS), core ak_runtime_new(1) (AK_CORE_WORKERS); Df-chan mpsc depth 1 (AK_CHAN_DEPTH); AK_CORE_CHAN_DEPTH=unset (honoured only by the patched core, which prints its depth on stderr)

## 16MiB, k = 1: client CPU per call, ms: median [p10-p90] over all calls; below it the range of the per-iteration medians

| cell | base | h1 | c1 | b1 |
|---|---:|---:|---:|---:|
| A | 9.94 [8.55-11.68]<br>it 9.00-11.19 | 9.66 [8.34-11.04]<br>it 8.99-9.99 | 9.54 [8.42-11.16]<br>it 9.27-9.66 | 9.54 [8.19-11.14]<br>it 8.65-10.17 |
| Df-retain | 10.32 [9.08-12.30]<br>it 9.47-11.39 | 9.98 [8.81-11.54]<br>it 9.46-10.53 | 10.49 [9.30-12.12]<br>it 9.86-11.12 | 10.04 [8.89-11.51]<br>it 9.59-10.42 |
| Df-chan | 10.21 [9.24-11.67]<br>it 9.88-10.41 | 10.81 [9.50-12.64]<br>it 10.21-11.09 | 10.87 [9.57-12.40]<br>it 10.29-11.42 | 10.34 [9.17-11.94]<br>it 9.73-11.07 |
| Cf-cb-retain | 10.68 [9.26-12.59]<br>it 9.83-11.40 | 10.53 [9.17-12.25]<br>it 9.94-11.73 | 10.79 [9.42-12.53]<br>it 10.07-11.60 | 10.57 [9.32-12.27]<br>it 9.80-11.71 |

### 16MiB: variant minus `base`, ms per call, per iteration (the two processes of one iteration ran back to back); `x/n neg` = iterations with a negative difference

| cell | h1 | c1 | b1 |
|---|---:|---:|---:|
| A | -0.22 (-1.56..+0.57; 3/5 neg) | -0.53 (-1.69..+0.65; 4/5 neg) | -1.02 (-1.21..+0.99; 3/5 neg) |
| Df-retain | -0.89 (-0.98..+1.06; 4/5 neg) | -0.37 (-0.95..+1.65; 3/5 neg) | -0.79 (-1.23..+0.90; 3/5 neg) |
| Df-chan | +0.64 (-0.07..+1.21; 1/5 neg) | +0.28 (-0.04..+1.54; 1/5 neg) | -0.26 (-0.60..+1.19; 3/5 neg) |
| Cf-cb-retain | -0.40 (-1.09..+1.46; 3/5 neg) | +0.44 (-0.62..+1.19; 2/5 neg) | -0.24 (-1.13..+1.45; 3/5 neg) |

### 16MiB: in-process control: cell minus A of the SAME process, ms per call (per-iteration medians; median over iterations, range)

| cell | base | h1 | c1 | b1 |
|---|---:|---:|---:|---:|
| Df-retain | +0.47 (+0.20..+0.61) | +0.47 (-0.15..+0.96) | +0.97 (+0.31..+1.47) | +0.38 (+0.26..+0.96) |
| Df-chan | +0.46 (-0.78..+0.88) | +1.22 (+1.00..+1.52) | +1.19 (+0.74..+1.79) | +0.77 (-0.02..+1.22) |
| Cf-cb-retain | +1.19 (-1.36..+1.27) | +1.06 (+0.20..+2.16) | +1.30 (+0.80..+2.02) | +1.33 (-0.13..+1.80) |

### 16MiB: context switches and minor faults per call (getrusage, whole process; medians over rounds): voluntary / involuntary / minflt

| cell | base | h1 | c1 | b1 |
|---|---:|---:|---:|---:|
| A | 27 / 3 / 0 | 22 / 0 / 0 | 26 / 2 / 0 | 24 / 0 / 0 |
| Df-retain | 31 / 6 / 0 | 21 / 0 / 0 | 31 / 7 / 0 | 21 / 0 / 0 |
| Df-chan | 48 / 16 / 0 | 37 / 10 / 0 | 48 / 16 / 0 | 37 / 10 / 0 |
| Cf-cb-retain | 57 / 19 / 125 | 52 / 17 / 0 | 45 / 15 / 128 | 39 / 11 / 0 |

## 4MiB, k = 1: client CPU per call, ms: median [p10-p90] over all calls; below it the range of the per-iteration medians

| cell | base | h1 | c1 | b1 |
|---|---:|---:|---:|---:|
| A | 2.64 [2.21-3.27]<br>it 2.24-2.93 | 2.51 [2.07-3.08]<br>it 2.25-2.71 | 2.48 [2.18-2.96]<br>it 2.37-2.63 | 2.50 [2.09-3.20]<br>it 2.24-3.04 |
| Df-retain | 2.57 [2.16-3.08]<br>it 2.38-2.79 | 2.62 [2.15-3.25]<br>it 2.31-2.92 | 2.70 [2.31-3.32]<br>it 2.55-2.91 | 2.45 [2.06-3.01]<br>it 2.27-2.71 |
| Df-chan | 2.63 [2.29-3.17]<br>it 2.49-2.74 | 2.60 [2.25-3.10]<br>it 2.39-2.79 | 2.75 [2.27-3.33]<br>it 2.35-3.00 | 2.49 [2.18-2.95]<br>it 2.33-2.69 |
| Cf-cb-retain | 2.65 [2.25-3.18]<br>it 2.46-2.80 | 2.72 [2.21-3.33]<br>it 2.33-2.94 | 2.61 [2.22-3.16]<br>it 2.48-2.78 | 2.69 [2.26-3.23]<br>it 2.48-2.92 |

### 4MiB: variant minus `base`, ms per call, per iteration (the two processes of one iteration ran back to back); `x/n neg` = iterations with a negative difference

| cell | h1 | c1 | b1 |
|---|---:|---:|---:|
| A | -0.16 (-0.44..+0.25; 3/5 neg) | -0.32 (-0.48..+0.35; 4/5 neg) | -0.15 (-0.65..+0.37; 3/5 neg) |
| Df-retain | +0.07 (-0.31..+0.40; 2/5 neg) | +0.08 (-0.23..+0.45; 2/5 neg) | -0.18 (-0.35..+0.33; 4/5 neg) |
| Df-chan | -0.03 (-0.21..+0.12; 3/5 neg) | +0.08 (-0.14..+0.28; 1/5 neg) | -0.12 (-0.20..-0.03; 5/5 neg) |
| Cf-cb-retain | +0.15 (-0.33..+0.48; 2/5 neg) | +0.05 (-0.27..+0.18; 2/5 neg) | +0.15 (-0.30..+0.47; 2/5 neg) |

### 4MiB: in-process control: cell minus A of the SAME process, ms per call (per-iteration medians; median over iterations, range)

| cell | base | h1 | c1 | b1 |
|---|---:|---:|---:|---:|
| Df-retain | -0.07 (-0.33..+0.14) | +0.23 (-0.34..+0.36) | +0.19 (+0.11..+0.33) | -0.01 (-0.56..+0.20) |
| Df-chan | -0.05 (-0.32..+0.48) | +0.09 (-0.31..+0.30) | +0.25 (-0.02..+0.41) | +0.07 (-0.51..+0.21) |
| Cf-cb-retain | -0.12 (-0.17..+0.21) | +0.17 (-0.17..+0.44) | +0.11 (+0.04..+0.23) | +0.22 (-0.27..+0.48) |

### 4MiB: context switches and minor faults per call (getrusage, whole process; medians over rounds): voluntary / involuntary / minflt

| cell | base | h1 | c1 | b1 |
|---|---:|---:|---:|---:|
| A | 12 / 2 / 0 | 10 / 0 / 0 | 13 / 2 / 0 | 11 / 0 / 0 |
| Df-retain | 12 / 2 / 0 | 10 / 0 / 0 | 12 / 2 / 0 | 10 / 0 / 0 |
| Df-chan | 19 / 6 / 0 | 15 / 2 / 0 | 19 / 6 / 0 | 15 / 2 / 0 |
| Cf-cb-retain | 22 / 6 / 0 | 21 / 7 / 0 | 18 / 5 / 0 | 15 / 4 / 0 |

## the split cells' host work per chunk (medians over rounds): encode CPU us / send-entry CPU us / send wall us (blocking: until the send returned; callback: until its completion arrived); recv wall ms per call

| cell | size | base | h1 | c1 | b1 |
|---|---|---:|---:|---:|---:|
| Df-chan | 16MiB | 393 / 7.7 / 591; 6.74 | 428 / 10.2 / 703; 7.15 | 432 / 8.1 / 592; 6.90 | 410 / 9.4 / 616; 6.35 |
| Df-chan | 4MiB | 365 / 8.0 / 132; 4.09 | 385 / 9.8 / 142; 3.91 | 388 / 8.2 / 135; 4.12 | 367 / 9.5 / 123; 3.66 |
