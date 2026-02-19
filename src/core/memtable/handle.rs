use std::ptr::NonNull;

use super::memtable::MemtableInner;

// MemtableHandle is a simple handle to enforce the lifetime and ref count of a Memtable
pub(crate) struct MemtableHandle {
    inner: NonNull<MemtableInner>,
}

// All public methods on MemtableHandle return either:
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
// Because MemtableHandle refcounts and pins the memtable,
// the arena memory remains valid for the duration of the borrow.

struct Key<'a>(&'a [u8]);
struct Value<'a>(&'a [u8]);

struct Entry<'a> {
    key: Key<'a>,
    value: Value<'a>,
}

#[test]
fn test_slice_lifetime() {
    let handle = MemtableHandle {
        inner: NonNull::dangling(),
    };
}
