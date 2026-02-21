// Memtable

// The memtable needs to be able to track the number of active readers and writers threads on live in-flight operations
// This will be used to ensure that the memtable is not dropped or underlying arena doesn't reset leaving us pointing to invalid memory locations
// We also need to track state flags such as whether the memtable is active, immutable, flushing or cleared.
//
// // All public methods on Memtable return either:
//
// 1) A lifetime-bound reference (&'a [u8]) tied to &self
//    — ensuring the returned data cannot outlive the borrowed handle.
//
// 2) An owned copy (e.g. Vec<u8>) if the data must outlive the handle.
//
// Internally, the handle dereferences MemtableInner via raw pointers and
// the skiplist returns a RawSlice { ptr, len } pointing into arena memory.
// That RawSlice is then unsafely converted into &'a [u8], where 'a is
// tied to &self.
//
// Because Memtable refcounts and pins the itself,
// the arena memory remains valid for the duration of the borrow.

use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU16};

use crate::storage::memory::arena::Arena;
use crate::storage::memtable::skip_list::SkipList;

#[repr(u8)]
enum MemLifeCycle {
    Active = 1,
    Freezing = 2,
    Frozen = 3,
    Flushing = 4,
    Flushed = 5,
}

pub(crate) trait MemtableState {}

#[derive(Debug)]
pub(crate) struct Mutable {}
impl MemtableState for Mutable {}

#[derive(Debug)]
pub(crate) struct Immutable {}
impl MemtableState for Immutable {}

#[derive(Debug)]
pub(crate) struct Frozen {}
impl MemtableState for Frozen {}

// Main Memtable
pub(crate) struct Memtable<S: MemtableState> {
    _state: PhantomData<S>,
    inner: Arc<MemtableInner>,
}

impl<S: MemtableState> Clone for Memtable<S> {
    fn clone(&self) -> Self {
        Self {
            _state: PhantomData,
            inner: self.inner.clone(),
        }
    }
}

pub(super) struct MemtableInner {
    lifecycle: AtomicU8,
    ref_count: AtomicU16,
    in_flight_writers: AtomicU16,
    arena: Arena,
    skiplist: SkipList,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::allocator::SystemAllocator;

    #[test]
    fn mem_enum() {
        let mem = MemLifeCycle::Active;
        assert_eq!(mem as u8, 1);
    }

    #[test]
    fn lifetime() {
        let mem: Memtable<Mutable> = Memtable {
            _state: PhantomData,
            inner: Arc::new(MemtableInner {
                lifecycle: AtomicU8::new(MemLifeCycle::Active as u8),
                ref_count: AtomicU16::new(1),
                in_flight_writers: AtomicU16::new(0),
                arena: Arena::new(
                    crate::storage::memory::ArenaSize::Test(10, 20),
                    crate::storage::memory::allocator::Allocator::System(SystemAllocator::new()),
                ),
                skiplist: SkipList::default(),
            }),
        };

        let mem_cloned = mem.clone();

        println!("mem {:?}", mem._state);
        drop(mem);
        println!("mem cloned {:?}", mem_cloned._state);
    }
}
