# Releasing the Rust crates

Three crates ship from `packages/rust`, and they have to go out in one order. There is no
automation: `.github/workflows/publish.yml` publishes the other languages' packages and contains no
`cargo publish` step, so this is the procedure.

## The order, and why it is forced

```sh
cd packages/rust
cargo publish -p armonik-transport   # 1
cargo publish -p armonik-macros      # 2
cargo publish -p armonik             # 3
```

Every edge is a `path` dependency, and a `path` dependency cannot be published: `cargo publish`
rewrites it into a version requirement against the registry, so the version it names has to be there
already.

- `armonik` → `armonik-transport`, `version = "3.29.2-beta-0"` in `Cargo.toml`'s
  `[workspace.dependencies]`. A caret requirement, so a later patch of the transport satisfies it.
- `armonik` → `armonik-macros`, `version = "=3.29.2-beta-0"` in `armonik/Cargo.toml`. An **exact**
  pin: the macros emit `crate::`-rooted paths into `armonik`, so the two are one artifact split
  across two crates and a mismatched pair does not compile. Both are released together, always.

crates.io's index is eventually consistent, so **wait for each crate to appear before publishing the
next one**; `cargo publish` on step 2 or 3 fails with "no matching package named ..." otherwise.

## Before publishing

`nr update-versions` (see `scripts/update-versions.ts`) keeps all four version strings in step: the
workspace `version`, the two dependency requirements above, and the rest of the repository's
manifests. `nr verify-versions` checks them. Do not edit them by hand: the `=` pin in
`armonik/Cargo.toml` is the one a manual bump forgets, and the failure lands on the *consumer*, at
compile time, with an expansion that names paths their `armonik` does not have.

Then, from `packages/rust`, check what each crate would ship:

```sh
cargo package --list -p armonik-transport
cargo package --list -p armonik-macros
cargo package --list -p armonik
```

That is the check for `include`, and it works offline for all three (from a clean tree: a dirty one
needs `--allow-dirty`). `armonik`'s list carries all 40 protos, resolved through the `protos`
symlink, and every crate's carries its `LICENSE`; `armonik-macros` ships `tests/**` because its
only test suite compiles a fixture proto, and without it a vendored copy of the crate cannot be
tested by whoever vendored it.

A **dry run** does more -- it builds the packaged crate -- and for that reason it cannot be run
ahead of time for all three: `cargo publish -p armonik --dry-run` resolves the rewritten
`=3.29.2-beta-0` requirement against the registry, so it fails with "no matching package named
`armonik-macros`" until the macros are actually published. It is the publish order, one step
earlier. So interleave:

```sh
cargo publish -p armonik-transport --dry-run
cargo publish -p armonik-transport
# wait for it to appear in the index
cargo publish -p armonik-macros --dry-run
cargo publish -p armonik-macros
# wait for it to appear in the index
cargo publish -p armonik --dry-run
cargo publish -p armonik
```
