//! Keeping panics on the Rust side of the ABI.
//!
//! A panic unwinding out of an `extern "C"` function into whatever called it is undefined behaviour.
//! Every entry point runs its body through one of the functions below rather than running it
//! directly, so a panic turns into a result code instead of crossing the boundary.

use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::status::ak_status;

/// Run a fallible entry point that has no way to report a message, only a result code.
pub(crate) fn catch_unwind_status_only(body: impl FnOnce() -> i32) -> i32 {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or(ak_status::AK_INTERNAL_PANIC.code())
}

/// Run an entry point that returns nothing, such as a `_release` function.
///
/// A panic is swallowed: there is no return value to signal it through, and these functions must not
/// be allowed to abort the process either.
pub(crate) fn catch_unwind_void(body: impl FnOnce()) {
    let _ = catch_unwind(AssertUnwindSafe(body));
}

/// Run `future` to completion, turning a panic inside it into an `Err` carrying its message.
///
/// [`catch_unwind`] only wraps a closure, and the standard library has no equivalent for a future,
/// so this catches around each individual poll - which is where a panic escapes from. It is for work
/// that does not run inside an entry point: a panic on a runtime thread of its own cannot cross the
/// ABI by unwinding, but it must not be silently swallowed either, or a caller waits for a result
/// that is never coming.
///
/// Boxed rather than pin-projected by hand, which is what keeps this free of `unsafe`: a
/// `Pin<Box<dyn Future>>` is `Unpin`, so its `poll` is reachable through an ordinary `&mut`.
///
/// A panic leaves the future half-finished, so it is dropped without being polled again - the same
/// rule as any other future that has returned `Ready`.
pub(crate) async fn catch_unwind_future<'a, T>(
    future: std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>,
) -> Result<T, String> {
    struct CatchUnwind<'a, T>(std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>);

    impl<T> std::future::Future for CatchUnwind<'_, T> {
        type Output = Result<T, String>;

        fn poll(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            // `Self` is `Unpin` (it holds only a boxed future), so this projection is the safe one.
            let inner = &mut self.get_mut().0;
            match catch_unwind(AssertUnwindSafe(|| inner.as_mut().poll(cx))) {
                Ok(std::task::Poll::Pending) => std::task::Poll::Pending,
                Ok(std::task::Poll::Ready(value)) => std::task::Poll::Ready(Ok(value)),
                Err(payload) => std::task::Poll::Ready(Err(panic_message(payload.as_ref()))),
            }
        }
    }

    CatchUnwind(future).await
}

/// Render a panic payload as the message that crosses the ABI.
///
/// The payload survives in every build. It is prose this crate wrote - "a handle was registered at
/// an address already in use" - and names no file and no type; a panic's location goes to the panic
/// hook rather than into the payload.
pub(crate) fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        format!("panicked: {message}")
    } else if let Some(message) = payload.downcast_ref::<String>() {
        format!("panicked: {message}")
    } else {
        String::from("panicked with a non-string payload")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_only_reports_the_panic_status_without_a_message() {
        assert_eq!(
            catch_unwind_status_only(|| panic!("boom")),
            ak_status::AK_INTERNAL_PANIC.code()
        );
        assert_eq!(catch_unwind_status_only(|| 7), 7);
    }

    #[test]
    fn void_swallows_the_panic() {
        catch_unwind_void(|| panic!("boom"));
        // Reaching here at all is the assertion.
    }

    #[test]
    fn a_future_that_panics_reports_what_it_panicked_about() {
        use crate::test_support::{block_on, yield_once};

        let future: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> =
            Box::pin(async {
                // From the second poll, so this is a panic escaping a `poll` rather than the call
                // that built the future - the only case that needs catching here.
                yield_once().await;
                panic!("the transport fell over");
            });

        let mut caught = Box::pin(catch_unwind_future(future));
        let message = block_on(caught.as_mut()).expect_err("the panic should be reported");

        assert!(
            message.contains("the transport fell over"),
            "the payload is the diagnosis, so it has to survive: {message}"
        );
    }

    #[test]
    fn a_future_that_does_not_panic_passes_its_value_through() {
        use crate::test_support::{block_on, yield_once};

        let future: std::pin::Pin<Box<dyn std::future::Future<Output = i32> + Send>> =
            Box::pin(async {
                yield_once().await;
                7
            });
        let mut caught = Box::pin(catch_unwind_future(future));
        assert_eq!(block_on(caught.as_mut()), Ok(7));
    }

    #[test]
    fn panic_message_reads_string_payloads() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            panic!("specific message");
        }));
        let Err(payload) = result else {
            panic!("expected a panic");
        };
        assert!(panic_message(payload.as_ref()).contains("specific message"));
    }
}
