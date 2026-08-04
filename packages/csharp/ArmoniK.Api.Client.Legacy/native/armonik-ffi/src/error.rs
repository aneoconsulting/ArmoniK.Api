//! Error codes crossing the C ABI: `0` on success, a gRPC status code above zero, or one of these
//! FFI-internal codes below zero. Every code above zero is a `tonic::Code` discriminant, so the two
//! ranges never collide.

use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// A required pointer was null.
pub const AK_ERR_NULL_POINTER: i32 = -1;
/// Bytes that were supposed to be UTF-8 were not.
pub const AK_ERR_INVALID_UTF8: i32 = -2;
/// The configuration was rejected before a connection was attempted.
pub const AK_ERR_INVALID_CONFIG: i32 = -3;
/// Establishing the connection itself failed.
pub const AK_ERR_CONNECTION_FAILED: i32 = -4;
/// Rust code on the other side of this call panicked; the panic did not cross the ABI boundary.
pub const AK_ERR_PANIC: i32 = -5;

/// Runs `body`, converting an unwinding panic into [`AK_ERR_PANIC`] rather than letting it cross
/// the ABI boundary, which is undefined behaviour. Every `#[no_mangle] extern "C"` entry point in
/// this crate is wrapped in this at its outermost layer.
///
/// `AssertUnwindSafe` because `body` typically blocks on a future involving tokio/tonic internals
/// that are not `UnwindSafe` by the type system's own conservative rule (interior mutability that
/// *could* be observed half-updated after a panic) - but nothing here reuses that state across a
/// caught panic: every entry point starts from its own arguments and returns, it never resumes a
/// partially-mutated value.
pub fn guard<F>(body: F) -> i32
where
    F: FnOnce() -> i32,
{
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(code) => code,
        Err(payload) => {
            report_panic(&payload);
            AK_ERR_PANIC
        }
    }
}

/// `error`, followed by every cause behind it, each joined onto the same line with `": "`.
///
/// `armonik-transport`'s own `Display` names only the step that failed (e.g. "Could not connect to
/// the remote"), not why: the reason - a TLS failure, a refused connection, a DNS lookup - lives one
/// or more levels down its `source()` chain. Reporting only the top level would hand a caller a
/// message that never says what actually went wrong.
pub fn describe(error: &(dyn std::error::Error + 'static)) -> String {
    let mut message = String::from(strip_location(&error.to_string()));
    let mut cause = error.source();
    while let Some(source) = cause {
        message.push_str(": ");
        message.push_str(strip_location(&source.to_string()));
        cause = source.source();
    }
    message
}

/// Drops a trailing `[some/file.rs:12:34]` from `message`, if the whole string ends with one.
///
/// Every level of `armonik-transport`'s own errors embeds its own source location this way, for
/// debugging that crate; a caller on the other side of this ABI cannot open the file it names, so
/// it is noise here rather than the diagnostic it is meant to be. Only strips a suffix shaped
/// exactly like a location - `word/word.rs:digits:digits` - so a deeper cause that happens to end
/// in brackets for an unrelated reason (an IPv6 address, say) is left alone.
fn strip_location(message: &str) -> &str {
    let Some(tag_start) = message.rfind(" [") else {
        return message;
    };
    let Some(tag) = message[tag_start + 2..].strip_suffix(']') else {
        return message;
    };
    if looks_like_a_source_location(tag) {
        message[..tag_start].trim_end()
    } else {
        message
    }
}

fn looks_like_a_source_location(tag: &str) -> bool {
    let mut parts = tag.rsplitn(3, ':');
    let (Some(column), Some(line), Some(file)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    file.contains(".rs")
        && !line.is_empty()
        && line.bytes().all(|b| b.is_ascii_digit())
        && !column.is_empty()
        && column.bytes().all(|b| b.is_ascii_digit())
}

/// Best-effort extraction of a panic's own message, for whoever reads this crate's logs; the panic
/// itself carries no location information once caught here.
fn report_panic(payload: &(dyn Any + Send)) {
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("Box<dyn Any> (no string payload)");
    eprintln!("armonik-ffi: caught a panic at the ABI boundary: {message}");
}

#[cfg(test)]
mod tests {
    use super::strip_location;

    #[test]
    fn a_trailing_source_location_is_dropped() {
        let message = "Could not connect to the remote https://x [src/connect.rs:64:9]";
        assert_eq!(
            strip_location(message),
            "Could not connect to the remote https://x"
        );
    }

    #[test]
    fn a_message_with_no_location_is_untouched() {
        let message = "connection refused (os error 111)";
        assert_eq!(strip_location(message), message);
    }

    #[test]
    fn an_ipv6_address_is_not_mistaken_for_a_location() {
        // Genuinely ends in `]`, same as a real location tag would, but the bracket holds an
        // address - three colon-separated segments, none of them a `.rs` path - not a source
        // location, so `looks_like_a_source_location` has to be the thing that tells them apart.
        let message = "could not reach [::1]";
        assert_eq!(strip_location(message), message);
    }
}
