//! The retry policy's status codes, against the gRPC vocabulary this crate speaks.
//!
//! `armonik-transport` names the retryable failures as the numbers gRPC puts on the wire, so that
//! declaring them costs it no gRPC dependency. Here is where those numbers meet `tonic::Code`, and
//! a mismatch would silently stop replaying the failures ArmoniK's other clients replay.

use armonik::transport::RetryConfig;

#[test]
fn the_default_retryable_codes_are_the_ones_tonic_spells() {
    let policy = RetryConfig::default();

    for code in [
        tonic::Code::Unavailable,
        tonic::Code::Aborted,
        tonic::Code::Unknown,
    ] {
        assert!(policy.is_retryable(code as i32), "{code:?}");
    }
    assert!(!policy.is_retryable(tonic::Code::InvalidArgument as i32));
}

#[test]
fn a_status_a_call_reports_is_matched_by_its_code() {
    // The shape a caller applying the policy uses: a `Status` off a failed call, and the code it
    // carries handed to the policy.
    let status = tonic::Status::unavailable("the server is restarting");

    assert!(RetryConfig::default().is_retryable(status.code() as i32));
}
