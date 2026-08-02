//! When a failed request is worth sending again.
//!
//! This module holds the policy and the loop, and nothing that knows what a call is. Deciding
//! *whether* a given request may be sent twice belongs to whoever made it: only that layer knows the
//! shape of the method, whether anything has already been handed to the caller, and whether the
//! request can still be reproduced. See [`retry`] for how the two meet.
//!
//! The waits follow the gRPC retry specification, which grpc-dotnet applies in `RetryCall`:
//! `random(0, min(initial * multiplier^n, max))`. The draw is uniform *below* the computed delay, not
//! added to it, which is why no backoff crate is used: the ones that offer jitter add it, and would
//! wait three times longer on average.

use std::time::Duration;

/// Attempts in all, first try included, matching `GrpcChannelFactory`.
const DEFAULT_MAX_ATTEMPTS: u32 = 5;
/// Wait before the second attempt.
const DEFAULT_INITIAL_BACKOFF: Duration = Duration::from_secs(1);
/// Ceiling the wait grows to.
const DEFAULT_MAX_BACKOFF: Duration = Duration::from_secs(5);
/// What each wait is multiplied by.
const DEFAULT_BACKOFF_MULTIPLIER: f32 = 1.5;
/// How much of a streamed request may be held to be able to send it again, as grpc-dotnet holds 1 MB.
const DEFAULT_MAX_BUFFER_PER_CALL: usize = 1024 * 1024;
/// The largest single request still worth sending again, the same 1 MB grpc-dotnet applies to one.
const DEFAULT_MAX_UNARY_SIZE: usize = 1024 * 1024;

/// Replaying failed requests, on the same terms as ArmoniK's other clients.
///
/// The defaults are the ones `GrpcChannelFactory` hands grpc-dotnet, so a deployment behaves the same
/// whichever client talks to it. `max_attempts` of 1 never replays.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RetryPolicy {
    /// Attempts in all, first try included.
    pub max_attempts: u32,
    /// Wait before the second attempt.
    pub initial_backoff: Duration,
    /// Ceiling the wait grows to.
    pub max_backoff: Duration,
    /// What each wait is multiplied by.
    pub backoff_multiplier: f32,
    /// Failures worth sending the request again for.
    pub retryable_status_codes: Vec<tonic::Code>,
    /// Bytes of a streamed request held so it can be sent again, the messages adding up.
    pub max_buffer_per_call: usize,
    /// Largest single request still worth sending again.
    pub max_unary_size: usize,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            initial_backoff: DEFAULT_INITIAL_BACKOFF,
            max_backoff: DEFAULT_MAX_BACKOFF,
            backoff_multiplier: DEFAULT_BACKOFF_MULTIPLIER,
            retryable_status_codes: vec![
                tonic::Code::Unavailable,
                tonic::Code::Aborted,
                tonic::Code::Unknown,
            ],
            max_buffer_per_call: DEFAULT_MAX_BUFFER_PER_CALL,
            max_unary_size: DEFAULT_MAX_UNARY_SIZE,
        }
    }
}

impl RetryPolicy {
    /// Whether a request that failed with `code` is worth sending again.
    pub fn is_retryable(&self, code: tonic::Code) -> bool {
        self.retryable_status_codes.contains(&code)
    }

    /// The delay each replay is drawn from, before the draw.
    ///
    /// One item per replay, so at most `max_attempts - 1` of them: `initial * multiplier^n`, never
    /// above `max_backoff`. Public so the arithmetic can be checked without a random number in the
    /// way, and so a caller can report what it is about to do.
    pub fn bounds(&self) -> impl Iterator<Item = Duration> + use<> {
        let ceiling = self.max_backoff;
        let multiplier = f64::from(self.backoff_multiplier);
        let mut delay = self.initial_backoff.min(ceiling);

        (0..self.max_attempts.saturating_sub(1)).map(move |_| {
            let bound = delay;
            delay = delay.mul_f64(multiplier).min(ceiling);
            bound
        })
    }

    /// How long to wait before each further attempt, and when to stop.
    ///
    /// Each wait is drawn uniformly below its bound, so that clients which failed together do not
    /// come back together.
    pub fn delays(&self) -> impl Iterator<Item = Duration> + use<> {
        self.bounds().map(draw_below)
    }
}

/// A wait drawn uniformly in `[0, bound]`, to the millisecond.
///
/// Milliseconds because that is the resolution the specification and every other client work in, and
/// a finer draw would claim a precision the network does not have.
fn draw_below(bound: Duration) -> Duration {
    let millis = bound.as_millis().min(u128::from(u64::MAX)) as u64;
    Duration::from_millis(fastrand::u64(0..=millis))
}

/// The gRPC status a failure carries, if it carries one.
///
/// What decides a replay is the status code, and only the caller's error type knows where it keeps
/// one. A failure with no status is never replayed: it did not come back from a server.
pub trait GrpcStatus {
    /// The code the server answered with.
    fn grpc_code(&self) -> Option<tonic::Code>;
}

impl GrpcStatus for tonic::Status {
    fn grpc_code(&self) -> Option<tonic::Code> {
        Some(self.code())
    }
}

/// Send a request again while the policy says it is worth it.
///
/// A macro rather than a function because the loop belongs in the caller's own body: an attempt has
/// to borrow the client it is made on, and a closure handed to a function cannot return a future that
/// borrows what the closure captured. Expanded here, the borrow ends with each turn of the loop.
///
/// Three holes to fill:
///
/// - `policy`, an `Option<RetryPolicy>` or a `&Option<RetryPolicy>`. [`None`] runs the attempt once.
/// - `code`, how to read a [`tonic::Code`] out of the attempt's error. [`GrpcStatus`] is the usual
///   answer; a caller whose error may carry no status answers [`None`] and is not replayed.
/// - `attempt`, evaluated afresh each turn. It must produce a *new* request: sending consumes one.
///
/// The wait is a plain `.await`, so dropping the future that contains this abandons the wait at once.
/// A caller that has a deadline should wrap the whole expansion rather than check between attempts.
///
/// ```
/// use armonik_transport::{GrpcStatus, RetryPolicy};
/// use armonik_transport::reexports::tonic;
///
/// # let runtime = armonik_transport::reexports::tokio::runtime::Builder::new_current_thread()
/// #     .enable_time()
/// #     .build()
/// #     .unwrap();
/// # runtime.block_on(async {
/// // Waits of zero, so the example does not sleep. Left alone the policy waits about a second.
/// let mut policy = RetryPolicy::default();
/// policy.initial_backoff = std::time::Duration::ZERO;
/// policy.max_backoff = std::time::Duration::ZERO;
///
/// let mut attempts = 0;
///
/// let outcome: Result<u32, tonic::Status> = armonik_transport::retry! {
///     policy = Some(policy),
///     code = GrpcStatus::grpc_code,
///     // Evaluated afresh each turn: sending consumes a request, so make a new one.
///     attempt = {
///         attempts += 1;
///         if attempts < 3 {
///             Err(tonic::Status::unavailable("not yet"))
///         } else {
///             Ok(attempts)
///         }
///     }
/// };
///
/// assert_eq!(outcome.unwrap(), 3);
/// # });
/// ```
#[macro_export]
macro_rules! retry {
    (policy = $policy:expr, code = $code:expr, attempt = $attempt:block) => {{
        let policy = $policy;
        let mut delays = policy
            .as_ref()
            .map($crate::RetryPolicy::delays)
            .into_iter()
            .flatten();
        let read_code = $code;

        loop {
            match $attempt {
                Ok(value) => break Ok(value),
                Err(error) => {
                    let worth_it = policy
                        .as_ref()
                        .zip(read_code(&error))
                        .is_some_and(|(policy, code)| policy.is_retryable(code));

                    // Only draw a delay when the failure deserves one, so that a non-retryable error
                    // does not spend an attempt.
                    match if worth_it { delays.next() } else { None } {
                        Some(wait) => $crate::reexports::tokio::time::sleep(wait).await,
                        None => break Err(error),
                    }
                }
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    /// A failure that carries a code, standing in for whatever a caller's error type is.
    #[derive(Debug, PartialEq)]
    struct Failed(tonic::Code);

    impl GrpcStatus for Failed {
        fn grpc_code(&self) -> Option<tonic::Code> {
            Some(self.0)
        }
    }

    /// Counts what the loop did, so a test asserts on attempts rather than on the outcome alone.
    #[derive(Default)]
    struct Attempts(AtomicU32);

    impl Attempts {
        fn next(&self) -> u32 {
            self.0.fetch_add(1, Ordering::SeqCst) + 1
        }

        fn made(&self) -> u32 {
            self.0.load(Ordering::SeqCst)
        }
    }

    fn policy(max_attempts: u32) -> RetryPolicy {
        RetryPolicy {
            max_attempts,
            ..RetryPolicy::default()
        }
    }

    #[test]
    fn the_defaults_are_the_ones_the_other_clients_use() {
        // `GrpcChannelFactory` hands grpc-dotnet 5 attempts, 1s growing by 1.5 to a 5s ceiling, and
        // replays on Unavailable, Aborted and Unknown. A deployment should not care which client it
        // is talking to. The buffer bound is grpc-dotnet's `MaxRetryBufferPerCallSize`.
        let policy = RetryPolicy::default();

        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_backoff, Duration::from_secs(1));
        assert_eq!(policy.max_backoff, Duration::from_secs(5));
        assert_eq!(policy.backoff_multiplier, 1.5);
        assert_eq!(policy.max_buffer_per_call, 1024 * 1024);
        assert_eq!(policy.max_unary_size, 1024 * 1024);
        assert!(policy.is_retryable(tonic::Code::Unavailable));
        assert!(policy.is_retryable(tonic::Code::Aborted));
        assert!(policy.is_retryable(tonic::Code::Unknown));
        assert!(!policy.is_retryable(tonic::Code::InvalidArgument));
    }

    #[test]
    fn the_bounds_grow_by_the_multiplier_and_stop_at_the_ceiling() {
        let policy = RetryPolicy {
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(4),
            backoff_multiplier: 2.0,
            ..policy(6)
        };

        assert_eq!(
            policy.bounds().collect::<Vec<_>>(),
            [1, 2, 4, 4, 4].map(Duration::from_secs),
            "1, 2, 4, then the ceiling holds"
        );
    }

    #[test]
    fn there_is_one_bound_per_replay_and_none_beyond() {
        assert_eq!(policy(3).bounds().count(), 2, "three attempts, two waits");
        assert_eq!(policy(1).bounds().count(), 0, "one attempt is no replay");
    }

    #[test]
    fn each_wait_is_drawn_below_its_bound() {
        // The specification says `random(0, bound)`, which is what grpc-dotnet draws. Adding jitter to
        // the bound instead, as backoff crates do, would wait three times longer on average.
        let policy = RetryPolicy {
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(8),
            backoff_multiplier: 2.0,
            ..policy(4)
        };
        let bounds: Vec<_> = policy.bounds().collect();

        // Drawn, so one run proves little.
        for _ in 0..1_000 {
            for (wait, bound) in policy.delays().zip(&bounds) {
                assert!(wait <= *bound, "{wait:?} was drawn above {bound:?}");
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_success_is_returned_without_a_second_attempt() {
        let attempts = Attempts::default();

        let outcome: Result<u32, Failed> = retry! {
            policy = Some(policy(5)),
            code = GrpcStatus::grpc_code,
            attempt = { Ok(attempts.next()) }
        };

        assert_eq!(outcome, Ok(1));
        assert_eq!(attempts.made(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_retryable_failure_is_sent_again_until_the_attempts_are_spent() {
        let attempts = Attempts::default();

        let outcome: Result<u32, Failed> = retry! {
            policy = Some(policy(3)),
            code = GrpcStatus::grpc_code,
            attempt = {
                attempts.next();
                Err(Failed(tonic::Code::Unavailable))
            }
        };

        assert_eq!(outcome, Err(Failed(tonic::Code::Unavailable)));
        assert_eq!(
            attempts.made(),
            3,
            "three attempts in all, not three replays"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_failure_that_is_not_retryable_ends_it_at_once() {
        let attempts = Attempts::default();

        let outcome: Result<u32, Failed> = retry! {
            policy = Some(policy(5)),
            code = GrpcStatus::grpc_code,
            attempt = {
                attempts.next();
                Err(Failed(tonic::Code::InvalidArgument))
            }
        };

        assert_eq!(outcome, Err(Failed(tonic::Code::InvalidArgument)));
        assert_eq!(attempts.made(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_failure_carrying_no_status_is_never_replayed() {
        // It did not come back from a server, so there is nothing to say it would go better again.
        let attempts = Attempts::default();

        let outcome: Result<u32, Failed> = retry! {
            policy = Some(policy(5)),
            code = |_: &Failed| None,
            attempt = {
                attempts.next();
                Err(Failed(tonic::Code::Unavailable))
            }
        };

        assert!(outcome.is_err());
        assert_eq!(attempts.made(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn no_policy_is_one_attempt() {
        let attempts = Attempts::default();

        let outcome: Result<u32, Failed> = retry! {
            policy = None::<RetryPolicy>,
            code = GrpcStatus::grpc_code,
            attempt = {
                attempts.next();
                Err(Failed(tonic::Code::Unavailable))
            }
        };

        assert!(outcome.is_err());
        assert_eq!(attempts.made(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn the_last_failure_is_not_followed_by_a_wait() {
        // Sleeping after the attempt that will not be retried delays the answer for nothing. The waits
        // are drawn, so the clock is only evidence once the draw is reproducible: seeded, the elapsed
        // time is exactly the waits between attempts, and one wait too many cannot hide in the noise.
        const SEED: u64 = 0x5EED;
        let policy = policy(4);

        fastrand::seed(SEED);
        let expected: Duration = policy.delays().sum();

        fastrand::seed(SEED);
        let start = tokio::time::Instant::now();
        let _: Result<u32, Failed> = retry! {
            policy = Some(policy),
            code = GrpcStatus::grpc_code,
            attempt = { Err(Failed(tonic::Code::Unavailable)) }
        };

        assert_eq!(
            start.elapsed(),
            expected,
            "one wait per replay, and none after the failure that ends it"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn abandoning_the_call_abandons_the_wait_at_once() {
        // grpc-dotnet's `FailureWithLongDelay_Dispose_CallImmediatelyDisposed`: whoever gives up during
        // a backoff must get the answer immediately, not once the delay has run out.
        let policy = RetryPolicy {
            initial_backoff: Duration::from_secs(600),
            max_backoff: Duration::from_secs(600),
            ..policy(5)
        };
        let start = tokio::time::Instant::now();

        let call = async {
            let outcome: Result<u32, Failed> = retry! {
                policy = Some(policy),
                code = GrpcStatus::grpc_code,
                attempt = { Err(Failed(tonic::Code::Unavailable)) }
            };
            outcome
        };

        // Standing in for a deadline or a cancellation: both are a dropped future.
        let abandoned = tokio::time::timeout(Duration::from_secs(1), call).await;

        assert!(
            abandoned.is_err(),
            "the wait should still have been running"
        );
        assert_eq!(
            start.elapsed(),
            Duration::from_secs(1),
            "giving up must not wait the backoff out"
        );
    }
}
