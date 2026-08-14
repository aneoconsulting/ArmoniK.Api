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

Then, from `packages/rust`:

```sh
cargo publish -p armonik-transport --dry-run
cargo publish -p armonik-macros --dry-run
cargo publish -p armonik --dry-run
```

A dry run packages the crate from its `include` list, so it is also what catches a file the
manifest does not ship. `armonik-macros` ships `tests/**` for that reason: its only test suite
compiles a fixture proto, and without it a vendored copy of the crate cannot be tested by whoever
vendored it.
