//! Checks on the generated C# declarations, which are a committed artefact rather than a build
//! output.
//!
//! `build.rs` regenerates `include/NativeMethods.g.cs` on every build, so these run against whatever
//! the current sources produce. What they pin is not the wording but the shape the consuming project
//! needs: it targets netstandard2.0 and may be loaded into a 32-bit host, which rules out several of
//! the generator's more modern defaults.

const BINDINGS: &str = include_str!("../include/NativeMethods.g.cs");

#[test]
fn every_entry_point_is_declared_and_called_the_way_the_library_expects() {
    // `CallingConvention.Cdecl` is not the platform default, and on x86 the difference is a stack
    // the callee cleans up twice. 32-bit hosts are still in use, so this is spelled out per entry
    // point rather than assumed.
    for symbol in [
        "ak_abi_version",
        "ak_bytes_release",
        "ak_client_create",
        "ak_client_release",
    ] {
        let declaration = format!(
            "[DllImport(__DllName, EntryPoint = \"{symbol}\", \
             CallingConvention = CallingConvention.Cdecl"
        );
        assert!(
            BINDINGS.contains(&declaration),
            "`{symbol}` is missing, or is not declared as Cdecl"
        );
    }

    let imports = BINDINGS.matches("[DllImport(").count();
    let cdecl = BINDINGS
        .matches("CallingConvention = CallingConvention.Cdecl")
        .count();
    assert_eq!(imports, cdecl, "an entry point is not declared as Cdecl");
}

#[test]
fn every_result_code_is_declared_with_its_value() {
    for (name, value) in [
        ("AK_OK", 0),
        ("AK_NULL_ARGUMENT", -1),
        ("AK_INVALID_UTF8", -2),
        ("AK_INVALID_CONFIG", -3),
        ("AK_CONNECTION_FAILED", -4),
        ("AK_INVALID_HANDLE", -5),
        ("AK_INVALID_STATE", -6),
        ("AK_INTERNAL", -8),
        ("AK_INTERNAL_PANIC", -9),
        ("AK_CANCELLED", -10),
        ("AK_TIMEOUT", -11),
        ("AK_TRANSPORT", -12),
    ] {
        assert!(
            BINDINGS.contains(&format!("{name} = {value},")),
            "`{name} = {value}` is missing from the generated bindings"
        );
    }

    assert!(
        BINDINGS.contains("public const int AK_ABI_VERSION = "),
        "the ABI revision is missing from the generated bindings"
    );
    assert!(!BINDINGS.contains("= -7,"), "-7 is reserved");
}

#[test]
fn every_type_of_the_contract_is_declared() {
    for declaration in [
        "public unsafe partial struct ak_bytes",
        "public unsafe partial struct ak_bytes_in",
        "public enum ak_status : int",
        // Opaque on this side too: the struct carries no field, and exists only so that a handle
        // has a type to be a pointer to.
        "public unsafe partial struct ak_client",
    ] {
        assert!(
            BINDINGS.contains(declaration),
            "`{declaration}` is missing from the generated bindings"
        );
    }
}

#[test]
fn nothing_here_needs_a_language_version_the_consumer_does_not_have() {
    // netstandard2.0, and a host that may be .NET Framework. `nint`/`nuint` and function pointers
    // are both later than that, and `UnmanagedCallersOnly` does not exist there at all, so a
    // callback has to be a delegate.
    for absent in ["nint", "nuint", "delegate*", "UnmanagedCallersOnly"] {
        assert!(
            !BINDINGS.contains(absent),
            "the bindings use {absent:?}, which netstandard2.0 does not have"
        );
    }
    assert!(
        BINDINGS.contains("System.UIntPtr"),
        "a pointer-sized integer should be `UIntPtr`"
    );
    // The class is extended by hand-written code beside it, so it has to be partial, and the
    // structs carry raw pointers.
    assert!(BINDINGS.contains("public static unsafe partial class NativeMethods"));
}

#[test]
fn every_delegate_is_declared_with_the_calling_convention() {
    // A callback crossing this boundary is a delegate, and one marshalled with the platform default
    // rather than Cdecl corrupts the stack on x86. There is nothing to check while the ABI takes no
    // callback; when it does, this is what stops the attribute from being forgotten.
    let delegates = BINDINGS.matches("delegate ").count();
    let marked = BINDINGS
        .matches("[UnmanagedFunctionPointer(CallingConvention.Cdecl)]")
        .count();
    assert_eq!(
        delegates, marked,
        "a delegate is not marked as Cdecl: {delegates} declared, {marked} marked"
    );
}

#[test]
fn nothing_in_the_bindings_says_what_it_was_generated_from() {
    // The same cleaning pass as the header runs over this file: documentation written for one
    // language reads as a leak in another, and a C# consumer can look none of it up.
    // The library's own file name is not a leak, so the module path is what is looked for rather
    // than the bare name.
    for marker in ["[`", "`]", "# Safety", "this crate", "armonik_transport::"] {
        assert!(
            !BINDINGS.contains(marker),
            "the bindings still carry {marker:?}, which the cleaning pass should have taken out"
        );
    }
    assert!(
        BINDINGS.contains("Safety:"),
        "the safety requirements lost their label"
    );
}
