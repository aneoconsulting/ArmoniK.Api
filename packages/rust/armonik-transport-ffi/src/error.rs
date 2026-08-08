//! Owned and borrowed byte buffers handed across the ABI.

use bytes::Bytes;

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
}
