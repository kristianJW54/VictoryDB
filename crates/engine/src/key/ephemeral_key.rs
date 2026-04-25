use super::INITIAL_KEY_BUFFER_CAP;
use super::MAX_BUFFER_RETAINED;
use super::MAX_KEY_SIZE;
use super::OperationType;
use super::inner_key::InnerKey;
use super::internal_key::INLINE_IK_SIZE;
use super::internal_key::encode_trailer;

use crate::thread_ctx::TCTX;

use std::cell::UnsafeCell;
use std::ptr::NonNull;

//
pub(crate) type EphemeralInternalKey = EphemeralKey<INLINE_IK_SIZE>;

/// EphemeralKey is a temporary closure based key wrapper that uses InnerKey for inline storage and a thread-local buffer for heap storage.
/// The main purpose is to provide a short-lived key wrapper that can be used in closures and to fallback to a single thread-local buffer for heap storage.
/// Because the heap storage is thread-local, there is no contention and the overhead of a heap allocation is avoided BUT we must ensure that
/// the buffer is not retained across multiple calls, as the key may be short-lived and the buffer should be cleared after each use.
///
/// Example usage:
/// ```
///
/// let user_key = "key".to_string().into_bytes();
///
/// // Create an EphemeralKey which initializes an inline stack buffer for small keys or does nothing for large keys
/// let mut key = EphemeralKey::new();
///
/// // Use the EphemeralKey in a closure which calls into TCTX thread-local EphemeralBuffer for heap storage
/// key.with_ephemeral_key(&user_key, 10, OperationType::Put, |internal_key| {
///     // Do something with the internal_key in the closure
/// });
/// ```
pub(crate) struct EphemeralKey<const N: usize> {
    _inner: InnerKey<N>,
}

impl<const N: usize> EphemeralKey<N> {
    // TODO: Decide if we need this because we should not be able to create instances of EphemeralKey as it should be short lived within
    // a closure
    pub(crate) fn new() -> Self {
        Self {
            _inner: InnerKey::new(),
        }
    }

    pub(crate) fn with_ephemeral_key<F, R>(
        &mut self,
        user_key: &[u8],
        seq_no: u64,
        op_type: OperationType,
        f: F,
    ) -> R
    where
        // NOTE: We require F to be of the lifetime 'a, so that the borrow checker can ensure we do not return a reference to the
        // ephemeral key buffer as that lifetime is unknown and will be caught at compile time
        F: for<'a> FnOnce(&'a [u8]) -> R,
    {
        let total = user_key.len() + 8;

        // Check if inlining or using TLS

        if total <= N {
            // Now encode the bytes into _inner
            self._inner.encode_inline(user_key, seq_no, op_type);
            return f(self._inner.as_slice());
        }

        // Use TLS buffer as fallback
        self._inner
            .set_external(total, unsafe { NonNull::dangling().as_mut() });
        TCTX.with(|ctx| {
            ctx.inner_key_buf()
                .with_inner_key(user_key, seq_no, op_type, f)
        })
    }
}

pub(crate) struct Ephemeral_Buffer {
    buffer: UnsafeCell<Vec<u8>>,
}

impl Ephemeral_Buffer {
    pub(crate) fn new() -> Self {
        Self {
            buffer: UnsafeCell::new(Vec::with_capacity(INITIAL_KEY_BUFFER_CAP)),
        }
    }

    pub(crate) fn with_inner_key<F, R>(
        &self,
        user_key: &[u8],
        seq_no: u64,
        op: OperationType,
        f: F,
    ) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        let u_len = user_key.len();
        let total = u_len + 8;
        let trailer = encode_trailer(seq_no, op);

        debug_assert!(total <= MAX_KEY_SIZE);

        // TODO: Need Safety comment
        let buf = unsafe { &mut *self.buffer.get() };

        if buf.capacity() > MAX_BUFFER_RETAINED && total < MAX_BUFFER_RETAINED {
            buf.shrink_to(INITIAL_KEY_BUFFER_CAP);
        }

        buf.clear();

        if buf.capacity() < total {
            buf.reserve(total - buf.capacity());
        }
        buf.extend_from_slice(user_key);
        buf.extend_from_slice(&trailer);
        return f(&buf[..total]);
    }
}

#[cfg(test)]
mod tests {
    use super::super::internal_key::InternalKeyRef;
    use super::*;

    #[test]
    fn ephemeral_key_works() {
        let user_key = "User".to_string().into_bytes();
        let seq_no = 12345 as u64;
        let op_type = OperationType::Put;

        // We should not be able to return a reference to the TLS buffer

        // Ephemeral key is just an _inner which can hold inline bytes but fallsback to TLS buffer so we don't hold heap allocated bytes
        // in here
        let mut ek = EphemeralInternalKey::new();

        let _ = EphemeralKey::with_ephemeral_key(&mut ek, &user_key, seq_no, op_type, |key| {
            // Assert inside the closure as we don't ever expose the TLS buffer unless we explicity heap allocated and copied out

            assert_eq!(key.len(), user_key.len() + 8);

            let ik_ref = InternalKeyRef::from(key);
            assert_eq!(ik_ref.user_key, user_key.as_slice());
            assert_eq!(ik_ref.seq_no, seq_no);
            assert_eq!(ik_ref.op, op_type as u8);
        });

        //
        //
    }

    #[test]
    fn ephemeral_inline_works() {
        let user_key = "ReallyLongKeyWhichShouldActuallyHeapAllocateBecauseItIsLongerThan200BytesIHopeThatNobodyAbsolutelyAnihiliatesMyDatabaseWithTheseBecauseThatsNotVeryNiceAndItMakesMeHaveToWorkHardOnMemoryAndMyBrainStrugglesWithMemoryAlready".to_string().into_bytes();
        let inline_key = "User".to_string().into_bytes();
        let seq_no = 12345 as u64;
        let op_type = OperationType::Put;

        let mut ek = EphemeralInternalKey::new();
        let mut ek_2 = EphemeralInternalKey::new();

        let _ = EphemeralKey::with_ephemeral_key(&mut ek, &user_key, seq_no, op_type, |key| {
            assert_eq!(key.len(), user_key.len() + 8);
        });

        let _ = EphemeralKey::with_ephemeral_key(&mut ek_2, &inline_key, seq_no, op_type, |key| {
            assert_eq!(key.len(), inline_key.len() + 8);
        });
    }
}
