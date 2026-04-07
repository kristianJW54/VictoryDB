// Memtable

use std::fmt::Display;
use std::marker::PhantomData;
use std::ptr::{self, NonNull};
use std::slice;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use std::sync::atomic::{AtomicU8, AtomicU16};

use crate::iterator::internal_iterator::InternalIterator;
use crate::key::comparator::Comparator;
use crate::key::internal_key::{InternalKeyRef, OperationType, encode_trailer};
use crate::memory::ArenaSize;
use crate::memory::allocator::Allocator;
use crate::memory::arena::Arena;
use crate::memtable::skip_list::{Iter, Node, SkipList};

pub(crate) type MemID = u64;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MemReturn<'a> {
    NotFound,
    Merge,
    Deleted,
    Value(&'a [u8]),
}

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

impl<S: MemtableState> Memtable<S> {
    pub(crate) fn new_memtable() -> Memtable<Mutable> {
        todo!()
    }

    unsafe fn encode_key(&self, ptr: *mut Node, user_key: &[u8], seq_no: u32, op_type: u32) {
        todo!()
    }
}

impl Memtable<Mutable> {
    //
    pub(crate) fn new(
        id: MemID,
        arena_size: ArenaSize,
        allocator: Allocator,
        comp: Arc<dyn Comparator>,
    ) -> Self {
        Self {
            _state: PhantomData,
            inner: Arc::new(MemtableInner::new(id, arena_size, allocator, comp)),
        }
    }

    pub(crate) fn insert(&self, key: &[u8], value: &[u8]) {
        self.inner.insert(key, value)
    }

    // TODO: Do we want the Value(v) to include the key and value?
    pub(crate) fn get(&self, key: &[u8]) -> MemReturn<'_> {
        if let Some((skip_key, v)) = self.inner.first_ge(key) {
            let sk = InternalKeyRef::from(skip_key);
            let lookup = InternalKeyRef::from(key.as_ref());

            if sk.user_key != lookup.user_key {
                return MemReturn::NotFound;
            }

            match sk.op.into() {
                OperationType::Put => MemReturn::Value(v),
                OperationType::Delete => MemReturn::Deleted,
                OperationType::Merge => MemReturn::Merge,
                _ => unreachable!(),
            }
        } else {
            MemReturn::NotFound
        }
    }

    pub(crate) fn iter(&self) -> MemtableIterator<'_> {
        self.inner.iter()
    }
}

pub(super) struct MemtableInner {
    id: MemID,
    highest_seqno: AtomicU64,
    size: AtomicU64,
    requested_rotation: AtomicBool,
    lifecycle: AtomicU8,
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
        let skiplist = SkipList::new(comp.clone(), &arena);
        Self {
            id,
            highest_seqno: AtomicU64::new(0),
            size: AtomicU64::new(0),
            requested_rotation: AtomicBool::new(false),
            lifecycle: AtomicU8::new(MemLifeCycle::Active as u8),
            arena: arena,
            skiplist,
        }
    }

    fn first_ge(&self, key: &[u8]) -> Option<(&[u8], &[u8])> {
        let node = self.skiplist.search(key).successors[0];
        if !node.is_null() {
            return Some((
                Node::get_key_bytes(node as *mut Node),
                Node::get_value_bytes(node as *mut Node),
            ));
        }
        None
    }

    fn insert(&self, key: &[u8], value: &[u8]) {
        let _ = unsafe { self.skiplist.insert(key, value, &self.arena) };
    }

    // NOTE: If we insert direct we have to make sure that the internal key seq no is greater than the highest seq no so we don't fail on insert and alloc
    // A dead node
    fn insert_direct(&self, user_key: &[u8], seq_no: u64, op_type: OperationType, value: &[u8]) {
        let user_key_len = user_key.len();

        unsafe {
            self.skiplist
                .insert_with((user_key_len + 8) as u16, value, &self.arena, |node_ptr| {
                    // Insert the user key
                    ptr::copy_nonoverlapping(
                        user_key.as_ptr(),
                        Node::key_ptr(node_ptr),
                        user_key_len,
                    );
                    // Insert the trailer
                    ptr::copy_nonoverlapping(
                        encode_trailer(seq_no, op_type).as_ptr(),
                        Node::key_ptr(node_ptr).add(user_key_len),
                        8,
                    );
                });
        }
    }

    fn iter(&self) -> MemtableIterator<'_> {
        MemtableIterator {
            sl: &self.skiplist,
            item: self.skiplist.iter(),
            current: None,
        }
    }

    fn iter_from(&self, key: &[u8]) -> MemtableIterator<'_> {
        MemtableIterator {
            sl: &self.skiplist,
            item: self.skiplist.seek(key),
            current: None,
        }
    }
}

pub(crate) struct MemtableIterator<'a> {
    sl: &'a SkipList,
    item: Iter<'a>,
    current: Option<NonNull<Node>>,
}

impl<'a> MemtableIterator<'a> {
    pub(crate) fn internal_key(&self) -> InternalKeyRef<'_> {
        InternalKeyRef::from(self.key())
    }
}

impl<'a> InternalIterator for MemtableIterator<'a> {
    fn valid(&self) -> bool {
        self.current.is_some()
    }

    fn seek_to_first(&mut self) {
        self.item = self.sl.iter();
        self.current = self
            .item
            .next()
            .map(|ptr| unsafe { NonNull::new_unchecked(ptr) });
    }

    fn seek(&mut self, key: &[u8]) {
        self.item = self.sl.seek(key);
        self.current = self
            .item
            .next()
            .map(|ptr| unsafe { NonNull::new_unchecked(ptr) });
    }

    fn next(&mut self) {
        self.current = self
            .item
            .next()
            .map(|ptr| unsafe { NonNull::new_unchecked(ptr) });
    }

    fn key(&self) -> &[u8] {
        debug_assert!(self.valid());
        Node::get_key_bytes(self.current.unwrap().as_ptr())
    }

    fn value(&self) -> &[u8] {
        debug_assert!(self.valid());
        Node::get_value_bytes(self.current.unwrap().as_ptr())
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
