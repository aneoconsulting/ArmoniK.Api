// Point the linker at the plain C library that every mechanism arm reaches, and
// set an rpath so the built extension finds it at import time without the
// driver having to set LD_LIBRARY_PATH (which would be one more difference
// between this arm and the C-extension arm).
fn main() {
    let dir = std::env::var("AKMECH_CABI_DIR").expect("AKMECH_CABI_DIR");
    println!("cargo:rustc-link-search=native={dir}");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
    println!("cargo:rerun-if-env-changed=AKMECH_CABI_DIR");
}
