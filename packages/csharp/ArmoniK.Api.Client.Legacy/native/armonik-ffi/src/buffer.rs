//! Bytes handed across the ABI: Rust allocates, .NET reads them and calls back to free them.

use crate::error::guard;

/// A byte buffer Rust allocated. `ptr` is null and `len` is `0` for "nothing to report", which
/// [`ak_bytes_free`] accepts as a no-op rather than a null-pointer error.
///
/// `ptr` was allocated by this crate's own global allocator; freeing it any other way, including
/// letting the process exit without calling [`ak_bytes_free`], leaks or corrupts the heap.
#[repr(C)]
pub struct ak_bytes {
    pub ptr: *const u8,
    pub len: usize,
}

impl ak_bytes {
    /// The empty buffer: what every entry point returns in an out-parameter it has nothing to
    /// write, so a caller can always initialise its own storage from this rather than a literal.
    pub const EMPTY: Self = Self {
        ptr: std::ptr::null(),
        len: 0,
    };

    /// Takes ownership of `bytes`, to be handed back to Rust through [`ak_bytes_free`].
    pub fn from_boxed(bytes: Box<[u8]>) -> Self {
        if bytes.is_empty() {
            return Self::EMPTY;
        }
        let len = bytes.len();
        // SAFETY: `Box::into_raw` on a `Box<[u8]>` gives a fat pointer; `as *const u8` narrows it to
        // its data pointer, and `len` is recorded here for `ak_bytes_free` to reconstruct the slice.
        let ptr = Box::into_raw(bytes) as *const u8;
        Self { ptr, len }
    }

    pub fn from_vec(bytes: Vec<u8>) -> Self {
        Self::from_boxed(bytes.into_boxed_slice())
    }

    pub fn from_string(text: String) -> Self {
        Self::from_vec(text.into_bytes())
    }
}

/// Releases a buffer this crate allocated. Safe to call on [`ak_bytes::EMPTY`]; calling it twice on
/// the same non-empty buffer, or on one this crate did not allocate, is undefined behaviour.
///
/// # Safety
///
/// `bytes.ptr`/`bytes.len` must be exactly what an `armonik-ffi` function returned, unmodified, and
/// must not have already been freed.
#[no_mangle]
pub unsafe extern "C" fn ak_bytes_free(bytes: ak_bytes) {
    let _ = guard(|| {
        if !bytes.ptr.is_null() {
            // SAFETY: `ptr`/`len` name a `Box<[u8]>` this crate leaked in `from_boxed`, per this
            // function's own contract; reconstructing it here and letting it drop is exactly
            // undoing that leak.
            let slice_ptr = std::ptr::slice_from_raw_parts_mut(bytes.ptr as *mut u8, bytes.len);
            drop(unsafe { Box::from_raw(slice_ptr) });
        }
        0
    });
}
