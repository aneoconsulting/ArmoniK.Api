//! Link the core as a shared library.
//!
//! `ak-core` is a cdylib rather than an rlib on purpose. With the rlib in the crate graph
//! rustc inlined every `extern "C"` entry point into the host: the release binary had zero
//! call sites to `ak_encode_ListResultsResponse`, and the boundary-call counters still
//! incremented, because the counting code was inlined along with everything else. An arm
//! measured like that is a measurement of the optimiser.
//!
//! Through the dynamic linker the call cannot be inlined, and it is also what a managed
//! host actually does: `dlopen` a shared object. A C or C++ host that statically links the
//! staticlib gets a direct call instead, which is cheaper; this slice measures the shared
//! case and says so.
use std::path::PathBuf;

fn main() {
    // OUT_DIR is <target>/<profile>/build/<pkg>-<hash>/out; the cdylib lands in <profile>.
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let profile_dir = out
        .ancestors()
        .nth(3)
        .expect("unexpected OUT_DIR shape")
        .to_path_buf();
    // A dependency's cdylib is left in <profile>/deps; only a workspace MEMBER's is
    // uplifted to <profile>. Search both so this works either way -- but **deps first**,
    // because R0 moved `ak-core` out of this workspace and it is therefore no longer
    // uplifted. Whatever `<profile>/libak_core.so` still holds is a build from before the
    // move, and with the old order the harness would have linked and loaded THAT: the
    // exact shape of "a change that measures the same because it is not in the build".
    let deps = profile_dir.join("deps");
    // An explicit override for a toolchain whose target-directory layout is not the one
    // the ancestor walk above assumes: the nightly used for the ThreadSanitizer build
    // (FIX-PLAN WP4 item 9) puts a dependency's cdylib under `build/<crate>/<hash>/out`.
    // Set by `gen/tsan.sh`, which finds the one `libak_core.so` that build produced.
    println!("cargo:rerun-if-env-changed=AK_CORE_LIB_DIR");
    if let Ok(dir) = std::env::var("AK_CORE_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
    }
    for d in [&deps, &profile_dir] {
        println!("cargo:rustc-link-search=native={}", d.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", d.display());
    }
    println!("cargo:rustc-link-lib=dylib=ak_core");
    println!("cargo:rerun-if-changed=build.rs");
}
