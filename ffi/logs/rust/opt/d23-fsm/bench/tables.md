# D23: decode-read through push, pull and the FSM (container instrumentation)

Source: `bench` (`gen/d23_bench.sh`); every figure is container instrumentation, not a campaign result. Per cell: the median over launches of each launch's median ns per decode (criterion, 10 rounds, process CPU), and in brackets the range of the launch medians. One process per launch holds all three arms (same core, same binding, same facade); arm blocks and cases in a seeded random order per launch.

    # arm order: core-native,incumbent-prod,core-ffi,core-ffi-pull,core-ffi-fsm,armonik (arm blocks and the cases inside each block in a seeded random order, seed = launch; criterion runs them in this reg
    # arm order: armonik,core-ffi-fsm,core-ffi,core-ffi-pull,incumbent-prod,core-native (arm blocks and the cases inside each block in a seeded random order, seed = launch; criterion runs them in this reg
    # arm order: core-native,core-ffi,core-ffi-pull,core-ffi-fsm,armonik,incumbent-prod (arm blocks and the cases inside each block in a seeded random order, seed = launch; criterion runs them in this reg

| input | mode | push | pull | fsm |
|---|---|---|---|---|
| P1.1 | drop | 1.63 us [1.59 us .. 1.63 us] (n=3) | 1.47 us [1.39 us .. 1.69 us] (n=3) | 1.51 us [1.4 us .. 1.65 us] (n=3) |
| P1.1 | retain | 1.61 us [1.61 us .. 1.69 us] (n=3) | 1.54 us [1.52 us .. 1.65 us] (n=3) | 1.64 us [1.59 us .. 1.72 us] (n=3) |
| P1.2 | drop | 487 us [443 us .. 488 us] (n=3) | 451 us [438 us .. 457 us] (n=3) | 465 us [445 us .. 476 us] (n=3) |
| P1.2 | retain | 492 us [462 us .. 569 us] (n=3) | 449 us [447 us .. 459 us] (n=3) | 460 us [459 us .. 481 us] (n=3) |
| P1.2/latin1 | drop | 531 us [512 us .. 566 us] (n=3) | 538 us [505 us .. 559 us] (n=3) | 509 us [489 us .. 527 us] (n=3) |
| P1.2/latin1 | retain | 543 us [516 us .. 548 us] (n=3) | 516 us [510 us .. 538 us] (n=3) | 546 us [490 us .. 548 us] (n=3) |
| P1.2/wide | drop | 572 us [565 us .. 612 us] (n=3) | 581 us [541 us .. 583 us] (n=3) | 578 us [578 us .. 579 us] (n=3) |
| P1.2/wide | retain | 582 us [528 us .. 586 us] (n=3) | 593 us [591 us .. 611 us] (n=3) | 582 us [528 us .. 617 us] (n=3) |
| P1.3 | drop | 16.3 us [16 us .. 16.3 us] (n=3) | 16.2 us [16.2 us .. 16.4 us] (n=3) | 15.8 us [15.7 us .. 16.4 us] (n=3) |
| P1.3 | retain | 16.5 us [16.5 us .. 16.7 us] (n=3) | 16.3 us [16.2 us .. 16.7 us] (n=3) | 16.4 us [15.9 us .. 16.4 us] (n=3) |
| P2.1 | drop | 2.84 us [2.73 us .. 2.99 us] (n=3) | 2.89 us [2.87 us .. 2.99 us] (n=3) | 2.93 us [2.93 us .. 2.95 us] (n=3) |
| P2.1 | retain | 2.97 us [2.91 us .. 3.06 us] (n=3) | 2.97 us [2.88 us .. 3.01 us] (n=3) | 2.99 us [2.8 us .. 3.56 us] (n=3) |
| P2.2 | drop | 2.5 ms [2.47 ms .. 2.78 ms] (n=3) | 2.31 ms [2.17 ms .. 2.37 ms] (n=3) | 2.47 ms [2.46 ms .. 2.89 ms] (n=3) |
| P2.2 | retain | 2.53 ms [2.51 ms .. 2.54 ms] (n=3) | 2.36 ms [2.14 ms .. 2.39 ms] (n=3) | 2.54 ms [2.43 ms .. 2.58 ms] (n=3) |
| P2.2/latin1 | drop | 2.57 ms [2.46 ms .. 2.68 ms] (n=3) | 2.62 ms [2.54 ms .. 2.64 ms] (n=3) | 2.55 ms [2.54 ms .. 2.6 ms] (n=3) |
| P2.2/latin1 | retain | 2.54 ms [2.52 ms .. 2.7 ms] (n=3) | 2.6 ms [2.49 ms .. 2.62 ms] (n=3) | 2.65 ms [2.53 ms .. 2.75 ms] (n=3) |
| P2.2/wide | drop | 2.72 ms [2.72 ms .. 2.85 ms] (n=3) | 2.71 ms [2.62 ms .. 2.87 ms] (n=3) | 2.95 ms [2.72 ms .. 2.96 ms] (n=3) |
| P2.2/wide | retain | 2.89 ms [2.64 ms .. 2.98 ms] (n=3) | 2.67 ms [2.63 ms .. 2.81 ms] (n=3) | 2.7 ms [2.6 ms .. 2.94 ms] (n=3) |
| P2.3 | drop | 1.5 ms [1.45 ms .. 1.64 ms] (n=3) | 1.38 ms [1.26 ms .. 1.39 ms] (n=3) | 1.39 ms [1.34 ms .. 1.4 ms] (n=3) |
| P2.3 | retain | 1.44 ms [1.33 ms .. 1.45 ms] (n=3) | 1.45 ms [1.34 ms .. 1.48 ms] (n=3) | 1.53 ms [1.4 ms .. 1.54 ms] (n=3) |
| P2.4 | drop | 2.37 ms [2.02 ms .. 2.54 ms] (n=3) | 2.19 ms [1.92 ms .. 2.25 ms] (n=3) | 2.37 ms [2.34 ms .. 2.47 ms] (n=3) |
| P2.4 | retain | 2.32 ms [2.05 ms .. 2.52 ms] (n=3) | 2.02 ms [1.86 ms .. 2.09 ms] (n=3) | 2.16 ms [2.06 ms .. 2.53 ms] (n=3) |
| P2.4/latin1 | drop | 2.68 ms [2.5 ms .. 3 ms] (n=3) | 2.71 ms [2.7 ms .. 2.86 ms] (n=3) | 2.74 ms [2.67 ms .. 2.82 ms] (n=3) |
| P2.4/latin1 | retain | 2.77 ms [2.6 ms .. 3.05 ms] (n=3) | 2.73 ms [2.63 ms .. 2.77 ms] (n=3) | 2.82 ms [2.68 ms .. 2.88 ms] (n=3) |
| P2.4/wide | drop | 3.05 ms [2.86 ms .. 3.09 ms] (n=3) | 3.03 ms [2.92 ms .. 3.17 ms] (n=3) | 2.9 ms [2.85 ms .. 3.12 ms] (n=3) |
| P2.4/wide | retain | 3.17 ms [2.81 ms .. 3.18 ms] (n=3) | 3.29 ms [2.98 ms .. 3.48 ms] (n=3) | 3.03 ms [2.85 ms .. 3.07 ms] (n=3) |
| P2.5 | drop | 62.8 us [62.8 us .. 64.2 us] (n=3) | 60.1 us [59.9 us .. 60.3 us] (n=3) | 65.9 us [64 us .. 68.2 us] (n=3) |
| P2.5 | retain | 63.7 us [62.2 us .. 67.2 us] (n=3) | 60.3 us [59.9 us .. 61.3 us] (n=3) | 66.1 us [65.2 us .. 66.7 us] (n=3) |
| P3.1 | drop | 33.6 us [33.3 us .. 34 us] (n=3) | 32.7 us [32.6 us .. 33.5 us] (n=3) | 33.4 us [32.8 us .. 33.6 us] (n=3) |
| P3.1 | retain | 35.3 us [32.4 us .. 38.5 us] (n=3) | 33.3 us [33.1 us .. 33.7 us] (n=3) | 33.9 us [33.2 us .. 33.9 us] (n=3) |
| P4.1 | drop | 349 us [347 us .. 351 us] (n=3) | 332 us [327 us .. 336 us] (n=3) | 356 us [351 us .. 372 us] (n=3) |
| P4.1 | retain | 353 us [342 us .. 372 us] (n=3) | 343 us [326 us .. 352 us] (n=3) | 348 us [346 us .. 363 us] (n=3) |
| P5.1 | drop | 115 ns [112 ns .. 115 ns] (n=3) | 120 ns [120 ns .. 124 ns] (n=3) | 129 ns [123 ns .. 129 ns] (n=3) |
| P5.1 | retain | 125 ns [123 ns .. 145 ns] (n=3) | 132 ns [130 ns .. 133 ns] (n=3) | 146 ns [144 ns .. 147 ns] (n=3) |
| P5.2 | drop | 1.71 us [1.7 us .. 1.71 us] (n=3) | 1.69 us [1.69 us .. 1.73 us] (n=3) | 1.72 us [1.69 us .. 1.74 us] (n=3) |
| P5.2 | retain | 1.7 us [1.69 us .. 1.71 us] (n=3) | 1.71 us [1.7 us .. 1.76 us] (n=3) | 1.71 us [1.7 us .. 1.74 us] (n=3) |
| P5.3 | drop | 65 us [63.4 us .. 70.1 us] (n=3) | 65.5 us [63.3 us .. 76.3 us] (n=3) | 68.9 us [67.6 us .. 76.2 us] (n=3) |
| P5.3 | retain | 74.3 us [72.8 us .. 75.3 us] (n=3) | 69.6 us [63.8 us .. 77.2 us] (n=3) | 69.3 us [66.5 us .. 76.5 us] (n=3) |
| P5.4 | drop | 288 us [270 us .. 308 us] (n=3) | 288 us [264 us .. 288 us] (n=3) | 302 us [267 us .. 309 us] (n=3) |
| P5.4 | retain | 290 us [279 us .. 305 us] (n=3) | 281 us [274 us .. 281 us] (n=3) | 308 us [294 us .. 313 us] (n=3) |
| P6.1 | drop | 224 us [216 us .. 228 us] (n=3) | 224 us [224 us .. 226 us] (n=3) | 307 us [302 us .. 310 us] (n=3) |
| P6.1 | retain | 218 us [213 us .. 239 us] (n=3) | 225 us [222 us .. 243 us] (n=3) | 312 us [305 us .. 312 us] (n=3) |
| P7.1 | drop | 350 ns [349 ns .. 357 ns] (n=3) | 366 ns [364 ns .. 402 ns] (n=3) | 412 ns [399 ns .. 413 ns] (n=3) |
| P7.1 | retain | 373 ns [372 ns .. 373 ns] (n=3) | 390 ns [386 ns .. 414 ns] (n=3) | 439 ns [429 ns .. 511 ns] (n=3) |
| U-deep-all | drop | 2.76 us [2.53 us .. 2.77 us] (n=3) | 2.62 us [2.59 us .. 2.65 us] (n=3) | 2.76 us [2.74 us .. 2.86 us] (n=3) |
| U-deep-all | retain | 3.07 us [3.01 us .. 3.09 us] (n=3) | 2.95 us [2.85 us .. 3.12 us] (n=3) | 3.03 us [2.85 us .. 3.23 us] (n=3) |
| U-deep-u-fixed32 | drop | 2.81 us [2.5 us .. 2.84 us] (n=3) | 2.63 us [2.56 us .. 2.64 us] (n=3) | 2.79 us [2.62 us .. 2.81 us] (n=3) |
| U-deep-u-fixed32 | retain | 2.89 us [2.66 us .. 2.94 us] (n=3) | 2.93 us [2.77 us .. 3.18 us] (n=3) | 2.72 us [2.68 us .. 2.91 us] (n=3) |
| U-deep-u-fixed64 | drop | 2.74 us [2.62 us .. 2.87 us] (n=3) | 2.54 us [2.53 us .. 2.76 us] (n=3) | 2.58 us [2.56 us .. 2.75 us] (n=3) |
| U-deep-u-fixed64 | retain | 2.87 us [2.86 us .. 2.93 us] (n=3) | 2.8 us [2.78 us .. 2.84 us] (n=3) | 2.68 us [2.66 us .. 2.96 us] (n=3) |
| U-deep-u-len | drop | 2.73 us [2.52 us .. 2.77 us] (n=3) | 2.62 us [2.52 us .. 2.79 us] (n=3) | 2.73 us [2.61 us .. 2.8 us] (n=3) |
| U-deep-u-len | retain | 3.01 us [2.74 us .. 3.05 us] (n=3) | 2.79 us [2.76 us .. 2.8 us] (n=3) | 2.76 us [2.71 us .. 2.83 us] (n=3) |
| U-deep-u-msg | drop | 2.74 us [2.53 us .. 2.78 us] (n=3) | 2.6 us [2.59 us .. 2.73 us] (n=3) | 2.59 us [2.58 us .. 2.78 us] (n=3) |
| U-deep-u-msg | retain | 2.85 us [2.84 us .. 2.89 us] (n=3) | 2.79 us [2.78 us .. 2.99 us] (n=3) | 2.98 us [2.9 us .. 3.03 us] (n=3) |
| U-deep-u-packed | drop | 2.71 us [2.53 us .. 2.75 us] (n=3) | 2.66 us [2.51 us .. 2.85 us] (n=3) | 2.59 us [2.57 us .. 2.89 us] (n=3) |
| U-deep-u-packed | retain | 2.75 us [2.73 us .. 2.91 us] (n=3) | 2.82 us [2.69 us .. 2.92 us] (n=3) | 2.87 us [2.78 us .. 2.9 us] (n=3) |
| U-deep-u-repeated | drop | 2.75 us [2.72 us .. 2.76 us] (n=3) | 2.65 us [2.51 us .. 2.77 us] (n=3) | 2.67 us [2.65 us .. 2.8 us] (n=3) |
| U-deep-u-repeated | retain | 2.9 us [2.7 us .. 3.03 us] (n=3) | 2.74 us [2.69 us .. 2.77 us] (n=3) | 2.9 us [2.82 us .. 2.9 us] (n=3) |
| U-deep-u-varint | drop | 2.68 us [2.49 us .. 2.81 us] (n=3) | 2.62 us [2.57 us .. 2.78 us] (n=3) | 2.75 us [2.54 us .. 2.76 us] (n=3) |
| U-deep-u-varint | retain | 2.87 us [2.86 us .. 2.95 us] (n=3) | 2.84 us [2.69 us .. 2.94 us] (n=3) | 2.81 us [2.8 us .. 3.65 us] (n=3) |
| U-element-all | drop | 2.78 us [2.64 us .. 2.82 us] (n=3) | 2.68 us [2.65 us .. 2.68 us] (n=3) | 2.66 us [2.63 us .. 2.82 us] (n=3) |
| U-element-all | retain | 3.08 us [3.03 us .. 3.09 us] (n=3) | 2.99 us [2.92 us .. 3.01 us] (n=3) | 2.97 us [2.87 us .. 3.18 us] (n=3) |
| U-element-u-fixed32 | drop | 2.72 us [2.69 us .. 2.8 us] (n=3) | 2.67 us [2.56 us .. 2.75 us] (n=3) | 2.69 us [2.59 us .. 3.11 us] (n=3) |
| U-element-u-fixed32 | retain | 2.88 us [2.67 us .. 2.99 us] (n=3) | 2.74 us [2.74 us .. 2.76 us] (n=3) | 2.76 us [2.69 us .. 2.99 us] (n=3) |
| U-element-u-fixed64 | drop | 2.74 us [2.58 us .. 2.74 us] (n=3) | 2.72 us [2.59 us .. 2.82 us] (n=3) | 2.64 us [2.6 us .. 2.68 us] (n=3) |
| U-element-u-fixed64 | retain | 2.89 us [2.86 us .. 2.92 us] (n=3) | 2.79 us [2.74 us .. 2.85 us] (n=3) | 2.73 us [2.69 us .. 2.91 us] (n=3) |
| U-element-u-len | drop | 2.73 us [2.52 us .. 2.77 us] (n=3) | 2.61 us [2.48 us .. 2.64 us] (n=3) | 2.71 us [2.61 us .. 2.83 us] (n=3) |
| U-element-u-len | retain | 2.94 us [2.87 us .. 3.11 us] (n=3) | 2.73 us [2.68 us .. 2.75 us] (n=3) | 2.72 us [2.66 us .. 2.94 us] (n=3) |
| U-element-u-msg | drop | 2.73 us [2.51 us .. 2.77 us] (n=3) | 2.67 us [2.6 us .. 2.7 us] (n=3) | 2.72 us [2.67 us .. 2.78 us] (n=3) |
| U-element-u-msg | retain | 2.91 us [2.88 us .. 2.93 us] (n=3) | 2.79 us [2.76 us .. 2.96 us] (n=3) | 2.84 us [2.69 us .. 2.9 us] (n=3) |
| U-element-u-packed | drop | 2.76 us [2.48 us .. 2.78 us] (n=3) | 2.55 us [2.53 us .. 2.63 us] (n=3) | 2.68 us [2.63 us .. 2.76 us] (n=3) |
| U-element-u-packed | retain | 2.94 us [2.72 us .. 2.98 us] (n=3) | 2.71 us [2.68 us .. 2.81 us] (n=3) | 2.79 us [2.75 us .. 2.94 us] (n=3) |
| U-element-u-repeated | drop | 2.72 us [2.71 us .. 2.87 us] (n=3) | 2.76 us [2.63 us .. 2.78 us] (n=3) | 2.75 us [2.58 us .. 2.92 us] (n=3) |
| U-element-u-repeated | retain | 2.94 us [2.84 us .. 2.95 us] (n=3) | 2.77 us [2.7 us .. 2.86 us] (n=3) | 2.86 us [2.73 us .. 2.9 us] (n=3) |
| U-element-u-varint | drop | 2.71 us [2.68 us .. 2.93 us] (n=3) | 2.86 us [2.62 us .. 2.91 us] (n=3) | 2.58 us [2.56 us .. 2.9 us] (n=3) |
| U-element-u-varint | retain | 2.91 us [2.7 us .. 3.01 us] (n=3) | 2.76 us [2.75 us .. 2.79 us] (n=3) | 2.77 us [2.75 us .. 2.94 us] (n=3) |
| U-enum-value-127 | drop | 211 ns [151 ns .. 239 ns] (n=3) | 248 ns [160 ns .. 265 ns] (n=3) | 186 ns [171 ns .. 244 ns] (n=3) |
| U-enum-value-127 | retain | 233 ns [181 ns .. 240 ns] (n=3) | 265 ns [181 ns .. 274 ns] (n=3) | 183 ns [169 ns .. 187 ns] (n=3) |
| U-enum-value-2147483647 | drop | 214 ns [158 ns .. 248 ns] (n=3) | 258 ns [171 ns .. 266 ns] (n=3) | 271 ns [264 ns .. 325 ns] (n=3) |
| U-enum-value-2147483647 | retain | 233 ns [190 ns .. 244 ns] (n=3) | 186 ns [186 ns .. 278 ns] (n=3) | 192 ns [190 ns .. 268 ns] (n=3) |
| U-enum-value-999 | drop | 231 ns [141 ns .. 238 ns] (n=3) | 231 ns [165 ns .. 239 ns] (n=3) | 174 ns [169 ns .. 220 ns] (n=3) |
| U-enum-value-999 | retain | 231 ns [158 ns .. 251 ns] (n=3) | 259 ns [183 ns .. 273 ns] (n=3) | 240 ns [206 ns .. 275 ns] (n=3) |
| U-enum-value-packed | drop | 142 ns [141 ns .. 145 ns] (n=3) | 160 ns [160 ns .. 168 ns] (n=3) | 179 ns [178 ns .. 180 ns] (n=3) |
| U-enum-value-packed | retain | 157 ns [156 ns .. 161 ns] (n=3) | 179 ns [178 ns .. 199 ns] (n=3) | 200 ns [197 ns .. 212 ns] (n=3) |
| U-leaf-all | drop | 540 ns [517 ns .. 555 ns] (n=3) | 442 ns [440 ns .. 531 ns] (n=3) | 577 ns [572 ns .. 591 ns] (n=3) |
| U-leaf-all | retain | 1.1 us [1.07 us .. 1.16 us] (n=3) | 1.09 us [1.08 us .. 1.1 us] (n=3) | 1.14 us [1.09 us .. 1.16 us] (n=3) |
| U-leaf-u-fixed32 | drop | 441 ns [366 ns .. 443 ns] (n=3) | 371 ns [364 ns .. 472 ns] (n=3) | 454 ns [384 ns .. 484 ns] (n=3) |
| U-leaf-u-fixed32 | retain | 582 ns [462 ns .. 589 ns] (n=3) | 582 ns [489 ns .. 592 ns] (n=3) | 505 ns [479 ns .. 703 ns] (n=3) |
| U-leaf-u-fixed64 | drop | 363 ns [342 ns .. 438 ns] (n=3) | 454 ns [363 ns .. 482 ns] (n=3) | 543 ns [479 ns .. 594 ns] (n=3) |
| U-leaf-u-fixed64 | retain | 563 ns [558 ns .. 576 ns] (n=3) | 594 ns [593 ns .. 703 ns] (n=3) | 589 ns [576 ns .. 608 ns] (n=3) |
| U-leaf-u-len | drop | 443 ns [369 ns .. 449 ns] (n=3) | 479 ns [478 ns .. 548 ns] (n=3) | 483 ns [392 ns .. 566 ns] (n=3) |
| U-leaf-u-len | retain | 561 ns [561 ns .. 590 ns] (n=3) | 573 ns [488 ns .. 603 ns] (n=3) | 598 ns [597 ns .. 620 ns] (n=3) |
| U-leaf-u-msg | drop | 441 ns [354 ns .. 468 ns] (n=3) | 363 ns [360 ns .. 482 ns] (n=3) | 450 ns [395 ns .. 579 ns] (n=3) |
| U-leaf-u-msg | retain | 548 ns [470 ns .. 579 ns] (n=3) | 478 ns [474 ns .. 600 ns] (n=3) | 592 ns [582 ns .. 598 ns] (n=3) |
| U-leaf-u-packed | drop | 448 ns [367 ns .. 472 ns] (n=3) | 469 ns [371 ns .. 484 ns] (n=3) | 407 ns [381 ns .. 474 ns] (n=3) |
| U-leaf-u-packed | retain | 479 ns [452 ns .. 584 ns] (n=3) | 596 ns [593 ns .. 611 ns] (n=3) | 534 ns [484 ns .. 709 ns] (n=3) |
| U-leaf-u-repeated | drop | 459 ns [402 ns .. 466 ns] (n=3) | 481 ns [473 ns .. 563 ns] (n=3) | 538 ns [381 ns .. 612 ns] (n=3) |
| U-leaf-u-repeated | retain | 501 ns [494 ns .. 586 ns] (n=3) | 623 ns [580 ns .. 633 ns] (n=3) | 629 ns [615 ns .. 726 ns] (n=3) |
| U-leaf-u-varint | drop | 443 ns [441 ns .. 464 ns] (n=3) | 383 ns [375 ns .. 479 ns] (n=3) | 423 ns [391 ns .. 471 ns] (n=3) |
| U-leaf-u-varint | retain | 586 ns [582 ns .. 590 ns] (n=3) | 600 ns [493 ns .. 612 ns] (n=3) | 584 ns [525 ns .. 600 ns] (n=3) |
| U-nested-all | drop | 463 ns [457 ns .. 490 ns] (n=3) | 398 ns [393 ns .. 485 ns] (n=3) | 499 ns [422 ns .. 504 ns] (n=3) |
| U-nested-all | retain | 668 ns [660 ns .. 713 ns] (n=3) | 689 ns [602 ns .. 702 ns] (n=3) | 712 ns [707 ns .. 749 ns] (n=3) |
| U-nested-before | drop | 395 ns [386 ns .. 490 ns] (n=3) | 405 ns [400 ns .. 416 ns] (n=3) | 533 ns [447 ns .. 542 ns] (n=3) |
| U-nested-before | retain | 699 ns [686 ns .. 755 ns] (n=3) | 713 ns [696 ns .. 714 ns] (n=3) | 721 ns [683 ns .. 812 ns] (n=3) |
| U-nested-group | drop | 457 ns [360 ns .. 501 ns] (n=3) | 507 ns [385 ns .. 561 ns] (n=3) | 478 ns [413 ns .. 499 ns] (n=3) |
| U-nested-group | retain | 548 ns [531 ns .. 549 ns] (n=3) | 473 ns [453 ns .. 562 ns] (n=3) | 575 ns [486 ns .. 580 ns] (n=3) |
| U-nested-interleaved | drop | 466 ns [461 ns .. 499 ns] (n=3) | 392 ns [390 ns .. 394 ns] (n=3) | 509 ns [492 ns .. 616 ns] (n=3) |
| U-nested-interleaved | retain | 695 ns [692 ns .. 699 ns] (n=3) | 684 ns [609 ns .. 703 ns] (n=3) | 706 ns [690 ns .. 824 ns] (n=3) |
| U-nested-u-fixed32 | drop | 436 ns [361 ns .. 473 ns] (n=3) | 470 ns [416 ns .. 472 ns] (n=3) | 384 ns [382 ns .. 557 ns] (n=3) |
| U-nested-u-fixed32 | retain | 523 ns [521 ns .. 528 ns] (n=3) | 518 ns [436 ns .. 528 ns] (n=3) | 458 ns [453 ns .. 520 ns] (n=3) |
| U-nested-u-fixed64 | drop | 437 ns [352 ns .. 460 ns] (n=3) | 485 ns [367 ns .. 532 ns] (n=3) | 455 ns [382 ns .. 570 ns] (n=3) |
| U-nested-u-fixed64 | retain | 507 ns [507 ns .. 529 ns] (n=3) | 435 ns [431 ns .. 448 ns] (n=3) | 538 ns [517 ns .. 539 ns] (n=3) |
| U-nested-u-len | drop | 433 ns [363 ns .. 449 ns] (n=3) | 374 ns [364 ns .. 528 ns] (n=3) | 514 ns [379 ns .. 574 ns] (n=3) |
| U-nested-u-len | retain | 521 ns [508 ns .. 523 ns] (n=3) | 552 ns [438 ns .. 639 ns] (n=3) | 445 ns [427 ns .. 457 ns] (n=3) |
| U-nested-u-msg | drop | 365 ns [344 ns .. 475 ns] (n=3) | 443 ns [354 ns .. 479 ns] (n=3) | 394 ns [384 ns .. 450 ns] (n=3) |
| U-nested-u-msg | retain | 507 ns [426 ns .. 567 ns] (n=3) | 535 ns [430 ns .. 536 ns] (n=3) | 545 ns [454 ns .. 561 ns] (n=3) |
| U-nested-u-packed | drop | 447 ns [443 ns .. 460 ns] (n=3) | 415 ns [364 ns .. 478 ns] (n=3) | 400 ns [382 ns .. 477 ns] (n=3) |
| U-nested-u-packed | retain | 504 ns [424 ns .. 552 ns] (n=3) | 529 ns [431 ns .. 538 ns] (n=3) | 556 ns [532 ns .. 563 ns] (n=3) |
| U-nested-u-repeated | drop | 432 ns [367 ns .. 447 ns] (n=3) | 471 ns [360 ns .. 472 ns] (n=3) | 470 ns [387 ns .. 486 ns] (n=3) |
| U-nested-u-repeated | retain | 525 ns [524 ns .. 547 ns] (n=3) | 519 ns [452 ns .. 553 ns] (n=3) | 470 ns [462 ns .. 534 ns] (n=3) |
| U-nested-u-varint | drop | 455 ns [431 ns .. 459 ns] (n=3) | 459 ns [371 ns .. 536 ns] (n=3) | 447 ns [382 ns .. 451 ns] (n=3) |
| U-nested-u-varint | retain | 516 ns [427 ns .. 554 ns] (n=3) | 539 ns [438 ns .. 542 ns] (n=3) | 527 ns [468 ns .. 550 ns] (n=3) |
| U-oneof-all | drop | 184 ns [180 ns .. 189 ns] (n=3) | 188 ns [183 ns .. 190 ns] (n=3) | 196 ns [195 ns .. 197 ns] (n=3) |
| U-oneof-all | retain | 363 ns [353 ns .. 383 ns] (n=3) | 366 ns [365 ns .. 375 ns] (n=3) | 396 ns [380 ns .. 411 ns] (n=3) |
| U-oneof-before | drop | 182 ns [180 ns .. 201 ns] (n=3) | 191 ns [187 ns .. 202 ns] (n=3) | 198 ns [197 ns .. 200 ns] (n=3) |
| U-oneof-before | retain | 359 ns [357 ns .. 377 ns] (n=3) | 370 ns [370 ns .. 375 ns] (n=3) | 388 ns [380 ns .. 397 ns] (n=3) |
| U-oneof-group | drop | 167 ns [167 ns .. 168 ns] (n=3) | 167 ns [166 ns .. 169 ns] (n=3) | 189 ns [189 ns .. 192 ns] (n=3) |
| U-oneof-group | retain | 243 ns [235 ns .. 251 ns] (n=3) | 241 ns [240 ns .. 242 ns] (n=3) | 263 ns [260 ns .. 264 ns] (n=3) |
| U-oneof-interleaved | drop | 180 ns [179 ns .. 181 ns] (n=3) | 189 ns [188 ns .. 194 ns] (n=3) | 202 ns [197 ns .. 206 ns] (n=3) |
| U-oneof-interleaved | retain | 357 ns [354 ns .. 368 ns] (n=3) | 388 ns [374 ns .. 393 ns] (n=3) | 375 ns [374 ns .. 410 ns] (n=3) |
| U-oneof-member-after-known | drop | 146 ns [145 ns .. 146 ns] (n=3) | 148 ns [147 ns .. 150 ns] (n=3) | 157 ns [155 ns .. 160 ns] (n=3) |
| U-oneof-member-after-known | retain | 210 ns [210 ns .. 212 ns] (n=3) | 217 ns [216 ns .. 235 ns] (n=3) | 227 ns [225 ns .. 228 ns] (n=3) |
| U-oneof-member-alone | drop | 134 ns [132 ns .. 144 ns] (n=3) | 139 ns [137 ns .. 142 ns] (n=3) | 141 ns [140 ns .. 145 ns] (n=3) |
| U-oneof-member-alone | retain | 199 ns [198 ns .. 204 ns] (n=3) | 209 ns [207 ns .. 210 ns] (n=3) | 210 ns [210 ns .. 214 ns] (n=3) |
| U-oneof-member-before-known | drop | 144 ns [143 ns .. 151 ns] (n=3) | 154 ns [149 ns .. 155 ns] (n=3) | 156 ns [156 ns .. 173 ns] (n=3) |
| U-oneof-member-before-known | retain | 207 ns [207 ns .. 210 ns] (n=3) | 221 ns [218 ns .. 223 ns] (n=3) | 227 ns [224 ns .. 232 ns] (n=3) |
| U-oneof-u-fixed32 | drop | 146 ns [144 ns .. 148 ns] (n=3) | 149 ns [146 ns .. 155 ns] (n=3) | 158 ns [155 ns .. 159 ns] (n=3) |
| U-oneof-u-fixed32 | retain | 211 ns [208 ns .. 214 ns] (n=3) | 220 ns [216 ns .. 223 ns] (n=3) | 232 ns [227 ns .. 234 ns] (n=3) |
| U-oneof-u-fixed64 | drop | 145 ns [144 ns .. 145 ns] (n=3) | 148 ns [146 ns .. 151 ns] (n=3) | 155 ns [155 ns .. 157 ns] (n=3) |
| U-oneof-u-fixed64 | retain | 209 ns [207 ns .. 211 ns] (n=3) | 220 ns [217 ns .. 221 ns] (n=3) | 229 ns [227 ns .. 232 ns] (n=3) |
| U-oneof-u-len | drop | 146 ns [144 ns .. 149 ns] (n=3) | 148 ns [147 ns .. 154 ns] (n=3) | 161 ns [156 ns .. 163 ns] (n=3) |
| U-oneof-u-len | retain | 210 ns [210 ns .. 210 ns] (n=3) | 218 ns [217 ns .. 227 ns] (n=3) | 229 ns [227 ns .. 231 ns] (n=3) |
| U-oneof-u-msg | drop | 145 ns [144 ns .. 145 ns] (n=3) | 149 ns [148 ns .. 150 ns] (n=3) | 156 ns [156 ns .. 166 ns] (n=3) |
| U-oneof-u-msg | retain | 215 ns [213 ns .. 216 ns] (n=3) | 218 ns [218 ns .. 218 ns] (n=3) | 230 ns [227 ns .. 234 ns] (n=3) |
| U-oneof-u-packed | drop | 145 ns [145 ns .. 147 ns] (n=3) | 149 ns [148 ns .. 151 ns] (n=3) | 163 ns [159 ns .. 165 ns] (n=3) |
| U-oneof-u-packed | retain | 213 ns [213 ns .. 217 ns] (n=3) | 218 ns [217 ns .. 219 ns] (n=3) | 231 ns [227 ns .. 235 ns] (n=3) |
| U-oneof-u-repeated | drop | 154 ns [153 ns .. 156 ns] (n=3) | 152 ns [152 ns .. 154 ns] (n=3) | 165 ns [161 ns .. 173 ns] (n=3) |
| U-oneof-u-repeated | retain | 222 ns [219 ns .. 234 ns] (n=3) | 231 ns [230 ns .. 241 ns] (n=3) | 239 ns [239 ns .. 246 ns] (n=3) |
| U-oneof-u-varint | drop | 149 ns [147 ns .. 156 ns] (n=3) | 151 ns [150 ns .. 151 ns] (n=3) | 161 ns [160 ns .. 162 ns] (n=3) |
| U-oneof-u-varint | retain | 213 ns [211 ns .. 216 ns] (n=3) | 226 ns [220 ns .. 230 ns] (n=3) | 235 ns [229 ns .. 236 ns] (n=3) |
| U-root-all | drop | 953 ns [953 ns .. 958 ns] (n=3) | 851 ns [801 ns .. 894 ns] (n=3) | 979 ns [914 ns .. 1 us] (n=3) |
| U-root-all | retain | 1.15 us [1.02 us .. 1.15 us] (n=3) | 1.15 us [1.11 us .. 1.2 us] (n=3) | 1.13 us [1.13 us .. 1.14 us] (n=3) |
| U-root-before | drop | 928 ns [841 ns .. 947 ns] (n=3) | 796 ns [784 ns .. 912 ns] (n=3) | 1.01 us [957 ns .. 1.1 us] (n=3) |
| U-root-before | retain | 1.14 us [1.14 us .. 1.21 us] (n=3) | 1.12 us [1.02 us .. 1.15 us] (n=3) | 1.17 us [1.13 us .. 1.19 us] (n=3) |
| U-root-group | drop | 913 ns [841 ns .. 920 ns] (n=3) | 837 ns [812 ns .. 1.02 us] (n=3) | 970 ns [806 ns .. 1.04 us] (n=3) |
| U-root-group | retain | 997 ns [903 ns .. 1.03 us] (n=3) | 900 ns [842 ns .. 958 ns] (n=3) | 933 ns [900 ns .. 1.03 us] (n=3) |
| U-root-interleaved | drop | 961 ns [887 ns .. 985 ns] (n=3) | 830 ns [798 ns .. 880 ns] (n=3) | 994 ns [921 ns .. 1 us] (n=3) |
| U-root-interleaved | retain | 1.16 us [1.07 us .. 1.21 us] (n=3) | 1.15 us [1.07 us .. 1.16 us] (n=3) | 1.21 us [1.12 us .. 1.23 us] (n=3) |
| U-root-max-tag | drop | 885 ns [799 ns .. 933 ns] (n=3) | 898 ns [861 ns .. 910 ns] (n=3) | 914 ns [881 ns .. 1.05 us] (n=3) |
| U-root-max-tag | retain | 910 ns [849 ns .. 965 ns] (n=3) | 869 ns [817 ns .. 977 ns] (n=3) | 937 ns [879 ns .. 938 ns] (n=3) |
| U-root-u-fixed32 | drop | 885 ns [883 ns .. 896 ns] (n=3) | 866 ns [796 ns .. 911 ns] (n=3) | 957 ns [949 ns .. 987 ns] (n=3) |
| U-root-u-fixed32 | retain | 974 ns [940 ns .. 1.01 us] (n=3) | 889 ns [877 ns .. 922 ns] (n=3) | 1.02 us [989 ns .. 1.07 us] (n=3) |
| U-root-u-fixed64 | drop | 901 ns [813 ns .. 909 ns] (n=3) | 878 ns [763 ns .. 919 ns] (n=3) | 970 ns [930 ns .. 981 ns] (n=3) |
| U-root-u-fixed64 | retain | 976 ns [943 ns .. 985 ns] (n=3) | 942 ns [927 ns .. 949 ns] (n=3) | 959 ns [875 ns .. 989 ns] (n=3) |
| U-root-u-len | drop | 841 ns [835 ns .. 917 ns] (n=3) | 906 ns [736 ns .. 954 ns] (n=3) | 935 ns [790 ns .. 945 ns] (n=3) |
| U-root-u-len | retain | 974 ns [906 ns .. 1.09 us] (n=3) | 851 ns [829 ns .. 906 ns] (n=3) | 990 ns [926 ns .. 1.04 us] (n=3) |
| U-root-u-msg | drop | 849 ns [831 ns .. 890 ns] (n=3) | 921 ns [848 ns .. 923 ns] (n=3) | 881 ns [864 ns .. 912 ns] (n=3) |
| U-root-u-msg | retain | 978 ns [852 ns .. 988 ns] (n=3) | 941 ns [864 ns .. 1.01 us] (n=3) | 975 ns [917 ns .. 1.02 us] (n=3) |
| U-root-u-packed | drop | 916 ns [794 ns .. 919 ns] (n=3) | 903 ns [875 ns .. 917 ns] (n=3) | 931 ns [853 ns .. 935 ns] (n=3) |
| U-root-u-packed | retain | 1.01 us [977 ns .. 1.03 us] (n=3) | 878 ns [849 ns .. 936 ns] (n=3) | 893 ns [881 ns .. 1.1 us] (n=3) |
| U-root-u-repeated | drop | 889 ns [880 ns .. 929 ns] (n=3) | 900 ns [861 ns .. 905 ns] (n=3) | 903 ns [880 ns .. 948 ns] (n=3) |
| U-root-u-repeated | retain | 980 ns [926 ns .. 989 ns] (n=3) | 846 ns [841 ns .. 922 ns] (n=3) | 984 ns [961 ns .. 1.13 us] (n=3) |
| U-root-u-varint | drop | 849 ns [816 ns .. 945 ns] (n=3) | 857 ns [814 ns .. 859 ns] (n=3) | 932 ns [815 ns .. 953 ns] (n=3) |
| U-root-u-varint | retain | 907 ns [903 ns .. 979 ns] (n=3) | 922 ns [877 ns .. 954 ns] (n=3) | 968 ns [958 ns .. 1 us] (n=3) |
| U-wire-DualResponse-left-as-wt0 | drop | 264 ns [263 ns .. 265 ns] (n=3) | 269 ns [268 ns .. 275 ns] (n=3) | 277 ns [273 ns .. 281 ns] (n=3) |
| U-wire-DualResponse-left-as-wt0 | retain | 334 ns [328 ns .. 368 ns] (n=3) | 337 ns [336 ns .. 338 ns] (n=3) | 355 ns [353 ns .. 369 ns] (n=3) |
| U-wire-DualResponse-left-as-wt1 | drop | 263 ns [263 ns .. 267 ns] (n=3) | 267 ns [266 ns .. 269 ns] (n=3) | 273 ns [273 ns .. 276 ns] (n=3) |
| U-wire-DualResponse-left-as-wt1 | retain | 332 ns [327 ns .. 369 ns] (n=3) | 338 ns [337 ns .. 342 ns] (n=3) | 345 ns [344 ns .. 345 ns] (n=3) |
| U-wire-DualResponse-left-as-wt5 | drop | 266 ns [264 ns .. 267 ns] (n=3) | 267 ns [266 ns .. 268 ns] (n=3) | 272 ns [272 ns .. 286 ns] (n=3) |
| U-wire-DualResponse-left-as-wt5 | retain | 333 ns [329 ns .. 361 ns] (n=3) | 336 ns [336 ns .. 349 ns] (n=3) | 344 ns [342 ns .. 345 ns] (n=3) |
| U-wire-ListMetricsResponse-batches-as-wt0 | drop | 1.89 us [1.89 us .. 1.97 us] (n=3) | 1.98 us [1.96 us .. 2 us] (n=3) | 2.57 us [2.52 us .. 2.61 us] (n=3) |
| U-wire-ListMetricsResponse-batches-as-wt0 | retain | 2.02 us [1.98 us .. 2.02 us] (n=3) | 2.1 us [2.05 us .. 2.13 us] (n=3) | 2.73 us [2.66 us .. 2.91 us] (n=3) |
| U-wire-ListMetricsResponse-batches-as-wt1 | drop | 1.93 us [1.91 us .. 1.97 us] (n=3) | 2.01 us [1.97 us .. 2.02 us] (n=3) | 2.53 us [2.51 us .. 2.62 us] (n=3) |
| U-wire-ListMetricsResponse-batches-as-wt1 | retain | 1.97 us [1.97 us .. 1.98 us] (n=3) | 2.12 us [2.06 us .. 2.19 us] (n=3) | 2.65 us [2.63 us .. 2.71 us] (n=3) |
| U-wire-ListMetricsResponse-batches-as-wt5 | drop | 1.91 us [1.89 us .. 1.94 us] (n=3) | 2.02 us [1.96 us .. 2.12 us] (n=3) | 2.56 us [2.53 us .. 2.56 us] (n=3) |
| U-wire-ListMetricsResponse-batches-as-wt5 | retain | 1.98 us [1.97 us .. 2 us] (n=3) | 2.08 us [2.06 us .. 2.24 us] (n=3) | 2.64 us [2.63 us .. 2.66 us] (n=3) |
| U-wire-ListProbeResponse-probes-as-wt0 | drop | 269 ns [268 ns .. 269 ns] (n=3) | 274 ns [274 ns .. 279 ns] (n=3) | 297 ns [290 ns .. 298 ns] (n=3) |
| U-wire-ListProbeResponse-probes-as-wt0 | retain | 335 ns [335 ns .. 335 ns] (n=3) | 343 ns [341 ns .. 346 ns] (n=3) | 361 ns [361 ns .. 377 ns] (n=3) |
| U-wire-ListProbeResponse-probes-as-wt1 | drop | 272 ns [272 ns .. 277 ns] (n=3) | 278 ns [273 ns .. 279 ns] (n=3) | 291 ns [290 ns .. 294 ns] (n=3) |
| U-wire-ListProbeResponse-probes-as-wt1 | retain | 336 ns [331 ns .. 338 ns] (n=3) | 351 ns [339 ns .. 375 ns] (n=3) | 358 ns [357 ns .. 364 ns] (n=3) |
| U-wire-ListProbeResponse-probes-as-wt5 | drop | 272 ns [270 ns .. 274 ns] (n=3) | 271 ns [269 ns .. 272 ns] (n=3) | 289 ns [285 ns .. 297 ns] (n=3) |
| U-wire-ListProbeResponse-probes-as-wt5 | retain | 338 ns [335 ns .. 342 ns] (n=3) | 347 ns [341 ns .. 350 ns] (n=3) | 359 ns [357 ns .. 360 ns] (n=3) |
| U-wire-ListResultsResponse-page-as-wt1 | drop | 891 ns [795 ns .. 914 ns] (n=3) | 853 ns [751 ns .. 950 ns] (n=3) | 851 ns [805 ns .. 912 ns] (n=3) |
| U-wire-ListResultsResponse-page-as-wt1 | retain | 962 ns [847 ns .. 996 ns] (n=3) | 921 ns [827 ns .. 1.06 us] (n=3) | 935 ns [890 ns .. 1.01 us] (n=3) |
| U-wire-ListResultsResponse-page-as-wt2 | drop | 878 ns [772 ns .. 897 ns] (n=3) | 802 ns [726 ns .. 875 ns] (n=3) | 933 ns [855 ns .. 1.06 us] (n=3) |
| U-wire-ListResultsResponse-page-as-wt2 | retain | 949 ns [903 ns .. 990 ns] (n=3) | 995 ns [985 ns .. 997 ns] (n=3) | 946 ns [932 ns .. 1.02 us] (n=3) |
| U-wire-ListResultsResponse-page-as-wt5 | drop | 899 ns [858 ns .. 935 ns] (n=3) | 850 ns [754 ns .. 918 ns] (n=3) | 874 ns [860 ns .. 987 ns] (n=3) |
| U-wire-ListResultsResponse-page-as-wt5 | retain | 961 ns [852 ns .. 974 ns] (n=3) | 991 ns [856 ns .. 1.09 us] (n=3) | 989 ns [899 ns .. 1.03 us] (n=3) |
| U-wire-ListResultsResponse-results-as-wt0 | drop | 890 ns [867 ns .. 911 ns] (n=3) | 858 ns [835 ns .. 900 ns] (n=3) | 877 ns [855 ns .. 885 ns] (n=3) |
| U-wire-ListResultsResponse-results-as-wt0 | retain | 975 ns [889 ns .. 986 ns] (n=3) | 848 ns [845 ns .. 986 ns] (n=3) | 993 ns [879 ns .. 1.1 us] (n=3) |
| U-wire-ListResultsResponse-results-as-wt1 | drop | 878 ns [834 ns .. 912 ns] (n=3) | 815 ns [770 ns .. 893 ns] (n=3) | 892 ns [839 ns .. 963 ns] (n=3) |
| U-wire-ListResultsResponse-results-as-wt1 | retain | 912 ns [792 ns .. 980 ns] (n=3) | 924 ns [907 ns .. 925 ns] (n=3) | 1.01 us [1 us .. 1.01 us] (n=3) |
| U-wire-ListResultsResponse-results-as-wt5 | drop | 889 ns [843 ns .. 907 ns] (n=3) | 933 ns [916 ns .. 949 ns] (n=3) | 959 ns [833 ns .. 986 ns] (n=3) |
| U-wire-ListResultsResponse-results-as-wt5 | retain | 961 ns [960 ns .. 1.02 us] (n=3) | 930 ns [902 ns .. 973 ns] (n=3) | 1.03 us [927 ns .. 1.06 us] (n=3) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt0 | drop | 2.6 us [2.43 us .. 2.63 us] (n=3) | 2.5 us [2.45 us .. 2.65 us] (n=3) | 2.73 us [2.53 us .. 2.75 us] (n=3) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt0 | retain | 2.68 us [2.51 us .. 2.71 us] (n=3) | 2.57 us [2.52 us .. 2.61 us] (n=3) | 2.78 us [2.63 us .. 2.93 us] (n=3) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt1 | drop | 2.57 us [2.41 us .. 2.6 us] (n=3) | 2.48 us [2.43 us .. 2.52 us] (n=3) | 2.64 us [2.49 us .. 2.66 us] (n=3) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt1 | retain | 2.68 us [2.49 us .. 2.71 us] (n=3) | 2.65 us [2.54 us .. 2.74 us] (n=3) | 2.68 us [2.63 us .. 2.76 us] (n=3) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | drop | 2.59 us [2.45 us .. 2.9 us] (n=3) | 2.56 us [2.43 us .. 2.66 us] (n=3) | 2.76 us [2.6 us .. 2.79 us] (n=3) |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | retain | 2.72 us [2.69 us .. 2.73 us] (n=3) | 2.54 us [2.51 us .. 2.58 us] (n=3) | 2.78 us [2.62 us .. 2.8 us] (n=3) |
| U-wire-ListTasksDetailedResponse-page-as-wt1 | drop | 5.72 us [5.49 us .. 5.82 us] (n=3) | 5.42 us [5.42 us .. 5.58 us] (n=3) | 5.55 us [5.53 us .. 5.71 us] (n=3) |
| U-wire-ListTasksDetailedResponse-page-as-wt1 | retain | 5.86 us [5.86 us .. 5.9 us] (n=3) | 5.51 us [5.39 us .. 5.78 us] (n=3) | 5.85 us [5.6 us .. 6.1 us] (n=3) |
| U-wire-ListTasksDetailedResponse-page-as-wt2 | drop | 5.54 us [5.44 us .. 5.6 us] (n=3) | 5.29 us [5.25 us .. 5.4 us] (n=3) | 5.71 us [5.71 us .. 5.81 us] (n=3) |
| U-wire-ListTasksDetailedResponse-page-as-wt2 | retain | 5.87 us [5.56 us .. 5.97 us] (n=3) | 5.49 us [5.46 us .. 5.52 us] (n=3) | 5.85 us [5.83 us .. 5.94 us] (n=3) |
| U-wire-ListTasksDetailedResponse-page-as-wt5 | drop | 5.68 us [5.5 us .. 5.81 us] (n=3) | 5.26 us [5.23 us .. 5.54 us] (n=3) | 5.67 us [5.59 us .. 5.9 us] (n=3) |
| U-wire-ListTasksDetailedResponse-page-as-wt5 | retain | 5.73 us [5.67 us .. 5.84 us] (n=3) | 5.52 us [5.35 us .. 5.73 us] (n=3) | 5.83 us [5.7 us .. 5.88 us] (n=3) |
| U-wire-ListTasksDetailedResponse-tasks-as-wt0 | drop | 5.61 us [5.56 us .. 5.87 us] (n=3) | 5.35 us [5.25 us .. 5.43 us] (n=3) | 5.73 us [5.56 us .. 5.86 us] (n=3) |
| U-wire-ListTasksDetailedResponse-tasks-as-wt0 | retain | 5.86 us [5.75 us .. 5.89 us] (n=3) | 5.41 us [5.4 us .. 6.1 us] (n=3) | 5.67 us [5.65 us .. 5.89 us] (n=3) |
| U-wire-ListTasksDetailedResponse-tasks-as-wt1 | drop | 5.76 us [5.6 us .. 5.88 us] (n=3) | 5.44 us [5.38 us .. 5.6 us] (n=3) | 5.72 us [5.62 us .. 6.06 us] (n=3) |
| U-wire-ListTasksDetailedResponse-tasks-as-wt1 | retain | 6.13 us [5.73 us .. 6.22 us] (n=3) | 5.39 us [5.38 us .. 5.78 us] (n=3) | 5.9 us [5.59 us .. 5.96 us] (n=3) |
| U-wire-ListTasksDetailedResponse-tasks-as-wt5 | drop | 5.7 us [5.61 us .. 5.81 us] (n=3) | 5.47 us [5.38 us .. 6.31 us] (n=3) | 5.76 us [5.75 us .. 6.07 us] (n=3) |
| U-wire-ListTasksDetailedResponse-tasks-as-wt5 | retain | 5.73 us [5.6 us .. 5.86 us] (n=3) | 5.63 us [5.52 us .. 5.65 us] (n=3) | 5.86 us [5.81 us .. 5.9 us] (n=3) |
| U-wire-UploadResultDataMessage-upload-as-wt0 | drop | 118 ns [117 ns .. 120 ns] (n=3) | 125 ns [125 ns .. 127 ns] (n=3) | 133 ns [132 ns .. 135 ns] (n=3) |
| U-wire-UploadResultDataMessage-upload-as-wt0 | retain | 174 ns [173 ns .. 178 ns] (n=3) | 183 ns [182 ns .. 184 ns] (n=3) | 192 ns [190 ns .. 199 ns] (n=3) |
| U-wire-UploadResultDataMessage-upload-as-wt1 | drop | 117 ns [116 ns .. 120 ns] (n=3) | 124 ns [123 ns .. 127 ns] (n=3) | 133 ns [133 ns .. 137 ns] (n=3) |
| U-wire-UploadResultDataMessage-upload-as-wt1 | retain | 172 ns [172 ns .. 175 ns] (n=3) | 185 ns [181 ns .. 188 ns] (n=3) | 190 ns [189 ns .. 191 ns] (n=3) |
| U-wire-UploadResultDataMessage-upload-as-wt5 | drop | 117 ns [115 ns .. 126 ns] (n=3) | 130 ns [124 ns .. 131 ns] (n=3) | 135 ns [131 ns .. 143 ns] (n=3) |
| U-wire-UploadResultDataMessage-upload-as-wt5 | retain | 172 ns [172 ns .. 175 ns] (n=3) | 182 ns [181 ns .. 188 ns] (n=3) | 189 ns [188 ns .. 189 ns] (n=3) |

## Events per decode (counting build, `fsm_diff --no-malformed`)

FSM events = pull records on every row (checked record for record); FSM calls = begin + (events - 1) next. Forward crossings counted in the core: pull = 1 (ak_parse_*), FSM = its calls; reverse = unknown-field grows (retain).

| input | mode | bytes | pull records | FSM events | FSM calls | pull fwd | FSM fwd | pull rev | FSM rev |
|---|---|---|---|---|---|---|---|---|---|
| P1.1 | drop | 858 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| P1.1 | no-unknown | 858 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| P1.1 | retain | 858 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| P1.2 | drop | 218121 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| P1.2 | no-unknown | 218121 | 5 | 5 | 5 | 1 | 5 | 0 | 0 |
| P1.2 | retain | 218121 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| P1.2/latin1 | drop | 370110 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| P1.2/latin1 | no-unknown | 370110 | 5 | 5 | 5 | 1 | 5 | 0 | 0 |
| P1.2/latin1 | retain | 370110 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| P1.2/wide | drop | 522099 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| P1.2/wide | no-unknown | 522099 | 5 | 5 | 5 | 1 | 5 | 0 | 0 |
| P1.2/wide | retain | 522099 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| P1.3 | drop | 605 | 3 | 3 | 3 | 1 | 3 | 0 | 0 |
| P1.3 | no-unknown | 605 | 3 | 3 | 3 | 1 | 3 | 0 | 0 |
| P1.3 | retain | 605 | 3 | 3 | 3 | 1 | 3 | 0 | 0 |
| P2.1 | drop | 1037 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| P2.1 | no-unknown | 1037 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| P2.1 | retain | 1037 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| P2.2 | drop | 540422 | 3501 | 3501 | 3501 | 1 | 3501 | 0 | 0 |
| P2.2 | no-unknown | 540422 | 3501 | 3501 | 3501 | 1 | 3501 | 0 | 0 |
| P2.2 | retain | 540422 | 3501 | 3501 | 3501 | 1 | 3501 | 0 | 0 |
| P2.2/latin1 | drop | 944454 | 3501 | 3501 | 3501 | 1 | 3501 | 0 | 0 |
| P2.2/latin1 | no-unknown | 944454 | 3501 | 3501 | 3501 | 1 | 3501 | 0 | 0 |
| P2.2/latin1 | retain | 944454 | 3501 | 3501 | 3501 | 1 | 3501 | 0 | 0 |
| P2.2/wide | drop | 1348486 | 3501 | 3501 | 3501 | 1 | 3501 | 0 | 0 |
| P2.2/wide | no-unknown | 1348486 | 3501 | 3501 | 3501 | 1 | 3501 | 0 | 0 |
| P2.2/wide | retain | 1348486 | 3501 | 3501 | 3501 | 1 | 3501 | 0 | 0 |
| P2.3 | drop | 647024 | 876 | 876 | 876 | 1 | 876 | 0 | 0 |
| P2.3 | no-unknown | 647024 | 876 | 876 | 876 | 1 | 876 | 0 | 0 |
| P2.3 | retain | 647024 | 876 | 876 | 876 | 1 | 876 | 0 | 0 |
| P2.4 | drop | 979465 | 561 | 561 | 561 | 1 | 561 | 0 | 0 |
| P2.4 | no-unknown | 979465 | 561 | 561 | 561 | 1 | 561 | 0 | 0 |
| P2.4 | retain | 979465 | 561 | 561 | 561 | 1 | 561 | 0 | 0 |
| P2.4/latin1 | drop | 1890741 | 561 | 561 | 561 | 1 | 561 | 0 | 0 |
| P2.4/latin1 | no-unknown | 1890741 | 561 | 561 | 561 | 1 | 561 | 0 | 0 |
| P2.4/latin1 | retain | 1890741 | 561 | 561 | 561 | 1 | 561 | 0 | 0 |
| P2.4/wide | drop | 2802017 | 561 | 561 | 561 | 1 | 561 | 0 | 0 |
| P2.4/wide | no-unknown | 2802017 | 561 | 561 | 561 | 1 | 561 | 0 | 0 |
| P2.4/wide | retain | 2802017 | 561 | 561 | 561 | 1 | 561 | 0 | 0 |
| P2.5 | drop | 19632 | 141 | 141 | 141 | 1 | 141 | 0 | 0 |
| P2.5 | no-unknown | 19632 | 141 | 141 | 141 | 1 | 141 | 0 | 0 |
| P2.5 | retain | 19632 | 141 | 141 | 141 | 1 | 141 | 0 | 0 |
| P3.1 | drop | 12097 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| P3.1 | no-unknown | 12097 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| P3.1 | retain | 12097 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| P4.1 | drop | 65321 | 601 | 601 | 601 | 1 | 601 | 0 | 0 |
| P4.1 | no-unknown | 65321 | 601 | 601 | 601 | 1 | 601 | 0 | 0 |
| P4.1 | retain | 65321 | 601 | 601 | 601 | 1 | 601 | 0 | 0 |
| P5.1 | drop | 116 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| P5.1 | no-unknown | 116 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| P5.1 | retain | 116 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| P5.2 | drop | 65620 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| P5.2 | no-unknown | 65620 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| P5.2 | retain | 65620 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| P5.3 | drop | 1048660 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| P5.3 | no-unknown | 1048660 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| P5.3 | retain | 1048660 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| P5.4 | drop | 4194390 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| P5.4 | no-unknown | 4194390 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| P5.4 | retain | 4194390 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| P6.1 | drop | 123354 | 1401 | 1401 | 1401 | 1 | 1401 | 0 | 0 |
| P6.1 | no-unknown | 123354 | 1401 | 1401 | 1401 | 1 | 1401 | 0 | 0 |
| P6.1 | retain | 123354 | 1401 | 1401 | 1401 | 1 | 1401 | 0 | 0 |
| P7.1 | drop | 98 | 7 | 7 | 7 | 1 | 7 | 0 | 0 |
| P7.1 | no-unknown | 98 | 7 | 7 | 7 | 1 | 7 | 0 | 0 |
| P7.1 | retain | 98 | 7 | 7 | 7 | 1 | 7 | 0 | 0 |
| SYN-packed-4096-4096-32768 | drop | 114374 | 13 | 13 | 13 | 1 | 13 | 0 | 0 |
| SYN-packed-4096-4096-32768 | no-unknown | 114374 | 13 | 13 | 13 | 1 | 13 | 0 | 0 |
| SYN-packed-4096-4096-32768 | retain | 114374 | 13 | 13 | 13 | 1 | 13 | 0 | 0 |
| SYN-packed-5000-4097-40000 | drop | 124920 | 17 | 17 | 17 | 1 | 17 | 0 | 0 |
| SYN-packed-5000-4097-40000 | no-unknown | 124920 | 17 | 17 | 17 | 1 | 17 | 0 | 0 |
| SYN-packed-5000-4097-40000 | retain | 124920 | 17 | 17 | 17 | 1 | 17 | 0 | 0 |
| SYN-packed-70000-9000-70000 | drop | 470403 | 56 | 56 | 56 | 1 | 56 | 0 | 0 |
| SYN-packed-70000-9000-70000 | no-unknown | 470403 | 56 | 56 | 56 | 1 | 56 | 0 | 0 |
| SYN-packed-70000-9000-70000 | retain | 470403 | 56 | 56 | 56 | 1 | 56 | 0 | 0 |
| SYN-strings-5000 | drop | 103104 | 10 | 10 | 10 | 1 | 10 | 0 | 0 |
| SYN-strings-5000 | no-unknown | 103104 | 10 | 10 | 10 | 1 | 10 | 0 | 0 |
| SYN-strings-5000 | retain | 103104 | 10 | 10 | 10 | 1 | 10 | 0 | 0 |
| U-deep-all | drop | 944 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-all | no-unknown | 944 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-all | retain | 944 | 8 | 8 | 8 | 1 | 8 | 8 | 8 |
| U-deep-u-fixed32 | drop | 853 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-u-fixed32 | no-unknown | 853 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-u-fixed32 | retain | 853 | 8 | 8 | 8 | 1 | 8 | 1 | 1 |
| U-deep-u-fixed64 | drop | 857 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-u-fixed64 | no-unknown | 857 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-u-fixed64 | retain | 857 | 8 | 8 | 8 | 1 | 8 | 1 | 1 |
| U-deep-u-len | drop | 858 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-u-len | no-unknown | 858 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-u-len | retain | 858 | 8 | 8 | 8 | 1 | 8 | 1 | 1 |
| U-deep-u-msg | drop | 856 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-u-msg | no-unknown | 856 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-u-msg | retain | 856 | 8 | 8 | 8 | 1 | 8 | 1 | 1 |
| U-deep-u-packed | drop | 874 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-u-packed | no-unknown | 874 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-u-packed | retain | 874 | 8 | 8 | 8 | 1 | 8 | 1 | 1 |
| U-deep-u-repeated | drop | 873 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-u-repeated | no-unknown | 873 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-u-repeated | retain | 873 | 8 | 8 | 8 | 1 | 8 | 2 | 2 |
| U-deep-u-varint | drop | 855 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-u-varint | no-unknown | 855 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-deep-u-varint | retain | 855 | 8 | 8 | 8 | 1 | 8 | 1 | 1 |
| U-element-all | drop | 944 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-all | no-unknown | 944 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-all | retain | 944 | 8 | 8 | 8 | 1 | 8 | 8 | 8 |
| U-element-u-fixed32 | drop | 853 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-u-fixed32 | no-unknown | 853 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-u-fixed32 | retain | 853 | 8 | 8 | 8 | 1 | 8 | 1 | 1 |
| U-element-u-fixed64 | drop | 857 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-u-fixed64 | no-unknown | 857 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-u-fixed64 | retain | 857 | 8 | 8 | 8 | 1 | 8 | 1 | 1 |
| U-element-u-len | drop | 858 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-u-len | no-unknown | 858 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-u-len | retain | 858 | 8 | 8 | 8 | 1 | 8 | 1 | 1 |
| U-element-u-msg | drop | 856 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-u-msg | no-unknown | 856 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-u-msg | retain | 856 | 8 | 8 | 8 | 1 | 8 | 1 | 1 |
| U-element-u-packed | drop | 874 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-u-packed | no-unknown | 874 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-u-packed | retain | 874 | 8 | 8 | 8 | 1 | 8 | 1 | 1 |
| U-element-u-repeated | drop | 873 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-u-repeated | no-unknown | 873 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-u-repeated | retain | 873 | 8 | 8 | 8 | 1 | 8 | 2 | 2 |
| U-element-u-varint | drop | 855 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-u-varint | no-unknown | 855 | 8 | 8 | 8 | 1 | 8 | 0 | 0 |
| U-element-u-varint | retain | 855 | 8 | 8 | 8 | 1 | 8 | 1 | 1 |
| U-enum-value-127 | drop | 16 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-enum-value-127 | no-unknown | 16 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-enum-value-127 | retain | 16 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-enum-value-2147483647 | drop | 20 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-enum-value-2147483647 | no-unknown | 20 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-enum-value-2147483647 | retain | 20 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-enum-value-999 | drop | 17 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-enum-value-999 | no-unknown | 17 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-enum-value-999 | retain | 17 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-enum-value-packed | drop | 16 | 4 | 4 | 4 | 1 | 4 | 0 | 0 |
| U-enum-value-packed | no-unknown | 16 | 4 | 4 | 4 | 1 | 4 | 0 | 0 |
| U-enum-value-packed | retain | 16 | 4 | 4 | 4 | 1 | 4 | 0 | 0 |
| U-leaf-all | drop | 1317 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-all | no-unknown | 1317 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-all | retain | 1317 | 2 | 2 | 2 | 1 | 2 | 16 | 16 |
| U-leaf-u-fixed32 | drop | 222 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-u-fixed32 | no-unknown | 222 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-u-fixed32 | retain | 222 | 2 | 2 | 2 | 1 | 2 | 2 | 2 |
| U-leaf-u-fixed64 | drop | 230 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-u-fixed64 | no-unknown | 230 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-u-fixed64 | retain | 230 | 2 | 2 | 2 | 1 | 2 | 2 | 2 |
| U-leaf-u-len | drop | 233 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-u-len | no-unknown | 233 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-u-len | retain | 233 | 2 | 2 | 2 | 1 | 2 | 2 | 2 |
| U-leaf-u-msg | drop | 300 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-u-msg | no-unknown | 300 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-u-msg | retain | 300 | 2 | 2 | 2 | 1 | 2 | 2 | 2 |
| U-leaf-u-packed | drop | 264 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-u-packed | no-unknown | 264 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-u-packed | retain | 264 | 2 | 2 | 2 | 1 | 2 | 2 | 2 |
| U-leaf-u-repeated | drop | 254 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-u-repeated | no-unknown | 254 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-u-repeated | retain | 254 | 2 | 2 | 2 | 1 | 2 | 4 | 4 |
| U-leaf-u-varint | drop | 226 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-u-varint | no-unknown | 226 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-leaf-u-varint | retain | 226 | 2 | 2 | 2 | 1 | 2 | 2 | 2 |
| U-nested-all | drop | 303 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-all | no-unknown | 303 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-all | retain | 303 | 2 | 2 | 2 | 1 | 2 | 8 | 8 |
| U-nested-before | drop | 303 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-before | no-unknown | 303 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-before | retain | 303 | 2 | 2 | 2 | 1 | 2 | 8 | 8 |
| U-nested-group | drop | 244 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-group | no-unknown | 244 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-group | retain | 244 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-nested-interleaved | drop | 303 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-interleaved | no-unknown | 303 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-interleaved | retain | 303 | 2 | 2 | 2 | 1 | 2 | 8 | 8 |
| U-nested-u-fixed32 | drop | 216 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-u-fixed32 | no-unknown | 216 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-u-fixed32 | retain | 216 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-nested-u-fixed64 | drop | 220 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-u-fixed64 | no-unknown | 220 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-u-fixed64 | retain | 220 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-nested-u-len | drop | 221 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-u-len | no-unknown | 221 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-u-len | retain | 221 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-nested-u-msg | drop | 219 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-u-msg | no-unknown | 219 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-u-msg | retain | 219 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-nested-u-packed | drop | 237 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-u-packed | no-unknown | 237 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-u-packed | retain | 237 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-nested-u-repeated | drop | 232 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-u-repeated | no-unknown | 232 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-u-repeated | retain | 232 | 2 | 2 | 2 | 1 | 2 | 2 | 2 |
| U-nested-u-varint | drop | 218 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-u-varint | no-unknown | 218 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-nested-u-varint | retain | 218 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-oneof-all | drop | 153 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-all | no-unknown | 153 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-all | retain | 153 | 2 | 2 | 2 | 1 | 2 | 8 | 8 |
| U-oneof-before | drop | 153 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-before | no-unknown | 153 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-before | retain | 153 | 2 | 2 | 2 | 1 | 2 | 8 | 8 |
| U-oneof-group | drop | 95 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-group | no-unknown | 95 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-group | retain | 95 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-oneof-interleaved | drop | 153 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-interleaved | no-unknown | 153 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-interleaved | retain | 153 | 2 | 2 | 2 | 1 | 2 | 8 | 8 |
| U-oneof-member-after-known | drop | 70 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-member-after-known | no-unknown | 70 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-member-after-known | retain | 70 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-oneof-member-alone | drop | 63 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-member-alone | no-unknown | 63 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-member-alone | retain | 63 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-oneof-member-before-known | drop | 70 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-member-before-known | no-unknown | 70 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-member-before-known | retain | 70 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-oneof-u-fixed32 | drop | 67 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-u-fixed32 | no-unknown | 67 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-u-fixed32 | retain | 67 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-oneof-u-fixed64 | drop | 71 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-u-fixed64 | no-unknown | 71 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-u-fixed64 | retain | 71 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-oneof-u-len | drop | 71 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-u-len | no-unknown | 71 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-u-len | retain | 71 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-oneof-u-msg | drop | 70 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-u-msg | no-unknown | 70 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-u-msg | retain | 70 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-oneof-u-packed | drop | 87 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-u-packed | no-unknown | 87 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-u-packed | retain | 87 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-oneof-u-repeated | drop | 83 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-u-repeated | no-unknown | 83 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-u-repeated | retain | 83 | 2 | 2 | 2 | 1 | 2 | 2 | 2 |
| U-oneof-u-varint | drop | 69 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-u-varint | no-unknown | 69 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-oneof-u-varint | retain | 69 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-root-all | drop | 523 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-all | no-unknown | 523 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-all | retain | 523 | 2 | 2 | 2 | 1 | 2 | 8 | 8 |
| U-root-before | drop | 523 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-before | no-unknown | 523 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-before | retain | 523 | 2 | 2 | 2 | 1 | 2 | 8 | 8 |
| U-root-group | drop | 461 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-group | no-unknown | 461 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-group | retain | 461 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-root-interleaved | drop | 523 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-interleaved | no-unknown | 523 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-interleaved | retain | 523 | 2 | 2 | 2 | 1 | 2 | 8 | 8 |
| U-root-max-tag | drop | 433 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-max-tag | no-unknown | 433 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-max-tag | retain | 433 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-root-u-fixed32 | drop | 433 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-u-fixed32 | no-unknown | 433 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-u-fixed32 | retain | 433 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-root-u-fixed64 | drop | 437 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-u-fixed64 | no-unknown | 437 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-u-fixed64 | retain | 437 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-root-u-len | drop | 440 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-u-len | no-unknown | 440 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-u-len | retain | 440 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-root-u-msg | drop | 436 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-u-msg | no-unknown | 436 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-u-msg | retain | 436 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-root-u-packed | drop | 454 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-u-packed | no-unknown | 454 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-u-packed | retain | 454 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-root-u-repeated | drop | 450 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-u-repeated | no-unknown | 450 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-u-repeated | retain | 450 | 2 | 2 | 2 | 1 | 2 | 2 | 2 |
| U-root-u-varint | drop | 435 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-u-varint | no-unknown | 435 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-root-u-varint | retain | 435 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-wire-DualResponse-left-as-wt0 | drop | 67 | 3 | 3 | 3 | 1 | 3 | 0 | 0 |
| U-wire-DualResponse-left-as-wt0 | no-unknown | 67 | 3 | 3 | 3 | 1 | 3 | 0 | 0 |
| U-wire-DualResponse-left-as-wt0 | retain | 67 | 3 | 3 | 3 | 1 | 3 | 1 | 1 |
| U-wire-DualResponse-left-as-wt1 | drop | 73 | 3 | 3 | 3 | 1 | 3 | 0 | 0 |
| U-wire-DualResponse-left-as-wt1 | no-unknown | 73 | 3 | 3 | 3 | 1 | 3 | 0 | 0 |
| U-wire-DualResponse-left-as-wt1 | retain | 73 | 3 | 3 | 3 | 1 | 3 | 1 | 1 |
| U-wire-DualResponse-left-as-wt5 | drop | 69 | 3 | 3 | 3 | 1 | 3 | 0 | 0 |
| U-wire-DualResponse-left-as-wt5 | no-unknown | 69 | 3 | 3 | 3 | 1 | 3 | 0 | 0 |
| U-wire-DualResponse-left-as-wt5 | retain | 69 | 3 | 3 | 3 | 1 | 3 | 1 | 1 |
| U-wire-ListMetricsResponse-batches-as-wt0 | drop | 1243 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListMetricsResponse-batches-as-wt0 | no-unknown | 1243 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListMetricsResponse-batches-as-wt0 | retain | 1243 | 15 | 15 | 15 | 1 | 15 | 1 | 1 |
| U-wire-ListMetricsResponse-batches-as-wt1 | drop | 1249 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListMetricsResponse-batches-as-wt1 | no-unknown | 1249 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListMetricsResponse-batches-as-wt1 | retain | 1249 | 15 | 15 | 15 | 1 | 15 | 1 | 1 |
| U-wire-ListMetricsResponse-batches-as-wt5 | drop | 1245 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListMetricsResponse-batches-as-wt5 | no-unknown | 1245 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListMetricsResponse-batches-as-wt5 | retain | 1245 | 15 | 15 | 15 | 1 | 15 | 1 | 1 |
| U-wire-ListProbeResponse-probes-as-wt0 | drop | 134 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListProbeResponse-probes-as-wt0 | no-unknown | 134 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListProbeResponse-probes-as-wt0 | retain | 134 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-wire-ListProbeResponse-probes-as-wt1 | drop | 140 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListProbeResponse-probes-as-wt1 | no-unknown | 140 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListProbeResponse-probes-as-wt1 | retain | 140 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-wire-ListProbeResponse-probes-as-wt5 | drop | 136 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListProbeResponse-probes-as-wt5 | no-unknown | 136 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListProbeResponse-probes-as-wt5 | retain | 136 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-wire-ListResultsResponse-page-as-wt1 | drop | 436 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListResultsResponse-page-as-wt1 | no-unknown | 436 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListResultsResponse-page-as-wt1 | retain | 436 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-wire-ListResultsResponse-page-as-wt2 | drop | 432 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListResultsResponse-page-as-wt2 | no-unknown | 432 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListResultsResponse-page-as-wt2 | retain | 432 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-wire-ListResultsResponse-page-as-wt5 | drop | 432 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListResultsResponse-page-as-wt5 | no-unknown | 432 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListResultsResponse-page-as-wt5 | retain | 432 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-wire-ListResultsResponse-results-as-wt0 | drop | 430 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListResultsResponse-results-as-wt0 | no-unknown | 430 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListResultsResponse-results-as-wt0 | retain | 430 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-wire-ListResultsResponse-results-as-wt1 | drop | 436 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListResultsResponse-results-as-wt1 | no-unknown | 436 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListResultsResponse-results-as-wt1 | retain | 436 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-wire-ListResultsResponse-results-as-wt5 | drop | 432 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListResultsResponse-results-as-wt5 | no-unknown | 432 | 2 | 2 | 2 | 1 | 2 | 0 | 0 |
| U-wire-ListResultsResponse-results-as-wt5 | retain | 432 | 2 | 2 | 2 | 1 | 2 | 1 | 1 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt0 | drop | 613 | 7 | 7 | 7 | 1 | 7 | 0 | 0 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt0 | no-unknown | 613 | 7 | 7 | 7 | 1 | 7 | 0 | 0 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt0 | retain | 613 | 7 | 7 | 7 | 1 | 7 | 1 | 1 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt1 | drop | 619 | 7 | 7 | 7 | 1 | 7 | 0 | 0 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt1 | no-unknown | 619 | 7 | 7 | 7 | 1 | 7 | 0 | 0 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt1 | retain | 619 | 7 | 7 | 7 | 1 | 7 | 1 | 1 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | drop | 615 | 7 | 7 | 7 | 1 | 7 | 0 | 0 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | no-unknown | 615 | 7 | 7 | 7 | 1 | 7 | 0 | 0 |
| U-wire-ListTaskSummaryResponse-tasks-as-wt5 | retain | 615 | 7 | 7 | 7 | 1 | 7 | 1 | 1 |
| U-wire-ListTasksDetailedResponse-page-as-wt1 | drop | 1752 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListTasksDetailedResponse-page-as-wt1 | no-unknown | 1752 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListTasksDetailedResponse-page-as-wt1 | retain | 1752 | 15 | 15 | 15 | 1 | 15 | 1 | 1 |
| U-wire-ListTasksDetailedResponse-page-as-wt2 | drop | 1748 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListTasksDetailedResponse-page-as-wt2 | no-unknown | 1748 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListTasksDetailedResponse-page-as-wt2 | retain | 1748 | 15 | 15 | 15 | 1 | 15 | 1 | 1 |
| U-wire-ListTasksDetailedResponse-page-as-wt5 | drop | 1748 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListTasksDetailedResponse-page-as-wt5 | no-unknown | 1748 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListTasksDetailedResponse-page-as-wt5 | retain | 1748 | 15 | 15 | 15 | 1 | 15 | 1 | 1 |
| U-wire-ListTasksDetailedResponse-tasks-as-wt0 | drop | 1746 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListTasksDetailedResponse-tasks-as-wt0 | no-unknown | 1746 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListTasksDetailedResponse-tasks-as-wt0 | retain | 1746 | 15 | 15 | 15 | 1 | 15 | 1 | 1 |
| U-wire-ListTasksDetailedResponse-tasks-as-wt1 | drop | 1752 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListTasksDetailedResponse-tasks-as-wt1 | no-unknown | 1752 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListTasksDetailedResponse-tasks-as-wt1 | retain | 1752 | 15 | 15 | 15 | 1 | 15 | 1 | 1 |
| U-wire-ListTasksDetailedResponse-tasks-as-wt5 | drop | 1748 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListTasksDetailedResponse-tasks-as-wt5 | no-unknown | 1748 | 15 | 15 | 15 | 1 | 15 | 0 | 0 |
| U-wire-ListTasksDetailedResponse-tasks-as-wt5 | retain | 1748 | 15 | 15 | 15 | 1 | 15 | 1 | 1 |
| U-wire-UploadResultDataMessage-upload-as-wt0 | drop | 99 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| U-wire-UploadResultDataMessage-upload-as-wt0 | no-unknown | 99 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| U-wire-UploadResultDataMessage-upload-as-wt0 | retain | 99 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
| U-wire-UploadResultDataMessage-upload-as-wt1 | drop | 105 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| U-wire-UploadResultDataMessage-upload-as-wt1 | no-unknown | 105 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| U-wire-UploadResultDataMessage-upload-as-wt1 | retain | 105 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
| U-wire-UploadResultDataMessage-upload-as-wt5 | drop | 101 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| U-wire-UploadResultDataMessage-upload-as-wt5 | no-unknown | 101 | 1 | 1 | 1 | 1 | 1 | 0 | 0 |
| U-wire-UploadResultDataMessage-upload-as-wt5 | retain | 101 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
