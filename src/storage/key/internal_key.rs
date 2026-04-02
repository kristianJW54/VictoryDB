// InternalKey is made up of a user key, sequence number, and operation type.
// | user_key (var len) | 8-byte trailer |
// Where:
// (trailer: u64) = (seq_no << 8) | value_type
// +-------------------+-------------------+
// | user key bytes    | 8 byte trailer    |
// +-------------------+-------------------+
//

//

use std::fmt::Display;

use crate::storage::key::{INITIAL_KEY_BUFFER_CAP, MAX_BUFFER_RETAINED};

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

pub(crate) struct InternalKeyBuffer {
    buffer: Vec<u8>,
}

impl InternalKeyBuffer {
    pub(crate) fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(INITIAL_KEY_BUFFER_CAP),
        }
    }

    pub(crate) fn push(&mut self, bytes: &[u8]) {
        let needed = bytes.len();

        if self.buffer.capacity() > MAX_BUFFER_RETAINED && needed < MAX_BUFFER_RETAINED {
            self.buffer.shrink_to(INITIAL_KEY_BUFFER_CAP);
        }

        self.buffer.clear();
        self.buffer.extend_from_slice(bytes);
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
}
