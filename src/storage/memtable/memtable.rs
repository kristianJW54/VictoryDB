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

use std::fmt::Display;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};
use std::sync::atomic::{AtomicU8, AtomicU16};

use crate::storage::key::comparator::Comparator;
use crate::storage::memory::ArenaSize;
use crate::storage::memory::allocator::Allocator;
use crate::storage::memory::arena::Arena;
use crate::storage::memtable::skip_list::SkipList;

pub(crate) type MemID = u64;

#[repr(u8)]
enum MemLifeCycle {
    Active = 1,
    Freezing = 2,
    Frozen = 3,
    Flushing = 4,
    Flushed = 5,
}

impl Display for MemLifeCycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemLifeCycle::Active => write!(f, "Active"),
            MemLifeCycle::Freezing => write!(f, "Freezing"),
            MemLifeCycle::Frozen => write!(f, "Frozen"),
            MemLifeCycle::Flushing => write!(f, "Flushing"),
            MemLifeCycle::Flushed => write!(f, "Flushed"),
        }
    }
}

impl From<u8> for MemLifeCycle {
    fn from(value: u8) -> Self {
        match value {
            1 => MemLifeCycle::Active,
            2 => MemLifeCycle::Freezing,
            3 => MemLifeCycle::Frozen,
            4 => MemLifeCycle::Flushing,
            5 => MemLifeCycle::Flushed,
            _ => panic!("Invalid MemLifeCycle value"),
        }
    }
}

pub(crate) trait MemtableState {
    const NAME: &'static str;
}

#[derive(Debug)]
pub(crate) struct Mutable {}
impl MemtableState for Mutable {
    const NAME: &'static str = "Mutable";
}

#[derive(Debug)]
pub(crate) struct Immutable {}
impl MemtableState for Immutable {
    const NAME: &'static str = "Immutable";
}

#[derive(Debug)]
pub(crate) struct Flushed {}
impl MemtableState for Flushed {
    const NAME: &'static str = "Flushed";
}

impl Display for Flushed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Flushed")
    }
}

// Main Memtable - It is formed as a typestate handle over an Inner which is shared
pub(crate) struct Memtable<S: MemtableState> {
    _state: PhantomData<S>,
    inner: Arc<MemtableInner>,
}

impl<S: MemtableState> Display for Memtable<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Memtable<{}> {{ {} }} ", S::NAME, self.inner)
    }
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
    id: MemID,
    highest_seqno: AtomicU64,
    size: AtomicU64,
    lifecycle: AtomicU8,
    // TODO: May want rotation request bool
    arena: Arena,
    skiplist: SkipList,
}

impl Display for MemtableInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ lifecycle: {} }}",
            Into::<MemLifeCycle>::into(self.lifecycle.load(Ordering::Relaxed)),
        )
    }
}

impl MemtableInner {
    fn new(
        id: MemID,
        arena_size: ArenaSize,
        allocator: Allocator,
        comp: Arc<dyn Comparator>,
    ) -> Self {
        let arena = Arena::new(arena_size, allocator);
        let skiplist = SkipList::new(comp, &arena);
        Self {
            id,
            highest_seqno: AtomicU64::new(0),
            size: AtomicU64::new(0),
            lifecycle: AtomicU8::new(MemLifeCycle::Active as u8),
            arena: arena,
            skiplist,
        }
    }

    fn search(&self, key: &[u8]) -> Option<&[u8]> {
        todo!()
    }
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
