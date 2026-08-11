//! Keeping a handle alive for as long as a call is using it.
//!
//! An opaque handle reaches the caller as a pointer, and every entry point has to turn one back into
//! something it can read. Doing that with `Box::into_raw`/`Box::from_raw` and a set of live
//! addresses is not enough, and the gap is not theoretical: the check that a pointer is live and the
//! use of what it points at are two separate moments, so a release on another thread lands between
//! them and deallocates the object a call is halfway through reading. A host application whose UI
//! thread abandons a request while a pool thread is still writing to it does exactly that.
//!
//! So the registry *owns* the handles. Every live handle is an [`Arc`] the registry holds, and an
//! entry point takes a counted reference for the duration of its call. A release drops the
//! registry's reference; the allocation goes away when the last call using it has returned. Which is
//! also why the name is `_release` and not `_free`: the call gives back a reference, it does not
//! destroy an object.
//!
//! One gap remains, and it is worth stating plainly: a pointer used after its release is rejected
//! only until the allocator hands that address to a new handle of the same type, at which point the
//! stale pointer resolves to the new object rather than being refused. Closing that needs
//! generation-tagged slots that are never reused, which is a larger structure than the handful of
//! call sites warrants.

use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

/// The live handles of one type, keyed by the address the caller holds.
///
/// Wrapped in a `OnceLock` by every call site, rather than being a plain `static Registry =
/// Registry::new()`, because `HashMap::new` reads from the OS to seed its hasher and so is not a
/// `const fn`, which a `static` initializer requires.
///
/// An [`RwLock`] rather than a `Mutex`: [`Self::get`] runs on the hot path - every call on every
/// concurrent handle - while insertion and removal happen once per handle. A mutex would serialise
/// all of those against each other for no reason.
pub(crate) struct Registry<T> {
    live: RwLock<HashMap<usize, Arc<T>>>,
}

impl<T> Registry<T> {
    pub(crate) fn new() -> Self {
        Self {
            live: RwLock::new(HashMap::new()),
        }
    }

    /// Take ownership of `value` and return the address the caller identifies it by.
    ///
    /// The address is the `Arc`'s payload, which does not move for the life of the allocation.
    pub(crate) fn insert(&self, value: T) -> *const T {
        let value = Arc::new(value);
        let ptr = Arc::as_ptr(&value);
        let previous = self
            .live
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(ptr as usize, value);
        assert!(
            previous.is_none(),
            "a handle was registered at an address already in use"
        );
        ptr
    }

    /// A counted reference to the live handle at `ptr`, or `None` if there is none.
    ///
    /// Holding the returned `Arc` is what makes it safe to keep reading the handle after this
    /// returns: a concurrent release drops the registry's reference, not this one.
    pub(crate) fn get(&self, ptr: *const T) -> Option<Arc<T>> {
        self.live
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&(ptr as usize))
            .cloned()
    }

    /// Give up the registry's reference to the handle at `ptr`.
    ///
    /// `None` means this address is not currently a live handle of this type - either it has already
    /// been released, or it never was one - which is what catches a double release. The returned
    /// `Arc` is the registry's own reference; dropping it releases the allocation only if no call is
    /// still using it.
    pub(crate) fn remove(&self, ptr: *const T) -> Option<Arc<T>> {
        self.live
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&(ptr as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_pointer_is_not_live() {
        let registry = Registry::<u32>::new();
        let value = 0u32;
        assert!(registry.get(std::ptr::addr_of!(value)).is_none());
    }

    #[test]
    fn a_registered_handle_is_reachable_until_it_is_removed() {
        let registry = Registry::new();
        let ptr = registry.insert(7u32);

        assert_eq!(registry.get(ptr).map(|value| *value), Some(7));
        assert!(registry.remove(ptr).is_some());
        assert!(registry.get(ptr).is_none());
    }

    #[test]
    fn removing_an_absent_pointer_is_reported_rather_than_silently_ignored() {
        let registry = Registry::<u32>::new();
        let value = 0u32;

        // Never registered: this is what catches a double release or a bogus pointer.
        assert!(registry.remove(std::ptr::addr_of!(value)).is_none());
    }

    #[test]
    fn a_double_remove_only_succeeds_once() {
        let registry = Registry::new();
        let ptr = registry.insert(7u32);

        assert!(registry.remove(ptr).is_some());
        assert!(
            registry.remove(ptr).is_none(),
            "the second release of the same handle must be rejected"
        );
    }

    #[test]
    fn a_handle_released_while_a_call_holds_it_stays_readable() {
        // The race the whole module exists for, in its smallest form. A borrowed handle survives a
        // release that lands while the call is still using it; under a set-of-addresses scheme this
        // read is a use-after-free.
        let registry = Registry::new();
        let ptr = registry.insert(String::from("still here"));

        let borrowed = registry.get(ptr).expect("live");
        assert!(registry.remove(ptr).is_some());
        assert!(
            registry.get(ptr).is_none(),
            "the handle is gone as far as any new call is concerned"
        );
        assert_eq!(borrowed.as_str(), "still here");
    }

    /// A handle address on its way to another thread, which is what a caller does with one.
    struct Shared(*const String);

    impl Shared {
        /// Read through the wrapper rather than the field, so that a closure captures `&Shared` -
        /// which is what the `Sync` below is about - instead of capturing the bare pointer.
        fn ptr(&self) -> *const String {
            self.0
        }
    }

    // SAFETY: the address is only ever used as a key into the registry. Nothing dereferences it, so
    // sending it between threads carries no access to what it points at.
    unsafe impl Send for Shared {}
    // SAFETY: as above.
    unsafe impl Sync for Shared {}

    #[test]
    fn the_same_handle_read_and_released_from_many_threads_at_once_is_never_a_use_after_free() {
        // The single-threaded test above pins the shape; this one puts real contention on it, which
        // is what a sanitiser or Miri needs in order to have anything to observe. Every thread reads
        // through the reference it was handed and every thread also tries to release, so the read
        // and the release genuinely overlap rather than merely being interleaved on paper.
        //
        // Scaled down under Miri, which interprets every access and would otherwise take hours.
        const THREADS: usize = if cfg!(miri) { 4 } else { 64 };
        const ITERATIONS: usize = if cfg!(miri) { 20 } else { 200 };

        let registry = Registry::new();
        let shared = Shared(registry.insert(String::from("still here")));
        let released = std::sync::atomic::AtomicUsize::new(0);

        std::thread::scope(|scope| {
            for _ in 0..THREADS {
                scope.spawn(|| {
                    for iteration in 0..ITERATIONS {
                        // Whether the handle is still live is a race, and either answer is correct.
                        // What is under test is that reading through the reference `get` hands back
                        // never touches a released allocation.
                        if let Some(borrowed) = registry.get(shared.ptr()) {
                            assert_eq!(borrowed.as_str(), "still here");
                        }
                        if iteration == ITERATIONS / 2 && registry.remove(shared.ptr()).is_some() {
                            released.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                });
            }
        });

        assert_eq!(
            released.load(std::sync::atomic::Ordering::Relaxed),
            1,
            "exactly one of the concurrent releases owns the handle"
        );
        assert!(registry.get(shared.ptr()).is_none());
    }
}
