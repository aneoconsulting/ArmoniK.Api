//! How many requests a client lets out per unit of time.
//!
//! `RateLimit` is written `count/duration`, and means what `tower::limit::RateLimit` means: at most
//! `count` requests are admitted in any one window of `duration`, and the window restarts at the
//! first request after the previous one ended. A burst of `count` therefore goes straight out, and
//! the `count + 1`th waits for the rest of the window.
//!
//! Written here rather than taken from `tower`, which would bring in a service stack this crate has
//! no other use for: the whole of the policy is two numbers, a deadline and a counter.
//!
//! Nothing in it blocks a thread. A permit is taken by awaiting, and the only wait is a
//! [`tokio::time`] sleep, so the runtime thread that would have waited runs other work instead. That
//! matters at an ABI boundary: an entry point that blocked on a rate limit would hold a host
//! application's thread for as long as the caller's own configuration says to.

use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::Instant;

/// A limiter admitting `limit` requests per `window`.
pub(crate) struct RateLimiter {
    /// Requests admitted per window. Above zero, which is what the option's reader guarantees.
    limit: u64,
    /// How long one window lasts. Above zero, likewise.
    window: Duration,
    /// What is left of the window in progress.
    ///
    /// An async mutex rather than a `std` one, because a waiter holds it across its sleep: that is
    /// what puts waiters in a queue rather than letting them wake together and all take the same
    /// permit. `tokio`'s mutex is fair, so the queue is the order they arrived in.
    state: Mutex<Window>,
}

/// The window in progress, and how much of its allowance is left.
#[derive(Debug, Clone, Copy)]
struct Window {
    /// Permits left before `until`.
    remaining: u64,
    /// When the window ends and the allowance is restored.
    until: Instant,
}

/// What a permit request resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// A permit was taken.
    Admitted,
    /// The allowance is spent; the next one arrives at this instant.
    WaitUntil(Instant),
}

impl RateLimiter {
    /// A limiter for the `(count, duration)` pair the `RateLimit` option reads.
    ///
    /// # Panics
    ///
    /// Panics if either half is zero, which the option's reader refuses first: a zero count admits
    /// nothing ever, and a zero window divides by no time at all. A caller that built an
    /// [`armonik_transport::HttpConfig`] by hand and set the field itself reaches this, and a panic
    /// at an ABI boundary is caught by the guard the entry point runs under.
    pub(crate) fn new((count, duration): (u64, Duration)) -> Self {
        assert!(count > 0, "a rate limit admitting nothing is not a limit");
        assert!(!duration.is_zero(), "a rate limit needs a window to be per");
        Self {
            limit: count,
            window: duration,
            // The first request opens the first window, rather than the window starting whenever
            // the client happened to be created: a client built and left idle for an hour must not
            // owe an hour of allowance to whoever finally uses it.
            state: Mutex::new(Window {
                remaining: count,
                until: Instant::now(),
            }),
        }
    }

    /// Wait until this limiter admits one more request.
    ///
    /// Never blocks the thread: the wait is a [`tokio::time`] sleep, so the runtime runs other work
    /// while it lasts. Requires a runtime with a time driver, which is what this crate's own
    /// runtime is.
    pub(crate) async fn acquire(&self) {
        let mut state = self.state.lock().await;
        loop {
            match state.take(self.limit, self.window, Instant::now()) {
                Step::Admitted => return,
                // The guard is held across the sleep on purpose: the next waiter is admitted after
                // this one rather than racing it for the permit this sleep is waiting for.
                Step::WaitUntil(deadline) => tokio::time::sleep_until(deadline).await,
            }
        }
    }
}

impl Window {
    /// Take one permit at `now`, or say when the next one falls due.
    ///
    /// Separate from the sleeping, so that what the policy decides is a pure function of the state
    /// and the clock: everything below is testable without a runtime, and so under Miri.
    fn take(&mut self, limit: u64, window: Duration, now: Instant) -> Step {
        if now >= self.until {
            // The window in progress is over, so this request opens the next one.
            *self = Window {
                remaining: limit - 1,
                until: now + window,
            };
            return Step::Admitted;
        }
        if self.remaining == 0 {
            return Step::WaitUntil(self.until);
        }
        self.remaining -= 1;
        Step::Admitted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A window that has just opened with `limit` permits, `window` long.
    fn opened(limit: u64, window: Duration, now: Instant) -> Window {
        let mut state = Window {
            remaining: limit,
            until: now,
        };
        assert_eq!(state.take(limit, window, now), Step::Admitted);
        state
    }

    #[test]
    fn a_burst_up_to_the_limit_is_admitted_without_waiting() {
        let now = Instant::now();
        let window = Duration::from_secs(1);
        let mut state = opened(3, window, now);

        // One is already taken by opening the window, so two more fill it.
        assert_eq!(state.take(3, window, now), Step::Admitted);
        assert_eq!(state.take(3, window, now), Step::Admitted);
        assert_eq!(
            state.take(3, window, now),
            Step::WaitUntil(now + window),
            "the fourth request of a limit of three waits for the window to end"
        );
    }

    #[test]
    fn the_allowance_comes_back_when_the_window_ends() {
        let now = Instant::now();
        let window = Duration::from_secs(1);
        let mut state = opened(1, window, now);
        assert_eq!(state.take(1, window, now), Step::WaitUntil(now + window));

        // At the instant the window ends, not after it: a waiter woken by `sleep_until` sees
        // exactly this instant, and a rule that needed one tick more would park it again forever.
        let next = now + window;
        assert_eq!(state.take(1, window, next), Step::Admitted);
        assert_eq!(
            state.take(1, window, next),
            Step::WaitUntil(next + window),
            "the new window runs from the request that opened it"
        );
    }

    #[test]
    fn an_idle_stretch_grants_one_window_and_not_the_ones_it_slept_through() {
        // The property that makes this a rate limit rather than a budget: a client left alone for a
        // minute at 2 per second does not come back owing 120 requests.
        let now = Instant::now();
        let window = Duration::from_secs(1);
        let mut state = opened(2, window, now);

        let much_later = now + Duration::from_secs(60);
        assert_eq!(state.take(2, window, much_later), Step::Admitted);
        assert_eq!(state.take(2, window, much_later), Step::Admitted);
        assert_eq!(
            state.take(2, window, much_later),
            Step::WaitUntil(much_later + window)
        );
    }

    #[test]
    fn a_limit_of_one_admits_one_request_per_window() {
        let now = Instant::now();
        let window = Duration::from_millis(250);
        let mut state = opened(1, window, now);

        assert_eq!(state.take(1, window, now), Step::WaitUntil(now + window));
        assert_eq!(
            state.take(1, window, now + Duration::from_millis(249)),
            Step::WaitUntil(now + window),
            "still inside the window"
        );
        assert_eq!(state.take(1, window, now + window), Step::Admitted);
    }

    #[test]
    #[should_panic(expected = "a rate limit admitting nothing is not a limit")]
    fn a_zero_count_is_refused_rather_than_admitting_nothing_forever() {
        let _ = RateLimiter::new((0, Duration::from_secs(1)));
    }

    #[test]
    #[should_panic(expected = "a rate limit needs a window to be per")]
    fn a_zero_window_is_refused() {
        let _ = RateLimiter::new((1, Duration::ZERO));
    }

    #[test]
    // The runtime is what a limiter waits on, and Miri cannot drive one.
    #[cfg_attr(miri, ignore)]
    fn acquiring_beyond_the_limit_waits_for_the_window_rather_than_the_thread() {
        // Paused time, so the assertion is on what the limiter waited *for* rather than on how long
        // a test took: `tokio` advances the clock only when every task is parked, which is exactly
        // the case where a limiter that blocked its thread would hang instead.
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .start_paused(true)
            .build()
            .expect("a current-thread runtime with a clock");

        runtime.block_on(async {
            let limiter = RateLimiter::new((2, Duration::from_secs(1)));
            let start = Instant::now();

            limiter.acquire().await;
            limiter.acquire().await;
            assert_eq!(
                Instant::now(),
                start,
                "a burst up to the limit waits for nothing"
            );

            limiter.acquire().await;
            assert_eq!(
                Instant::now() - start,
                Duration::from_secs(1),
                "the third waits exactly the rest of the window"
            );
        });
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn waiters_are_admitted_one_window_at_a_time_and_in_order() {
        // Several tasks contending, which is what a client under load looks like. What has to hold
        // is that they do not all wake and take the same permit: at one per window, the n-th is
        // admitted n-1 windows in.
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .start_paused(true)
            .build()
            .expect("a current-thread runtime with a clock");

        runtime.block_on(async {
            let limiter = std::sync::Arc::new(RateLimiter::new((1, Duration::from_secs(1))));
            let start = Instant::now();
            let mut waiters = Vec::new();

            for _ in 0..4 {
                let limiter = std::sync::Arc::clone(&limiter);
                waiters.push(tokio::spawn(async move {
                    limiter.acquire().await;
                    Instant::now() - start
                }));
            }

            let mut admitted = Vec::new();
            for waiter in waiters {
                admitted.push(waiter.await.expect("the waiter runs to completion"));
            }
            admitted.sort_unstable();

            assert_eq!(
                admitted,
                [0, 1, 2, 3].map(Duration::from_secs),
                "one window each, rather than all four at once"
            );
        });
    }
}
