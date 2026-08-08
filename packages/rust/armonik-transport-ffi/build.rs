//! Generates the two contract artefacts of this crate: the C header, and the C# declarations.
//!
//! Both are written into the source tree, under `include/`, and committed. They are what a reviewer
//! reads to see the whole contract at once, and what a caller's own declarations are checked
//! against; generating them into `OUT_DIR` instead would put them somewhere nobody looks. Because
//! they are committed, a build that changes the ABI also changes a tracked file - which is the
//! point.
//!
//! Neither artefact is an input to compiling this crate, so a generator that fails warns rather than
//! breaking the build: the most common cause is a toolchain that cannot expand macros in a
//! dependency, which has nothing to do with whether this crate is correct.

use std::path::{Path, PathBuf};

fn main() {
    let crate_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("cargo always sets CARGO_MANIFEST_DIR"),
    );
    let include = crate_dir.join("include");
    if let Err(error) = std::fs::create_dir_all(&include) {
        println!(
            "cargo:warning=could not create {}: {error}",
            include.display()
        );
        return;
    }

    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=build.rs");

    generate_header(&crate_dir, &include.join("armonik_transport_ffi.h"));
    generate_csharp(&crate_dir, &include.join("NativeMethods.g.cs"));
}

/// Write the C header, cleaned of everything that belongs to the sources rather than to the
/// contract.
fn generate_header(crate_dir: &Path, header: &Path) {
    let config = match cbindgen::Config::from_file(crate_dir.join("cbindgen.toml")) {
        Ok(config) => config,
        Err(error) => {
            println!("cargo:warning=could not read cbindgen.toml: {error}");
            return;
        }
    };

    let bindings = match cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
    {
        Ok(bindings) => bindings,
        Err(error) => {
            println!(
                "cargo:warning=could not generate {}: {error}",
                header.display()
            );
            return;
        }
    };

    let mut generated = Vec::new();
    bindings.write(&mut generated);
    match String::from_utf8(generated) {
        Ok(text) => write_if_changed(header, &as_contract_text(&text)),
        Err(error) => println!("cargo:warning=cbindgen produced invalid UTF-8: {error}"),
    }
}

/// Write the C# declarations.
///
/// The settings are what netstandard2.0 and a 32-bit host allow. `IntPtr`/`UIntPtr` rather than
/// `nint`/`nuint`, and delegates rather than function pointers, because neither those keywords nor
/// `UnmanagedCallersOnly` exist on .NET Framework. `CallingConvention.Cdecl` is spelled out on every
/// entry point, because the default is `StdCall` and the difference is a corrupted stack on x86,
/// where 32-bit Office still lives.
fn generate_csharp(crate_dir: &Path, bindings: &Path) {
    let result = csbindgen::Builder::default()
        .input_extern_file(crate_dir.join("src").join("lib.rs"))
        .input_extern_file(crate_dir.join("src").join("client.rs"))
        .input_extern_file(crate_dir.join("src").join("error.rs"))
        .input_extern_file(crate_dir.join("src").join("status.rs"))
        .csharp_namespace("ArmoniK.Api.Client.Native")
        .csharp_class_name("NativeMethods")
        .csharp_class_accessibility("public")
        .csharp_dll_name("armonik_transport_ffi")
        .csharp_use_function_pointer(false)
        .csharp_use_nint_types(false)
        // A type is emitted when a signature mentions it. These two are part of the contract without
        // being an argument to anything yet.
        .always_included_types(["ak_status", "ak_bytes_in"])
        // `AK_ABI_VERSION`, so a caller compares against a name rather than against a number it
        // copied. The result codes come across as an enum instead, which is the only form that
        // carries a negative value: a constant's value is read as a literal, and `-1` is not one.
        .csharp_generate_const_filter(|name| name.starts_with("AK_"))
        .generate_csharp_file(bindings);

    match result {
        Ok(()) => clean_in_place(bindings),
        Err(error) => println!(
            "cargo:warning=could not generate {}: {error}",
            bindings.display()
        ),
    }
}

/// Rewrite an artefact the generator has just written, cleaned of everything that belongs to the
/// sources rather than to the contract.
fn clean_in_place(path: &Path) {
    match std::fs::read_to_string(path) {
        Ok(text) => write_if_changed(path, &as_contract_text(&text)),
        Err(error) => println!("cargo:warning=could not read {}: {error}", path.display()),
    }
}

/// Take out of a generated artefact what belongs to the sources rather than to the contract.
///
/// Both generators copy documentation across verbatim, and documentation written for one language
/// reads as a leak in another: a link that resolves to nothing, a section heading from a convention
/// the reader does not follow, a path naming a module the reader cannot see. Whoever opens one of
/// these files has a compiler for their own language and no way to look any of that up.
fn as_contract_text(text: &str) -> String {
    let text = flatten_doc_links(text);
    let text = plain_headings(&text);
    text.replace("this crate", "this library")
        .replace("This crate", "This library")
}

/// Turn every documentation link into the plain name it points at.
///
/// `[`a::b::Thing`]` becomes `` `Thing` ``: the brackets read as broken markup, and a path resolves
/// to nothing outside the sources it came from.
fn flatten_doc_links(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find("[`") {
        let (before, from) = rest.split_at(start);
        out.push_str(before);
        let body = &from[2..];
        // An opening delimiter with no closing one is not a link: the rest is prose, and rewriting
        // it would eat text a reader needs.
        let Some(end) = body.find("`]") else {
            out.push_str(from);
            return out;
        };
        let path = &body[..end];
        out.push('`');
        out.push_str(path.rsplit_once("::").map_or(path, |(_, last)| last));
        out.push('`');
        rest = &body[end + 2..];
    }

    out.push_str(rest);
    out
}

/// The comment markers the generators put documentation behind.
const COMMENT_MARKERS: &[&str] = &["*", "///"];

/// Turn a documentation line that is a heading into an ordinary labelled line.
///
/// A `#`-marked heading is markup of the documentation format, not of the comment syntax it is
/// embedded in, so it arrives as a stray `#` in the middle of a comment block.
fn plain_headings(text: &str) -> String {
    let mut out = String::with_capacity(text.len());

    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        let content = body.trim_start();
        // The text before the `#`, kept verbatim so a heading stays aligned with the lines around
        // it, and the heading itself.
        let heading = COMMENT_MARKERS.iter().find_map(|marker| {
            let after_marker = content.strip_prefix(marker)?;
            let hashes = after_marker.trim_start();
            let heading = hashes.strip_prefix('#')?.trim_start_matches('#').trim();
            (!heading.is_empty()).then(|| (body.len() - hashes.len(), heading))
        });

        match heading {
            Some((prefix, heading)) => {
                out.push_str(&body[..prefix]);
                out.push_str(heading);
                out.push(':');
                out.push_str(&line[body.len()..]);
            }
            None => out.push_str(line),
        }
    }

    out
}

/// Write `content` only when it differs, so an ordinary rebuild does not keep touching the mtime of
/// a tracked file.
fn write_if_changed(path: &Path, content: &str) {
    if std::fs::read_to_string(path).is_ok_and(|existing| existing == content) {
        return;
    }
    if let Err(error) = std::fs::write(path, content) {
        println!("cargo:warning=could not write {}: {error}", path.display());
    }
}
