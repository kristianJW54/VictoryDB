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
use std::ops::Deref;
use std::ptr::NonNull;

use crate::key::inner_key::{ITER_INLINE, InnerKey, LOOKUP_INLINE};
use crate::key::{INITIAL_KEY_BUFFER_CAP, MAX_BUFFER_RETAINED, MAX_KEY_SIZE, encode_into};
use crate::thread_ctx::TCTX;

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

// LookupKey
pub(crate) type LookUpInternalKey = LookUpKey<LOOKUP_INLINE>;

pub(crate) struct LookUpKey<const N: usize> {
    _inner: InnerKey<N>,
    buffer: Option<Box<[u8]>>,
}

impl<const N: usize> LookUpKey<N> {
    //
    pub(crate) fn new(user_key: &[u8], seq_no: u64, op: OperationType) -> Self {
        //
        let total = user_key.len() + 8;
        debug_assert!(total <= MAX_KEY_SIZE);

        if total <= N {
            let mut inner = InnerKey::new();
            inner.encode_inline(user_key, seq_no, op);
            return Self {
                _inner: inner,
                buffer: None,
            };
        }
        // Allocate the key
        let mut buffer = Vec::with_capacity(total);
        buffer.extend_from_slice(user_key);
        buffer.extend_from_slice(&encode_trailer(seq_no, op));
        let mut inner = InnerKey::new();
        inner.set_external(total, buffer.as_mut_ptr());
        return Self {
            _inner: inner,
            buffer: Some(buffer.into_boxed_slice()),
        };
    }
}

impl<const N: usize> AsRef<[u8]> for LookUpKey<N> {
    fn as_ref(&self) -> &[u8] {
        self._inner.as_slice()
    }
}

// EphemeralKey
pub(crate) type EphemeralInternalKey = EphemeralKey<LOOKUP_INLINE>;

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
        TCTX.with_borrow(|ctx| {
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

// Iter Key - owned by an iterator and re-used across iterations

pub(crate) type InternalIterKey = IterKey<ITER_INLINE>;

pub(crate) struct IterKey<const N: usize> {
    _inner: InnerKey<N>,
    heap: Vec<u8>,
}

impl<const N: usize> IterKey<N> {
    pub(crate) fn new() -> Self {
        Self {
            _inner: InnerKey::new(),
            heap: Vec::with_capacity(INITIAL_KEY_BUFFER_CAP),
        }
    }

    pub(crate) fn set(&mut self, user_key: &[u8], seq_no: u64, op: OperationType) {
        let total = user_key.len() + 8;
        debug_assert!(total <= MAX_KEY_SIZE);

        if total <= N {
            // Can inline
            self._inner.encode_inline(user_key, seq_no, op);
        }

        // Use heap

        self.heap.clear();
        if self.heap.capacity() < total {
            self.heap.reserve(total - self.heap.capacity());
        }

        // Safety
        //
        // We have reserved enough capacity in the heap to hold the full key + trailer,
        // and set_len(total) ensures the heap's length matches the total size.
        unsafe {
            self.heap.set_len(total);
        }

        self._inner.set_external(total, self.heap.as_mut_ptr());
        encode_into(&mut self.heap[..total], user_key, seq_no, op);
    }

    pub(crate) fn set_from_slice(&mut self, slice: &[u8]) {
        let len = slice.len();

        if len <= N {
            self._inner.encode_inline_from_slice(slice)
        }

        self.heap.clear();
        if self.heap.capacity() < len {
            self.heap.reserve(len - self.heap.capacity());
        }

        unsafe {
            self.heap.set_len(len);
        }

        self.heap[..len].copy_from_slice(slice);
    }

    pub(crate) fn as_slice(&self) -> &[u8] {
        self._inner.as_slice()
    }
}

impl<const N: usize> AsRef<[u8]> for IterKey<N> {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

#[cfg(test)]
mod tests {

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
    fn ephemeral_key_works() {
        let user_key = "User".to_string().into_bytes();
        let seq_no = 12345 as u64;
        let op_type = OperationType::Put;

        // We should not be able to return a reference to the TLS buffer

        let mut ek = EphemeralInternalKey::new();

        let result_string =
            EphemeralKey::with_ephemeral_key(&mut ek, &user_key, seq_no, op_type, |key| {
                key.to_vec()
            });

        assert_eq!(result_string.len(), user_key.len() + 8);
        //
        //
        let ik_ref = InternalKeyRef::from(result_string.as_slice());
        assert_eq!(ik_ref.user_key, user_key.as_slice());
        assert_eq!(ik_ref.seq_no, seq_no);
        assert_eq!(ik_ref.op, op_type as u8);
    }

    #[test]
    fn ephemeral_inline_works() {
        let user_key = "ReallyLongKeyWhichShouldActuallyHeapAllocateBecauseItIsLongerThan200BytesIHopeThatNobodyAbsolutelyAnihiliatesMyDatabaseWithTheseBecauseThatsNotVeryNiceAndItMakesMeHaveToWorkHardOnMemoryAndMyBrainStrugglesWithMemoryAlready".to_string().into_bytes();
        let inline_key = "User".to_string().into_bytes();
        let seq_no = 12345 as u64;
        let op_type = OperationType::Put;

        let mut ek = EphemeralInternalKey::new();
        let mut ek_2 = EphemeralInternalKey::new();

        let tls_result =
            EphemeralKey::with_ephemeral_key(&mut ek, &user_key, seq_no, op_type, |key| {
                key.to_vec()
            });

        let inline_result =
            EphemeralKey::with_ephemeral_key(&mut ek_2, &inline_key, seq_no, op_type, |key| {
                key.to_vec()
            });

        assert_eq!(tls_result.len(), user_key.len() + 8);
        assert_eq!(inline_result.len(), inline_key.len() + 8);
    }

    #[test]
    fn lookup_key_size() {
        let user_key = "User".to_string().into_bytes();
        let seq_no = 12345 as u64;
        let op_type = OperationType::Put;

        let lookup_key: LookUpInternalKey = LookUpKey::new(&user_key, seq_no, op_type);
        assert_eq!(std::mem::size_of::<LookUpKey<LOOKUP_INLINE>>(), 40);
    }

    #[test]
    fn lookup_key_works() {
        //
        let user_key = "ReallyLongKeyWhichShouldActuallyHeapAllocateBecauseItIsLongerThan200BytesIHopeThatNobodyAbsolutelyAnihiliatesMyDatabaseWithTheseBecauseThatsNotVeryNiceAndItMakesMeHaveToWorkHardOnMemoryAndMyBrainStrugglesWithMemoryAlready".to_string().into_bytes();
        let inline_key = "User".to_string().into_bytes();
        let seq_no = 12345 as u64;
        let op_type = OperationType::Put;

        let lk: LookUpInternalKey = LookUpKey::new(&user_key, seq_no, op_type);
        let lk_inline: LookUpInternalKey = LookUpKey::new(&inline_key, seq_no, op_type);

        // Test we get back what we want

        let ik = InternalKeyRef::from(lk_inline.as_ref());
        assert_eq!(ik.user_key, inline_key.as_slice());
        assert_eq!(ik.seq_no, seq_no);
        assert_eq!(ik.op, op_type as u8);

        let ik_external = InternalKeyRef::from(lk.as_ref());
        assert_eq!(ik_external.user_key, user_key.as_slice());
        assert_eq!(ik_external.seq_no, seq_no);
        assert_eq!(ik_external.op, op_type as u8);
    }

    #[test]
    fn iter_key_set() {
        let mut iter_key = InternalIterKey::new();

        let user_key = b"User";
        let seq_no = 1;
        let op = OperationType::Put;

        iter_key.set(user_key, seq_no, op);

        let ik = InternalKeyRef::from(iter_key.as_ref());

        assert_eq!(ik.user_key, user_key);
        assert_eq!(ik.seq_no, seq_no);
        assert_eq!(ik.op, op as u8);
    }

    #[test]
    fn iter_key_reuse() {
        let mut iter_key = InternalIterKey::new();

        let user_key = b"Large-User-Key-Which-Should-Cause-A-Heap-Allocation";
        let user_key_2 = b"Another-Large-User-Key-Which-Should-Cause-A-Heap-Allocation";

        //

        iter_key.set(user_key, 1, OperationType::Put);

        let ptr = iter_key.as_slice().as_ptr();

        iter_key.set(user_key_2, 2, OperationType::Put);

        let ptr_2 = iter_key.as_slice().as_ptr();

        assert_eq!(ptr, ptr_2);
    }
}
