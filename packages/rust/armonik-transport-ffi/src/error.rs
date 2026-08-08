//! Owned byte buffers handed across the ABI, and the errors this crate reports through them.

use std::borrow::Cow;
use std::fmt;

use bytes::Bytes;

use crate::status::ak_status;

/// An owned buffer handed to the caller.
///
/// `ptr`/`len` are a read-only *view*; the allocation itself belongs to `owner`, an opaque handle
/// the caller passes back unchanged and never otherwise touches. Keeping the two apart - rather than
/// treating `ptr` as the thing to release - is what lets a buffer this crate already holds cross
/// without being copied: `owner` names whatever really owns those bytes, which may be a
/// reference-counted view into a larger allocation, so `ptr` on its own is not something that can be
/// released.
///
/// Every buffer with a non-null `owner` must be given up by exactly one call to
/// [`ak_bytes_release`]. The zeroed value (`ptr` and `owner` null, `len` 0) means "no data" and is
/// always safe to pass there.
///
/// Input buffers travel as [`ak_bytes_in`] instead: those are borrowed from the caller and are never
/// released by this crate. Only a value this crate produced is ever released here.
///
/// Copying the `(ptr, len, owner)` triple is harmless, which is why this is a plain value. What must
/// never happen, on either side, is passing more than one copy of the same original value to
/// [`ak_bytes_release`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ak_bytes {
    /// Pointer to the first byte, or null for an empty or absent buffer. Readable until this value
    /// is passed to [`ak_bytes_release`]; never write through it.
    pub ptr: *const u8,
    /// Number of bytes at `ptr`.
    pub len: usize,
    /// Opaque; pass back to [`ak_bytes_release`] unchanged, never dereference or otherwise inspect
    /// it.
    pub owner: *mut std::ffi::c_void,
}

impl ak_bytes {
    /// The zeroed value, meaning "no data".
    ///
    /// Public because it is what a caller initialises an `out_err` slot to before a call that may or
    /// may not write one.
    pub const EMPTY: Self = Self {
        ptr: std::ptr::null(),
        len: 0,
        owner: std::ptr::null_mut(),
    };

    /// Take ownership of `data` into an [`ak_bytes`] the caller must eventually release, without
    /// copying its content.
    // `Bytes::from` on a `Vec<u8>` or on a `String`'s bytes reuses the existing allocation, so
    // building `data` and handing it here moves ownership across the ABI in one step rather than
    // copying and then leaking.
    pub(crate) fn from_bytes(data: impl Into<Bytes>) -> Self {
        let data = data.into();
        if data.is_empty() {
            return Self::EMPTY;
        }
        let ptr = data.as_ptr();
        let len = data.len();
        let owner = Box::into_raw(Box::new(data)).cast::<std::ffi::c_void>();
        Self { ptr, len, owner }
    }
}

/// Give up an [`ak_bytes`] previously returned by this crate.
///
/// # Safety
///
/// `bytes` must be a value this crate returned, not yet released. Passing a borrowed input buffer, a
/// value already released, or a value with an `owner` this crate did not produce, is undefined
/// behaviour. The zeroed value is always safe to pass here.
#[no_mangle]
pub unsafe extern "C" fn ak_bytes_release(bytes: ak_bytes) {
    crate::guard::catch_unwind_void(|| {
        if bytes.owner.is_null() {
            return;
        }
        // SAFETY: per this function's contract, `bytes.owner` was produced by
        // `ak_bytes::from_bytes`, which always leaks exactly a `Box<Bytes>`. Dropping it runs
        // `Bytes`'s own destructor - a refcount decrement, freeing the backing allocation only once
        // the last reference goes - rather than assuming `ptr`/`len` describe an allocation to
        // deallocate directly.
        drop(unsafe { Box::from_raw(bytes.owner.cast::<Bytes>()) });
    });
}

/// A borrowed input buffer: a view into memory the *caller* owns.
///
/// Also what an event payload travels as, in the other direction: borrowed for the duration of the
/// invocation and invalid the moment it returns. Never released by whoever received it. A null `ptr`
/// or a zero `len` means "empty" or "absent".
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ak_bytes_in {
    /// Pointer to the first byte, or null for an empty or absent buffer.
    pub ptr: *const u8,
    /// Number of bytes at `ptr`.
    pub len: usize,
}

/// Errors reported by this crate's own logic.
///
/// Kept separate from [`armonik_transport::ConfigError`] rather than trying to reuse its variants:
/// `ConfigError` is `#[non_exhaustive]` and this crate only ever constructs errors, so there is
/// nothing to gain by fighting that boundary.
#[derive(Debug)]
pub(crate) enum FfiError {
    NullArgument(&'static str),
    InvalidUtf8,
    /// The configuration blob was not JSON, or named an option the transport refused.
    InvalidJson(String),
    Config(armonik_transport::ConfigError),
    Connection(armonik_transport::ConnectionError),
    InvalidHandle,
    InvalidState(&'static str),
}

/// Render `error` and everything that caused it as the one message that crosses the ABI.
///
/// Two things happen here, both because the caller gets a single string and nothing else.
///
/// The chain is flattened. `armonik-transport` reports "Could not establish TLS connection to the
/// remote ..." and leaves *why* - a key that does not match its certificate, a CA file that could
/// not be read - in the source beneath it. Nothing survives a C ABI but the bytes handed across it,
/// and there is no error chain left to walk on the other side, so a message that stopped at the
/// outermost error would drop the only part that says what to fix.
///
/// And the locations those errors carry are handled the way this build says to: removed by default,
/// kept under the `error-locations` feature.
pub(crate) fn describe(error: &dyn std::error::Error) -> String {
    let mut message = String::new();
    let mut current = Some(error);

    while let Some(error) = current {
        let rendered = error.to_string();
        let text = strip_location(&rendered);
        if !text.is_empty() {
            if !message.is_empty() {
                message.push_str(": ");
            }
            message.push_str(&text);
        }
        current = error.source();
    }

    message
}

/// Render a message the way this build reports errors.
///
/// Locations are removed unless the `error-locations` feature says to keep them. Both kinds: the
/// ` [file.rs:12:34]` of a Rust source, and the ` at line 1 column 56` that names a position in the
/// configuration document. Neither means anything to whoever reads a host application's log - that
/// document is generated by the options layer, so a position in it is as internal as a source path -
/// and both are the first thing wanted when the reader is the person who wrote this crate.
///
/// The ABI is identical either way, so the two builds are interchangeable: a consumer picks one by
/// which library it loads, not by how it calls.
fn strip_location(text: &str) -> Cow<'_, str> {
    if cfg!(feature = "error-locations") {
        Cow::Borrowed(text)
    } else {
        remove_locations(text)
    }
}

/// Drop every location the message carries: the bracketed source ones, and the `serde` position.
///
/// The bracketed form is looked for anywhere, not only at the end: `serde_json` appends its own
/// ` at line X column Y` after it, so a rule that only looked at the suffix lets a source path
/// through to the caller's log.
///
/// Deliberately narrow all the same: only a bracketed run whose last two `:`-separated parts are
/// numbers counts as a location, so a message with brackets of its own keeps them.
fn remove_locations(text: &str) -> Cow<'_, str> {
    let text = remove_position(text);
    let mut rest = text;
    let mut kept = String::new();
    // Whether anything was removed, which is not the same as `kept` being non-empty: a message that
    // is nothing but a location leaves an empty string behind, and that is the answer, not a reason
    // to hand back the original.
    let mut stripped = false;

    while let Some(open) = rest.find(" [") {
        let after = &rest[open + 2..];
        let Some(close) = after.find(']') else {
            // No closing bracket: nothing further can be a location.
            break;
        };
        if is_location(&after[..close]) {
            kept.push_str(rest[..open].trim_end());
            rest = &after[close + 1..];
            stripped = true;
        } else {
            // Not a location: keep it, and go on looking after it.
            kept.push_str(&rest[..open + 2]);
            rest = after;
        }
    }

    if !stripped {
        return Cow::Borrowed(text);
    }
    kept.push_str(rest);
    Cow::Owned(kept)
}

/// Drop a trailing ` at line <digits> column <digits>`, which is where `serde_json` says a value sat
/// in the document it was reading.
///
/// A suffix only, because that is the one place `serde_json` puts it, and looking anywhere would
/// start eating ordinary prose.
fn remove_position(text: &str) -> &str {
    let Some(at) = text.rfind(" at line ") else {
        return text;
    };
    let Some((line, column)) = text[at + " at line ".len()..].split_once(" column ") else {
        return text;
    };
    if is_number(line) && is_number(column) {
        text[..at].trim_end()
    } else {
        text
    }
}

/// Whether `candidate` reads as `path:line:column`.
fn is_location(candidate: &str) -> bool {
    let mut parts = candidate.rsplitn(3, ':');
    let column = parts.next().unwrap_or_default();
    let line = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();

    !path.is_empty() && is_number(line) && is_number(column)
}

/// Whether `value` is a non-empty run of ASCII digits.
fn is_number(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

impl fmt::Display for FfiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullArgument(name) => write!(f, "`{name}` must not be null"),
            Self::InvalidUtf8 => write!(f, "a buffer that was expected to be UTF-8 was not"),
            // The build's policy applies here too: a `serde_json` error renders the error it wraps,
            // position and all, and never reaches `describe` because it has no source chain to walk.
            Self::InvalidJson(source) => {
                write!(f, "invalid configuration: {}", strip_location(source))
            }
            Self::Config(source) => write!(f, "{}", describe(source)),
            Self::Connection(source) => write!(f, "{}", describe(source)),
            Self::InvalidHandle => write!(f, "the handle is invalid or has already been released"),
            Self::InvalidState(reason) => write!(f, "{reason}"),
        }
    }
}

impl FfiError {
    /// The negative result code this error is reported as.
    pub(crate) fn status(&self) -> i32 {
        match self {
            Self::NullArgument(_) => ak_status::AK_NULL_ARGUMENT.code(),
            Self::InvalidUtf8 => ak_status::AK_INVALID_UTF8.code(),
            Self::InvalidJson(_) | Self::Config(_) => ak_status::AK_INVALID_CONFIG.code(),
            Self::Connection(_) => ak_status::AK_CONNECTION_FAILED.code(),
            Self::InvalidHandle => ak_status::AK_INVALID_HANDLE.code(),
            Self::InvalidState(_) => ak_status::AK_INVALID_STATE.code(),
        }
    }

    /// Render this error into the `(status, ak_bytes)` pair every fallible entry point returns.
    pub(crate) fn into_ffi_result(self, out_err: *mut ak_bytes) -> i32 {
        let status = self.status();
        if !out_err.is_null() {
            // SAFETY: `out_err` is documented as writable by every function that takes it.
            unsafe { *out_err = ak_bytes::from_bytes(self.to_string()) };
        }
        status
    }
}

impl From<armonik_transport::ConfigError> for FfiError {
    fn from(source: armonik_transport::ConfigError) -> Self {
        Self::Config(source)
    }
}

impl From<armonik_transport::ConnectionError> for FfiError {
    fn from(source: armonik_transport::ConnectionError) -> Self {
        Self::Connection(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Read back what the caller would see, then release it exactly once.
    ///
    /// # Safety
    ///
    /// `bytes` must be a value `from_bytes` produced and that has not been released.
    unsafe fn read_and_release(bytes: ak_bytes) -> Vec<u8> {
        let seen = if bytes.len == 0 {
            Vec::new()
        } else {
            // SAFETY: `ptr`/`len` are documented as readable until `ak_bytes_release`.
            unsafe { std::slice::from_raw_parts(bytes.ptr, bytes.len) }.to_vec()
        };
        // SAFETY: forwarded from this function's contract.
        unsafe { ak_bytes_release(bytes) };
        seen
    }

    #[test]
    fn an_owned_buffer_is_readable_through_the_view_it_hands_out() {
        let bytes = ak_bytes::from_bytes(b"hello".to_vec());

        assert!(!bytes.owner.is_null(), "a non-empty buffer has an owner");
        assert_eq!(bytes.len, 5);
        // SAFETY: just produced above, released exactly once here.
        assert_eq!(unsafe { read_and_release(bytes) }, b"hello");
    }

    #[test]
    fn a_sub_slice_is_released_through_its_owner_rather_than_its_pointer() {
        // The case the split between `ptr` and `owner` exists for. A `Bytes` may be a refcounted
        // view into the middle of a larger allocation, so `ptr` is not something that can be
        // deallocated: an implementation that treated the view as the allocation would corrupt the
        // heap here, which is exactly what this test asks Miri to check.
        let whole = Bytes::from_static(b"0123456789");
        let middle = whole.slice(3..7);

        let bytes = ak_bytes::from_bytes(middle);
        assert_eq!(bytes.len, 4);
        // SAFETY: produced just above, released exactly once.
        assert_eq!(unsafe { read_and_release(bytes) }, b"3456");
    }

    #[test]
    fn an_empty_buffer_needs_no_owner_and_is_released_as_a_no_op() {
        let bytes = ak_bytes::from_bytes(Vec::new());

        assert!(bytes.ptr.is_null());
        assert_eq!(bytes.len, 0);
        assert!(
            bytes.owner.is_null(),
            "nothing was allocated, so there is nothing for the caller to release"
        );
        // SAFETY: the zeroed value is documented as always safe to pass here, more than once.
        unsafe {
            ak_bytes_release(bytes);
            ak_bytes_release(bytes);
        }
    }

    #[test]
    fn an_error_renders_its_message_and_status_together() {
        let mut out = ak_bytes::EMPTY;
        let status = FfiError::NullArgument("out").into_ffi_result(std::ptr::addr_of_mut!(out));

        assert_eq!(status, ak_status::AK_NULL_ARGUMENT.code());
        // SAFETY: written by `into_ffi_result` just above, released exactly once.
        assert_eq!(unsafe { read_and_release(out) }, b"`out` must not be null");
    }

    #[test]
    fn a_message_keeps_what_caused_it() {
        #[derive(Debug)]
        struct Cause;
        impl fmt::Display for Cause {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "the key does not match the certificate")
            }
        }
        impl std::error::Error for Cause {}

        #[derive(Debug)]
        struct Outer;
        impl fmt::Display for Outer {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "could not establish TLS [src/connect.rs:135:14]")
            }
        }
        impl std::error::Error for Outer {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&Cause)
            }
        }

        // What this is about is the *chain*: the outer message alone says nothing actionable. The
        // location is the other axis, and follows the build's feature, so it is asserted separately.
        let described = describe(&Outer);
        assert!(
            described.contains("could not establish TLS"),
            "the outer message is missing: {described}"
        );
        assert!(
            described.contains("the key does not match the certificate"),
            "the cause is what says what to fix, and it has to survive: {described}"
        );
        if cfg!(feature = "error-locations") {
            assert!(described.contains("src/connect.rs:135:14"), "{described}");
        } else {
            assert_eq!(
                described,
                "could not establish TLS: the key does not match the certificate"
            );
        }
    }

    #[test]
    fn only_a_real_source_location_is_removed() {
        assert_eq!(remove_locations("failed [src/a.rs:1:2]"), "failed");
        assert_eq!(
            remove_locations("failed [C:\\work\\src\\a.rs:12:34]"),
            "failed"
        );
        // Not a location, so not the boundary's business to remove: brackets are ordinary text.
        assert_eq!(remove_locations("option [Endpoint]"), "option [Endpoint]");
        assert_eq!(
            remove_locations("failed [src/a.rs:no:no]"),
            "failed [src/a.rs:no:no]"
        );
        assert_eq!(remove_locations("nothing bracketed"), "nothing bracketed");
    }

    #[test]
    fn a_location_anywhere_in_the_message_is_removed_not_only_at_the_end() {
        // What `serde_path_to_error` produces: the wrapped error's location, then its own suffix. A
        // rule that only looked at the end of the message lets this through to a customer's log.
        assert_eq!(
            remove_locations(
                "Could not read file `nope.pem` [armonik-transport/src/tls_config.rs:336:61] at line 1 column 56"
            ),
            "Could not read file `nope.pem`"
        );

        // Several, mixed with brackets that are not locations.
        assert_eq!(
            remove_locations("a [src/a.rs:1:2] b [Endpoint] c [src/b.rs:3:4] d"),
            "a b [Endpoint] c d"
        );

        // A message that is nothing but a location leaves nothing behind. Answering with the
        // original here would be the easy mistake: "I built no replacement" is not the same as "I
        // removed nothing".
        assert_eq!(remove_locations(" [foo.rs:1:2]"), "");
        assert_eq!(remove_locations(" [a.rs:1:2] [b.rs:3:4] tail"), " tail");

        // An unterminated bracket is text, and must not send the scan round again forever.
        assert_eq!(
            remove_locations("failed [src/a.rs:1:2] and then [unclosed"),
            "failed and then [unclosed"
        );
        assert_eq!(remove_locations(""), "");
    }

    #[test]
    fn a_serde_position_goes_the_same_way_as_a_source_location() {
        // `serde_json` names where in the document a value sat. That document is generated by the
        // options layer, so the position is as internal as a source path, and goes with it.
        assert_eq!(
            remove_locations("expected value at line 1 column 1"),
            "expected value"
        );
        assert_eq!(
            remove_locations("unknown field `Nope` at line 12 column 345"),
            "unknown field `Nope`"
        );
        // Only a trailing one, and only with numbers: ordinary prose keeps its words.
        assert_eq!(
            remove_locations("failed at line one column two"),
            "failed at line one column two"
        );
        assert_eq!(
            remove_locations("at line 1 column 2 is where it broke"),
            "at line 1 column 2 is where it broke"
        );
    }

    #[test]
    fn what_a_build_does_with_a_location_follows_its_feature() {
        // The policy, as opposed to the removal itself. Asserted against the build that is running,
        // so the test says the same thing whichever way the crate was compiled.
        let rendered = strip_location("failed [src/a.rs:1:2]");
        if cfg!(feature = "error-locations") {
            assert_eq!(rendered, "failed [src/a.rs:1:2]");
        } else {
            assert_eq!(rendered, "failed");
        }
    }

    #[test]
    fn a_serde_position_follows_the_same_feature() {
        let rendered = strip_location("unknown field `Nope` at line 12 column 345");
        if cfg!(feature = "error-locations") {
            assert_eq!(rendered, "unknown field `Nope` at line 12 column 345");
        } else {
            assert_eq!(rendered, "unknown field `Nope`");
        }
    }

    #[test]
    fn a_panic_payload_is_rendered_in_both_builds() {
        // The feature is about locations. A payload is prose this crate wrote, names no file and no
        // type, and is the whole diagnosis, so it is not covered by the rule either way.
        let message = crate::guard::panic_message(&"a handle was registered twice");
        assert!(
            message.contains("a handle was registered twice"),
            "{message}"
        );
    }

    #[test]
    fn an_error_reported_without_a_message_buffer_still_returns_its_status() {
        // `out_err` is optional on every entry point that takes it, so a caller that does not want
        // the message must not be a special case for the error path.
        let status = FfiError::InvalidHandle.into_ffi_result(std::ptr::null_mut());
        assert_eq!(status, ak_status::AK_INVALID_HANDLE.code());
    }
}
