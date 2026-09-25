//! The rpath to the shared core, as `crates/harness/build.rs` sets it for the harness's own
//! binaries (a `rustc-link-arg` does not propagate to dependents). `ak-core` is linked as a
//! cdylib through the dynamic linker, as in every other arm of this slice.
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let profile_dir = out.ancestors().nth(3).expect("unexpected OUT_DIR shape").to_path_buf();
    for d in [profile_dir.join("deps"), profile_dir.clone()] {
        println!("cargo:rustc-link-search=native={}", d.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", d.display());
    }
    println!("cargo:rerun-if-changed=build.rs");
}
