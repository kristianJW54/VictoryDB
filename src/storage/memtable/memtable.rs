// Memtable

// The memtable needs to be able to track the number of active readers and writers threads on live in-flight operations
// This will be used to ensure that the memtable is not dropped or underlying arena doesn't reset leaving us pointing to invalid memory locations
// We also need to track state flags such as whether the memtable is active, immutable, flushing or cleared.

use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::AtomicU16;

use crate::storage::memtable::skip_list::SkipList;

#[repr(u8)]
enum MemLifeCycle {
    Active = 1,
    Transitioning = 2,
    Flushing = 3,
}

trait MemtableState {}

struct Immutable {}
impl MemtableState for Immutable {}

struct Mutable {}
impl MemtableState for Mutable {}

struct Memtable<S: MemtableState> {
    _state: PhantomData<S>,
    arena: Arc<()>, // Put the arena here
    lifecycle: MemLifeCycle,
    in_flight_readers: AtomicU16,
    in_flight_writers: AtomicU16,
    skiplist: SkipList,
}

// With this type state we can make compile time gurantees that the memtable is not written to while it is an immutable memtable. We can also transition state
// And allow writers to drain.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_enum() {
        let mem = MemLifeCycle::Active;
        assert_eq!(mem as u8, 1);
    }
}
