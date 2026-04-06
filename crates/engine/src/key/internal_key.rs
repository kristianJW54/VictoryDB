// InternalKey is made up of a user key, sequence number, and operation type.
// | user_key (var len) | 8-byte trailer |
// Where:
// (trailer: u64) = (seq_no << 8) | value_type
// +-------------------+-------------------+
// | user key bytes    | 8 byte trailer    |
// +-------------------+-------------------+
//

//

use std::cell::UnsafeCell;
use std::fmt::Display;
use std::time::Instant;

use crate::key::inner_key::{ITER_INLINE, InnerKey, LOOKUP_INLINE};
use crate::key::{INITIAL_KEY_BUFFER_CAP, MAX_BUFFER_RETAINED, MAX_KEY_SIZE, encode_into};

const INLINE_IK_SIZE: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum OperationType {
    // NOTE: Put is aslo used as a sentinel value for lookup operations, because OperationType factors into the comparison order as it is packed into
    // the trailer (seq_no << 8) | op_type we must make sure that any LookupKey is not overshooting skip list keys so we give it zero op_type and let
    // the seq_no be the comparisons decider
    Put = 1,
    Delete = 2,
    Merge = 3, // TODO: Implement Merge Operation into the system
    Max = 255,
}

impl From<OperationType> for u64 {
    fn from(op: OperationType) -> Self {
        u64::from(op as u8)
    }
}

impl From<u8> for OperationType {
    fn from(op: u8) -> Self {
        match op {
            1 => OperationType::Put,
            2 => OperationType::Delete,
            3 => OperationType::Merge,
            255 => OperationType::Max,
            _ => unreachable!(),
        }
    }
}

impl Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::Put => write!(f, "Put"),
            OperationType::Delete => write!(f, "Delete"),
            OperationType::Merge => write!(f, "Merge"),
            OperationType::Max => write!(f, "Max"),
        }
    }
}

// A Pack function to take the seq_no and operation type and pack them into a trailer u64
#[inline(always)]
fn pack_trailer(seq_no: u64, op: OperationType) -> u64 {
    debug_assert!(seq_no < (1 << 56)); // Enforce that seq_no is less than 2^56
    (seq_no << 8) | u64::from(op)
}

#[inline(always)]
#[must_use = "trailer bytes should be big endian in order to be compared correctly"]
pub(crate) fn encode_trailer(seq_no: u64, op: OperationType) -> [u8; 8] {
    pack_trailer(seq_no, op).to_be_bytes()
}

#[inline(always)]
fn unpack_trailer_raw(trailer: u64) -> (u64, u8) {
    (trailer >> 8, (trailer & 0xff) as u8)
}

#[inline(always)]
fn unpack_trailer(trailer: u64) -> (u64, OperationType) {
    let (seq_no, op) = unpack_trailer_raw(trailer);
    (seq_no, OperationType::from(op))
}

#[inline(always)]
fn extract_seq_no(trailer: u64) -> u64 {
    trailer >> 8
}

#[inline(always)]
fn extract_op(trailer: u64) -> OperationType {
    OperationType::from((trailer & 0xff) as u8)
}

#[inline(always)]
fn extract_op_raw(trailer: u64) -> u8 {
    (trailer & 0xff) as u8
}

// TODO: Finish the internal key logic
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct InternalKeyRef<'a> {
    pub(crate) user_key: &'a [u8],
    pub(crate) seq_no: u64,
    pub(crate) op: u8,
    // NOTE: Add Trailer instead for lazy decoding
}

impl<'a> From<&'a [u8]> for InternalKeyRef<'a> {
    fn from(key: &'a [u8]) -> Self {
        debug_assert!(key.len() >= 8, "InternalKey must include trailer");

        let (user_key, trailer_bytes) = key.split_at(key.len() - 8);
        let trailer = u64::from_be_bytes(trailer_bytes.try_into().unwrap());

        let (seq_no, op) = unpack_trailer_raw(trailer);

        Self {
            user_key,
            seq_no,
            op,
        }
    }
}

impl<'a> Display for InternalKeyRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key = String::from_utf8_lossy(self.user_key);
        write!(
            f,
            "{}-{}-{}",
            key,
            self.seq_no,
            OperationType::from(self.op)
        )
    }
}

//--------------------- Moving Internal Key handling to TLS Buffer -------------------------------//

// EphemeralKey

pub(crate) struct EphemeralKey<const N: usize> {
    _inner: InnerKey<N>,
}

impl<const N: usize> EphemeralKey<N> {
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
        F: FnOnce(&[u8]) -> R,
    {
        let total = user_key.len() + 8;

        // Check if inlining or using TLS

        if total <= N {
            // Now encode the bytes into _inner
            self._inner.encode_inline(user_key, seq_no, op_type);
            return f(self._inner.as_slice());
        }

        // TODO: Finish from here

        //
        //
        todo!()
    }
}

pub(crate) struct InternalKeyBuffer {
    buffer: UnsafeCell<Vec<u8>>,
}

impl InternalKeyBuffer {
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

        // Fast inline path
        if u_len + 8 <= INLINE_IK_SIZE {
            let mut buf = [0u8; INLINE_IK_SIZE];
            buf[..u_len].copy_from_slice(user_key);
            buf[u_len..total].copy_from_slice(&trailer);
            return f(&buf[..total]);
        }

        // Else slow path - use scratch buffer
        let buf = unsafe { &mut *self.buffer.get() };
        debug_assert!(buf.len() >= total);

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

//--------------------- Moving Internal Key handling to TLS Buffer -------------------------------//

//
// LookupKey is a temporary struct used for internal key operations. Mainly on read and search operations but the LookupKey can
// also be used by the writer on the write path if Arena Direct is not selected. This way a temp scratch buffer or inline key will be created on
// write operations also.
#[repr(C)]
pub(crate) struct InternalKey {
    len: u32,
    inline: [u8; INLINE_IK_SIZE],
    heap: Option<Box<[u8]>>,
}

impl InternalKey {
    pub(crate) fn new(key: &[u8], seq_no: u64, op_type: OperationType) -> Self {
        let trailer = encode_trailer(seq_no, op_type);
        let len_key = key.len() + 8;

        let mut this = Self {
            len: len_key as u32,
            inline: [0u8; INLINE_IK_SIZE],
            heap: None,
        };

        if len_key <= INLINE_IK_SIZE {
            this.inline[..key.len()].copy_from_slice(key);
            this.inline[key.len()..].copy_from_slice(&trailer);
        } else {
            let mut buf = vec![0u8; len_key].into_boxed_slice();
            buf[..key.len()].copy_from_slice(key);
            buf[key.len()..].copy_from_slice(&trailer);
            this.heap = Some(buf);
        }

        this
    }
}

impl<'a> From<&'a InternalKey> for InternalKeyRef<'a> {
    fn from(key: &'a InternalKey) -> Self {
        InternalKeyRef::from(key.as_ref())
    }
}

impl AsRef<[u8]> for InternalKey {
    fn as_ref(&self) -> &[u8] {
        if let Some(ref heap) = self.heap {
            &heap[..self.len as usize]
        } else {
            &self.inline[..self.len as usize]
        }
    }
}

pub(crate) struct LookupKey(InternalKey);

impl LookupKey {
    pub(crate) fn new(key: &[u8], seq_no: u64) -> Self {
        Self(InternalKey::new(key, seq_no, OperationType::Max))
    }
}

impl AsRef<[u8]> for LookupKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use crate::thread_ctx::TCTX;

    use super::*;

    #[test]
    fn encode_trailer_works() {
        let trailer_1 = encode_trailer(12345 as u64, OperationType::Put);
        let trailer_2 = encode_trailer(12346 as u64, OperationType::Put);

        assert!(
            trailer_2 > trailer_1,
            "trailer_2 should be greater than trailer_1"
        );
    }

    #[test]
    fn inner_key() {
        let mut user_key = Vec::new();
        user_key.extend_from_slice(b"Hello".as_slice());

        // TLS Approach
        TCTX.with(|v| {
            // In here we would then take a reference to scratch
            //

            v.inner_key_buf()
                .with_inner_key(&user_key, 10, OperationType::Put, |byte| {
                    println!("{}", InternalKeyRef::from(byte))
                })
        });

        // Normal Approach
        let inner_key = InternalKeyBuffer::new();

        inner_key.with_inner_key(&user_key, 10, OperationType::Put, |byte| {
            println!("{}", InternalKeyRef::from(byte))
        })
    }
}
