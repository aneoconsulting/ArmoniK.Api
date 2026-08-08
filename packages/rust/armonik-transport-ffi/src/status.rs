//! The result codes every entry point of this crate returns.
//!
//! `0` is success; everything else is negative. There is no positive space: this ABI carries no gRPC
//! status, and an HTTP status is a response header rather than a result code.
//!
//! An enum rather than a list of constants, because it is the one form both generators carry
//! across: the C# generator reads a constant's value as a literal, so a negative one reaches its
//! output as nothing at all, while it reads an enum's discriminants sign and all.
//!
//! Entry points still return a plain `i32` rather than this type. A C enum's underlying type is
//! implementation-defined, so a signature naming one promises a width this ABI would then have to
//! keep; `int32_t` says exactly what crosses, and the enum names the values it may take.
//!
//! The `AK_` prefix is written here rather than acquired in a generator's rename table, so that one
//! name reads the same in the header, in the C# bindings and in this source, and so that a code
//! added later cannot reach one artefact unprefixed.

/// The result of a call: [`ak_status::AK_OK`] on success, and a negative code otherwise.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ak_status {
    /// The operation succeeded.
    AK_OK = 0,
    /// A pointer argument that must not be null was null.
    AK_NULL_ARGUMENT = -1,
    /// A byte buffer was not valid UTF-8 where UTF-8 was required.
    AK_INVALID_UTF8 = -2,
    /// The client configuration was rejected; the message says what in it.
    AK_INVALID_CONFIG = -3,
    /// The request never reached the server: no connection at all, or a connection that failed
    /// before any response header arrived.
    AK_CONNECTION_FAILED = -4,
    /// The handle passed to a call has already been released, or was never valid.
    AK_INVALID_HANDLE = -5,
    /// The object is not in a state that allows the operation.
    AK_INVALID_STATE = -6,
    // -7 is reserved: no code carries that value, and none is given it. A gap costs nothing, and it
    // keeps a caller that compares against -7 matching nothing rather than matching something else.
    /// Something inside this crate failed in a way that is not the caller's doing; see the message.
    AK_INTERNAL = -8,
    /// A panic was caught at the boundary. This is always a bug in this crate; the message carries
    /// whatever the panic payload could be turned into.
    AK_INTERNAL_PANIC = -9,
    /// The request was cancelled.
    AK_CANCELLED = -10,
    /// The configured timeout elapsed while the request was still in flight.
    AK_TIMEOUT = -11,
    /// The connection failed after the response headers had arrived: a reset stream, a broken
    /// connection, a protocol error.
    AK_TRANSPORT = -12,
}

impl ak_status {
    /// This code as it crosses the ABI.
    pub(crate) const fn code(self) -> i32 {
        self as i32
    }
}

#[cfg(test)]
mod tests {
    use super::ak_status::*;
    use super::*;

    /// Every code, so that a new one cannot be added without being weighed against the others.
    const ALL: &[ak_status] = &[
        AK_OK,
        AK_NULL_ARGUMENT,
        AK_INVALID_UTF8,
        AK_INVALID_CONFIG,
        AK_CONNECTION_FAILED,
        AK_INVALID_HANDLE,
        AK_INVALID_STATE,
        AK_INTERNAL,
        AK_INTERNAL_PANIC,
        AK_CANCELLED,
        AK_TIMEOUT,
        AK_TRANSPORT,
    ];

    #[test]
    fn success_is_zero_and_every_failure_is_negative() {
        assert_eq!(AK_OK.code(), 0);
        for status in ALL.iter().filter(|status| **status != AK_OK) {
            assert!(status.code() < 0, "{status:?} is not a failure code");
        }
    }

    #[test]
    fn no_two_codes_share_a_value_and_minus_seven_is_free() {
        let mut codes: Vec<i32> = ALL.iter().map(|status| status.code()).collect();
        let total = codes.len();
        codes.sort_unstable();
        codes.dedup();

        assert_eq!(codes.len(), total, "two codes share a value");
        assert!(!codes.contains(&-7), "-7 is reserved");
    }
}
