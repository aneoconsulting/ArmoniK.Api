/*
 * ArmoniK transport C ABI - GENERATED, DO NOT EDIT.
 *
 * Regenerated on every build of the library and committed, so that a change to this contract shows
 * up in review rather than only in a compiled binary. Edit the sources it is generated from, not
 * this file.
 *
 * This is an HTTP transport, not a gRPC stack. It opens a request, streams bytes both ways, and
 * reports headers, data and trailers. Message framing, `grpc-status`, deadlines and retry belong to
 * whatever speaks gRPC above it.
 *
 * Conventions that the signatures alone do not convey:
 *
 *  - Result codes. AK_OK is 0 and means success; every failure is negative. There is no positive
 *    space: no gRPC status is reported at this level. The value -7 is reserved and never returned.
 *
 *  - An `ak_bytes` is owned by whoever receives it. It comes back only through a synchronous
 *    out-parameter, and is given up by exactly one `ak_bytes_release`, passing the value back
 *    unchanged. Its `ptr`/`len` are a read-only view and its `owner` is opaque, so never
 *    dereference it. The zeroed value means "no data" and is always safe to release.
 *
 *  - An `ak_bytes_in` is the opposite: a view into memory this library does not own, borrowed for
 *    the duration of the call it is passed to. A null pointer or a zero length means "absent".
 *
 *  - Key/value blobs (request headers, response headers, trailers) are encoded as:
 *        uint32 count, then `count` times { uint32 key_len, key bytes, uint32 value_len, value bytes }
 *    Integers are in NATIVE byte order: this is an in-process ABI, not a wire format. Keys may
 *    repeat and are kept in the order they were given. Values are opaque bytes; a `-bin` header's
 *    base64 is the caller's convention and passes through untouched.
 *
 *  - Handles are reference-counted, and thread-safe. A `_release` gives up one reference rather
 *    than destroying anything; the object lives until the last reference goes, so a handle may be
 *    used from any thread and from several at once, `_release` included - a call already under way
 *    when another thread releases the handle finishes normally. What is not allowed is using a
 *    handle after your own `_release`.
 *
 *  - The reactor. One event callback per request, and three rules:
 *      1. Nothing arrives unarmed. One armed read produces exactly one event, and one armed write
 *         produces exactly one event. At most one read and at most one write are armed at a time on
 *         a request.
 *      2. COMPLETED exactly once, and last. If starting a request returns AK_OK, a COMPLETED event
 *         always follows, whatever happens - cancellation, a connection failure, an internal
 *         failure - and anything still armed is resolved by it. No callback is ever made for that
 *         request afterwards, so COMPLETED is where, and only there, the caller gives up whatever
 *         it rooted for `ctx`. If starting the request returns a failure instead, no event ever
 *         arrives.
 *      3. Never a callback during an inbound call. Every function posts a command and returns;
 *         every event is emitted from one of this library's own tasks. There is no reentrancy, and
 *         delivery is serialised per request, so a caller demultiplexing events needs no lock of
 *         its own.
 *
 *  - Event payloads are BORROWED for the duration of the invocation. Copy what is needed before
 *    returning; never release one. Nothing on the event path is owned by whoever receives it.
 *
 *  - The event callback runs on a foreign thread. It must not block, must not throw or otherwise
 *    unwind into this library, and must not re-enter this library for the same request. Deliveries
 *    for one request are serialised, so it is never entered twice at once for the same request; it
 *    may be entered concurrently for different ones. COMPLETED is terminal.
 *
 *  - Two simultaneous events have no promised order. This contract never promises an ordering
 *    between two independent events, so a caller that relies on one has read into it something that
 *    is not there. A test of this library asserts the set of acceptable outcomes, not a single one.
 *
 *  - The ABI is additive only, and `ak_abi_version` reports which revision a loaded library speaks.
 *    A version mismatch is diagnosable that way rather than through a missing entry point. Within
 *    one revision, what may be added: entry points, result codes, event kinds, keys in a blob.
 *    What may never change: the name, the signature or the calling convention of an existing entry
 *    point; the layout of an existing struct; the numeric value or the meaning of an existing
 *    constant; the ownership and lifetime rules above. A caller therefore treats an unknown result
 *    code as a failure and an unknown event kind as one to ignore, which is what lets it keep
 *    working against a library newer than itself.
 */


#ifndef ARMONIK_TRANSPORT_FFI_H
#define ARMONIK_TRANSPORT_FFI_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * The revision of this ABI that this library implements.
 *
 * A caller compiles this value in and compares it against what `ak_abi_version` answers.
 */
#define AK_ABI_VERSION 1

/**
 * The result of a call: `AK_OK` on success, and a negative code otherwise.
 */
enum ak_status
#if defined(__cplusplus) || __STDC_VERSION__ >= 202311L
  : int32_t
#endif // defined(__cplusplus) || __STDC_VERSION__ >= 202311L
 {
  /**
   * The operation succeeded.
   */
  AK_OK = 0,
  /**
   * A pointer argument that must not be null was null.
   */
  AK_NULL_ARGUMENT = -1,
  /**
   * A byte buffer was not valid UTF-8 where UTF-8 was required.
   */
  AK_INVALID_UTF8 = -2,
  /**
   * The client configuration was rejected; the message says what in it.
   */
  AK_INVALID_CONFIG = -3,
  /**
   * The request never reached the server: no connection at all, or a connection that failed
   * before any response header arrived.
   */
  AK_CONNECTION_FAILED = -4,
  /**
   * The handle passed to a call has already been released, or was never valid.
   */
  AK_INVALID_HANDLE = -5,
  /**
   * The object is not in a state that allows the operation.
   */
  AK_INVALID_STATE = -6,
  /**
   * Something inside this library failed in a way that is not the caller's doing; see the message.
   */
  AK_INTERNAL = -8,
  /**
   * A panic was caught at the boundary. This is always a bug in this library; the message carries
   * whatever the panic payload could be turned into.
   */
  AK_INTERNAL_PANIC = -9,
  /**
   * The request was cancelled.
   */
  AK_CANCELLED = -10,
  /**
   * The configured timeout elapsed while the request was still in flight.
   */
  AK_TIMEOUT = -11,
  /**
   * The connection failed after the response headers had arrived: a reset stream, a broken
   * connection, a protocol error.
   */
  AK_TRANSPORT = -12,
};
#ifndef __cplusplus
#if __STDC_VERSION__ >= 202311L
typedef enum ak_status ak_status;
#else
typedef int32_t ak_status;
#endif // __STDC_VERSION__ >= 202311L
#endif // __cplusplus

/**
 * An owned buffer handed to the caller.
 *
 * `ptr`/`len` are a read-only *view*; the allocation itself belongs to `owner`, an opaque handle
 * the caller passes back unchanged and never otherwise touches. Keeping the two apart - rather than
 * treating `ptr` as the thing to release - is what lets a buffer this library already holds cross
 * without being copied: `owner` names whatever really owns those bytes, which may be a
 * reference-counted view into a larger allocation, so `ptr` on its own is not something that can be
 * released.
 *
 * Every buffer with a non-null `owner` must be given up by exactly one call to
 * `ak_bytes_release`. The zeroed value (`ptr` and `owner` null, `len` 0) means "no data" and is
 * always safe to pass there.
 *
 * Input buffers travel as `ak_bytes_in` instead: those are borrowed from the caller and are never
 * released by this library. Only a value this library produced is ever released here.
 *
 * Copying the `(ptr, len, owner)` triple is harmless, which is why this is a plain value. What must
 * never happen, on either side, is passing more than one copy of the same original value to
 * `ak_bytes_release`.
 */
typedef struct ak_bytes {
  /**
   * Pointer to the first byte, or null for an empty or absent buffer. Readable until this value
   * is passed to `ak_bytes_release`; never write through it.
   */
  const uint8_t *ptr;
  /**
   * Number of bytes at `ptr`.
   */
  size_t len;
  /**
   * Opaque; pass back to `ak_bytes_release` unchanged, never dereference or otherwise inspect
   * it.
   */
  void *owner;
} ak_bytes;

/**
 * A borrowed input buffer: a view into memory the *caller* owns.
 *
 * Also what an event payload travels as, in the other direction: borrowed for the duration of the
 * invocation and invalid the moment it returns. Never released by whoever received it. A null `ptr`
 * or a zero `len` means "empty" or "absent".
 */
typedef struct ak_bytes_in {
  /**
   * Pointer to the first byte, or null for an empty or absent buffer.
   */
  const uint8_t *ptr;
  /**
   * Number of bytes at `ptr`.
   */
  size_t len;
} ak_bytes_in;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * The revision of this ABI that the loaded library implements.
 *
 * Queryable rather than only compiled in, because a host process loads one native module and every
 * add-in in it shares whichever was loaded first: an add-in that did not bring its own has to be
 * able to find out what it got. Asking turns a mismatch into a diagnosis, where reaching for an
 * entry point that is not there surfaces as an `EntryPointNotFoundException` from somewhere
 * unrelated.
 *
 * Compare against `AK_ABI_VERSION`, the value this library was compiled with.
 */
int32_t ak_abi_version(void);

/**
 * Give up an `ak_bytes` previously returned by this library.
 *
 * Safety:
 *
 * `bytes` must be a value this library returned, not yet released. Passing a borrowed input buffer, a
 * value already released, or a value with an `owner` this library did not produce, is undefined
 * behaviour. The zeroed value is always safe to pass here.
 */
void ak_bytes_release(struct ak_bytes bytes);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* ARMONIK_TRANSPORT_FFI_H */
