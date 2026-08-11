//! Fixtures shared by this crate's unit tests.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// Poll `future` to completion on this thread, with no runtime at all.
///
/// Deliberately not `#[tokio::test]`. Some of these tests run under Miri, which cannot drive tokio's
/// I/O reactor - it reaches `mio`'s IOCP calls and stops with "unsupported operation" - and none of
/// what they test needs one: the question is always what happens *inside a poll*, which a bare loop
/// asks more directly than a runtime would.
///
/// Only safe for a future that parks a bounded number of times, since a no-op waker wakes nobody and
/// this would otherwise spin. Everything it is used on parks at most once, by construction.
pub(crate) fn block_on<F: Future>(mut future: Pin<&mut F>) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
    }
}

/// Park once, then resume - so whatever follows it runs from the *second* poll.
///
/// That is the case worth testing: a panic on the first poll would also be caught by an ordinary
/// `catch_unwind` around the call that created the future.
pub(crate) async fn yield_once() {
    let mut parked = false;
    std::future::poll_fn(move |context| {
        if parked {
            Poll::Ready(())
        } else {
            parked = true;
            context.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await;
}
