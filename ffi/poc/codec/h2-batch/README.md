# h2-batch: an opt-in h2 variant of the core

The core has two h2 variants (owner decision, 2026-10-01):

| Variant | h2 | How it is built |
|---|---|---|
| **stock** (the default) | crates.io h2 0.4.19, as locked in `poc/codec/Cargo.lock` | every ordinary build: no `[patch]` exists in any manifest |
| **h2-batch** | h2 0.4.19 + `h2-batch.patch` | `build.sh h2-batch TARGET_DIR`, or the same `--config` by hand (below) |

## What the patch is

`h2-batch.patch` is the `src/` part of `git diff v0.4.19 4861bb0` in a checkout of
github.com/hyperium/h2. It holds three changes, applied in this order:

1. h2 PR 903 ("perf: allow multiple DATA frames per write", hyperium/h2#903, head a1f880b),
   ported onto v0.4.19 (tree 221c21e). The port had one defect, which is fixed: DATA frames
   need a free queue slot, while other frames need only buffer room.
2. p4 combined with it (tree b871798). One stream's queued element may span AK_H2_COALESCE
   max-size DATA frames. The element is written as consecutive frames, each with its own
   head and with END_STREAM only on the last, inside the same vectored write as the other
   queued elements.
3. AK_H2_COALESCE defaults to 16 (tree 4861bb0, the only change from b871798). It is read
   once per process. AK_H2_COALESCE=1 gives the PR-903 behaviour.

The test-support change of the full tree diff (`tests/h2-support/src/mock.rs`) is not in the
crate, so it is not in this file. The crates.io `.crate` of h2 0.4.19 was packaged from
d57d1b8 (its `.cargo_vcs_info.json`), and its `src/` is identical to the v0.4.19 tag
(checked with `diff -r`).

| File | sha256 |
|---|---|
| `h2-batch.patch` (`git diff v0.4.19 4861bb0 -- src`) | c64ffd965ba5a39a29ee5f98e1e51850b148af36d677b62bc17dda778ba9116f |
| full tree diff `git diff v0.4.19 4861bb0` (not committed here) | e88c777d3c7975de78e52c105aa74a6103f82b66c0be6a4cdda61e17f7ae7091 |
| h2-0.4.19.crate (= Cargo.lock's checksum) | ef8e5e5a340588f4452631496976cf8636d4a7ecf600239fdc27615d2530bc16 |

The evidence for the patch is under `ffi/logs/rust/opt/patches/h2-pr903/` and
`h2-pr903-p4/`: h2's own tests against stock 0.4.19, the checks, write counts and timings.
There is one known difference from stock in h2's own suite, at N > 1:
`stream_states::send_err_with_buffered_data` fails. When a stream is reset while its data
is queued, the sub-frames already queued go out before the RST_STREAM. p4 alone at N=16
fails the same test.

## Known divergences from stock h2 (kept as is, owner 2026-10-02, D16)

The patch is kept unchanged for the rest of the POC. What it changes besides the write
pattern:

1. **Data after a local reset.** The prioritizer hands the frame writer one element of up
   to AK_H2_COALESCE max-size DATA frames of one stream (16 x 16 KiB = 256 KiB by default).
   A reset of the stream after that hand-off (`ak_call_cancel`, a dropped call) drops only
   what was not handed over, so up to AK_H2_COALESCE - 1 further DATA frames go out before
   the RST_STREAM, the last one possibly carrying END_STREAM. Stock h2 sends at most the
   one frame in flight (h2's test `stream_states::send_err_with_buffered_data`). This is
   legal HTTP/2 (RFC 9113 sections 5.1 and 6.4: DATA may precede RST_STREAM) and stays
   within flow control (the send window is taken when the element is handed over). Its
   effect: a cancel racing the end of an upload is more likely to deliver the complete
   request body, END_STREAM included, before the reset (a window of 256 KiB instead of
   16 KiB).
2. **Control frames behind a burst (not tested).** A PING, SETTINGS ack, WINDOW_UPDATE or
   GOAWAY queued during a burst waits behind up to 256 KiB instead of 16 KiB: microseconds
   on loopback, about 2 ms at 1 Gbit/s. A SETTINGS, PING or GOAWAY arriving mid-burst is
   not covered by any check.

Not done: a cancel that stops at the next sub-frame boundary, tests of control frames
mid-burst, a right-sized IoSlice array (PR 903's write path).

## Building

```sh
# the stock core, the same as the default build
poc/codec/h2-batch/build.sh stock    /some/target-stock    [FEATURES]
# the h2-batch core
poc/codec/h2-batch/build.sh h2-batch /some/target-h2batch  [FEATURES]
```

- FEATURES defaults to `rpc,init-guard`, and unknown-fields is on by default. The library
  lands in `TARGET_DIR/release/libak_core.{so,a}`.
- For h2-batch the script works in this order:
  1. It extracts h2 from cargo's cached `.crate`. If the crate is not cached, it runs
     `cargo fetch` first.
  2. It checks the crate's sha256 against `Cargo.lock`.
  3. It patches the source into `TARGET_DIR/h2-batch-src`.
  4. It builds with `cargo --config 'patch.crates-io.h2.path="TARGET_DIR/h2-batch-src"'`.
- A path patch makes cargo rewrite `Cargo.lock`, and the script restores it on exit.
- The script prints the library's sha256 and which h2 source is compiled in.
- `AK_CARGO_PREFIX="taskset -c 0,9,10,19"` pins the build.

By hand, for a slice's own build rule (CMake, a script), add the same `--config` to the
`cargo build` of ak-core:

```sh
cargo build --release --manifest-path poc/codec/crates/ak-core/Cargo.toml --features ... \
  --config 'patch.crates-io.h2.path="<h2 0.4.19 source with h2-batch.patch applied>"'
git checkout poc/codec/Cargo.lock   # cargo rewrote it for the path patch
```

Keep each variant in its own target directory, and set `CARGO_BUILD_BUILD_DIR` to that
directory: a cdylib's intermediate file carries no hash.

### Rust slice

The harness builds ak-core as a path dependency inside `poc/rust`, which is always stock. A
timed h2-batch condition loads the h2-batch core in place of the harness's own by
`LD_LIBRARY_PATH=TARGET_DIR/release`, so tonic in the Rust host (cells A, D, F) keeps
crates.io h2. For a fair stock column, build the stock core with the same script and load
it the same way. The drivers record the sha256 of the core each condition loads.

### C++ slice

Point the core build at the h2-batch source through the same `--config`, in its own target
directory, or link the library `build.sh` produced.
