// Memtable

// The memtable needs to be able to track the number of active readers and writers threads on live in-flight operations
// This will be used to ensure that the memtable is not dropped or underlying arena doesn't reset leaving us pointing to invalid memory locations
// We also need to track state flags such as whether the memtable is active, immutable, flushing or cleared.

use std::sync::atomic::{AtomicU8, AtomicU16};

use crate::storage::arena::arena::Arena;
use crate::storage::memtable::skip_list::SkipList;

#[repr(u8)]
enum MemLifeCycle {
    Active = 1,
    Transitioning = 2,
    Flushing = 3,
}

#[repr(u8)]
enum MemtableState {
    Immutable = 1,
    Mutable = 2,
}

pub(crate) struct Memtable {
    state: AtomicU8,
    lifecycle: AtomicU8,
    in_flight_readers: AtomicU16,
    in_flight_writers: AtomicU16,
    arena: Arena,
    skiplist: SkipList,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_enum() {
        let mem = MemLifeCycle::Active;
        assert_eq!(mem as u8, 1);
    }
}
