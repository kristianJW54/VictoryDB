// InternalKey is made up of a user key, sequence number, and operation type.
// | user_key (var len) | 8-byte trailer |
// Where:
// (trailer: u64) = (seq_no << 8) | value_type
// +-------------------+-------------------+
// | user key bytes    | 8 byte trailer    |
// +-------------------+-------------------+
//

//

const INLINE_IK_SIZE: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum OperationType {
    Put = 1,
    Delete = 2,
    Merge = 3,       // TODO: Implement Merge Operation into the system
    RangeDelete = 4, // TODO: Implement RangeDelete Operation into the system
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
            4 => OperationType::RangeDelete,
            _ => unreachable!(),
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
pub(crate) struct InternalKeyRef<'a>(&'a [u8]);

//
// LookupKey is a temporary struct used for internal key operations. Mainly on read and search operations but the LookupKey can
// also be used by the writer on the write path if Arena Direct is not selected. This way a temp scratch buffer or inline key will be created on
// write operations also.
#[repr(C)]
pub(crate) struct LookupKey {
    len: u32,
    inline: [u8; INLINE_IK_SIZE],
    heap: Option<Box<[u8]>>,
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
