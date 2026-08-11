//! The one length-prefixed encoding every list of key/value pairs crosses this ABI in: request
//! headers, response headers, trailers.
//!
//! One encoder and decoder rather than a format per use means one implementation to get right, one
//! set of bounds checks, and one test suite covering the malformed cases.
//!
//! ```text
//! u32 count
//! repeated count times {
//!     u32 key_len;   key_len bytes
//!     u32 value_len; value_len bytes
//! }
//! ```
//!
//! Integers are in native byte order. Both ends of this ABI are one process built for one target, so
//! a reader decodes exactly what the writer encoded, whatever that target's byte order happens to
//! be. The generated header says as much, because the only thing that would make byte order anyone
//! else's business is this encoding being mistaken for a wire format.
//!
//! Keys and values are opaque bytes: a caller that needs text validates it itself. A `-bin` header
//! value is raw binary, and leaving it alone keeps the decoder free of any policy.

use crate::error::{ak_bytes, FfiError};

/// The largest key or value this format can describe, imposed by the `u32` length prefix.
const MAX_CHUNK: usize = u32::MAX as usize;

/// Key/value pairs borrowed straight out of the caller's blob, in the order they appeared.
///
/// Borrowed rather than owned: nothing here outlives the call that decoded it, and a caller of
/// [`decode`] turns the pairs into its own representation straight away.
pub(crate) type Pairs<'a> = Vec<(&'a [u8], &'a [u8])>;

/// Decode a blob into borrowed key/value pairs.
///
/// # Safety
///
/// `data` must point to `len` valid bytes for the duration of the borrow, or both must be zero.
pub(crate) unsafe fn decode<'a>(data: *const u8, len: usize) -> Result<Pairs<'a>, FfiError> {
    // SAFETY: forwarded from this function's own contract.
    let bytes = if data.is_null() || len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }
    };

    if bytes.is_empty() {
        // No count prefix at all is the same as a count of zero, not a truncated blob.
        return Ok(Vec::new());
    }

    let mut cursor = bytes;
    let count = read_u32(&mut cursor)? as usize;

    // Every entry costs at least two length prefixes, so a count that could not possibly fit is a
    // malformed blob rather than something to start allocating for.
    if count.saturating_mul(8) > cursor.len() {
        return Err(FfiError::InvalidState("truncated key/value blob"));
    }

    let mut pairs = Vec::with_capacity(count);
    for _ in 0..count {
        let key = read_chunk(&mut cursor)?;
        let value = read_chunk(&mut cursor)?;
        pairs.push((key, value));
    }

    Ok(pairs)
}

/// Encode key/value pairs into an owned blob the caller releases.
pub(crate) fn encode<'a>(
    pairs: impl IntoIterator<Item = (&'a [u8], &'a [u8])>,
) -> Result<ak_bytes, FfiError> {
    encode_vec(pairs).map(ak_bytes::from_bytes)
}

/// Encode key/value pairs into a buffer.
///
/// What [`encode`] is built on, and what an emitter of borrowed payloads uses directly: a payload
/// valid only for the duration of an invocation is kept on the emitter's own stack, so there is
/// nothing to own or to release on the other side.
pub(crate) fn encode_vec<'a>(
    pairs: impl IntoIterator<Item = (&'a [u8], &'a [u8])>,
) -> Result<Vec<u8>, FfiError> {
    let mut out = Vec::new();
    // Reserved up front and patched at the end, so the count does not have to be known before the
    // entries are visited.
    out.extend_from_slice(&0u32.to_ne_bytes());
    let mut count = 0u32;

    for (key, value) in pairs {
        write_chunk(&mut out, key)?;
        write_chunk(&mut out, value)?;
        count += 1;
    }

    out[0..4].copy_from_slice(&count.to_ne_bytes());
    Ok(out)
}

fn read_u32(cursor: &mut &[u8]) -> Result<u32, FfiError> {
    if cursor.len() < 4 {
        return Err(FfiError::InvalidState("truncated key/value blob"));
    }
    let (head, tail) = cursor.split_at(4);
    *cursor = tail;
    Ok(u32::from_ne_bytes(
        head.try_into().expect("split_at(4) yields 4 bytes"),
    ))
}

fn read_chunk<'a>(cursor: &mut &'a [u8]) -> Result<&'a [u8], FfiError> {
    let len = read_u32(cursor)? as usize;
    if cursor.len() < len {
        return Err(FfiError::InvalidState("truncated key/value blob"));
    }
    let (head, tail) = cursor.split_at(len);
    *cursor = tail;
    Ok(head)
}

fn write_chunk(out: &mut Vec<u8>, data: &[u8]) -> Result<(), FfiError> {
    // Refused rather than truncated: silently writing a prefix of the value would corrupt the blob
    // and, for a metadata value or a certificate, quietly change what was sent.
    let len = u32::try_from(data.len())
        .map_err(|_| FfiError::InvalidState("a key/value blob entry exceeds 4 GiB"))?;
    debug_assert!(data.len() <= MAX_CHUNK);
    out.extend_from_slice(&len.to_ne_bytes());
    out.extend_from_slice(data);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode, then decode, keeping the encoded buffer alive for the borrowed result.
    fn round_trip(pairs: &[(&[u8], &[u8])]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let encoded = encode(pairs.iter().copied()).expect("encode");
        // SAFETY: just produced above and freed at the end of this function.
        let decoded = unsafe { decode(encoded.ptr, encoded.len) }.expect("decode");
        let owned = decoded
            .into_iter()
            .map(|(key, value)| (key.to_vec(), value.to_vec()))
            .collect();
        // SAFETY: produced by `encode`, freed exactly once here.
        unsafe { crate::error::ak_bytes_release(encoded) };
        owned
    }

    #[test]
    fn pairs_round_trip_in_order() {
        let pairs: &[(&[u8], &[u8])] = &[
            (b"first", b"one"),
            (b"second", b"two"),
            // Duplicate keys are the caller's business, not the format's: metadata allows them.
            (b"first", b"again"),
        ];

        let decoded = round_trip(pairs);
        assert_eq!(
            decoded,
            vec![
                (b"first".to_vec(), b"one".to_vec()),
                (b"second".to_vec(), b"two".to_vec()),
                (b"first".to_vec(), b"again".to_vec()),
            ]
        );
    }

    #[test]
    fn arbitrary_bytes_survive_including_nul_and_high_bytes() {
        // The format carries opaque bytes, so a value must not be treated as a C string or as text.
        let value: &[u8] = &[0, 1, 0xff, 0xfe, 0, b'x'];
        let decoded = round_trip(&[(b"key-bin", value)]);
        assert_eq!(decoded, vec![(b"key-bin".to_vec(), value.to_vec())]);
    }

    #[test]
    fn empty_keys_and_values_survive() {
        let decoded = round_trip(&[(b"", b""), (b"key", b"")]);
        assert_eq!(
            decoded,
            vec![(Vec::new(), Vec::new()), (b"key".to_vec(), Vec::new()),]
        );
    }

    #[test]
    fn no_pairs_encodes_to_just_a_count() {
        let encoded = encode(std::iter::empty()).expect("encode");
        assert_eq!(encoded.len, 4, "only the count prefix");
        // SAFETY: produced by `encode` just above, freed once.
        let decoded = unsafe { decode(encoded.ptr, encoded.len) }.expect("decode");
        assert!(decoded.is_empty());
        unsafe { crate::error::ak_bytes_release(encoded) };
    }

    #[test]
    fn a_null_or_empty_blob_decodes_to_no_pairs() {
        // SAFETY: the null/zero case is explicitly allowed by the contract.
        let decoded = unsafe { decode(std::ptr::null(), 0) }.expect("decode");
        assert!(decoded.is_empty());
    }

    #[test]
    fn a_truncated_blob_is_rejected_rather_than_read_out_of_bounds() {
        let cases: Vec<Vec<u8>> = vec![
            // A count claiming one entry, but nothing after it.
            1u32.to_ne_bytes().to_vec(),
            // A key length claiming more bytes than are present.
            {
                let mut blob = 1u32.to_ne_bytes().to_vec();
                blob.extend_from_slice(&100u32.to_ne_bytes());
                blob.extend_from_slice(b"short");
                blob
            },
            // A well-formed key, then a value length running past the end.
            {
                let mut blob = 1u32.to_ne_bytes().to_vec();
                blob.extend_from_slice(&3u32.to_ne_bytes());
                blob.extend_from_slice(b"key");
                blob.extend_from_slice(&100u32.to_ne_bytes());
                blob
            },
            // A partial count prefix.
            vec![1, 0],
        ];

        for blob in cases {
            // SAFETY: `blob` is a live slice for its own length across the call.
            let result = unsafe { decode(blob.as_ptr(), blob.len()) };
            assert!(result.is_err(), "should have been rejected: {blob:?}");
        }
    }

    #[test]
    fn an_absurd_count_does_not_trigger_a_huge_allocation() {
        // A hostile blob claiming four billion entries in eight bytes must be rejected on the
        // arithmetic, before `Vec::with_capacity` is asked for the space.
        let mut blob = u32::MAX.to_ne_bytes().to_vec();
        blob.extend_from_slice(&0u32.to_ne_bytes());
        // SAFETY: a live slice for its own length.
        let result = unsafe { decode(blob.as_ptr(), blob.len()) };
        assert!(result.is_err());
    }
}
