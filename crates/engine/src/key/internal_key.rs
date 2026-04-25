//
//
//
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

pub(super) const INLINE_IK_SIZE: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum OperationType {
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
