//
//
// Concurrent Lock-Free Append-Only List//
// IMPORTANT: This List must NOT be used as a replacement for a regular linked list. Nodes in the list will continue to grow and
// not be deleted until arena is dropped. For that reason, this must be used when Nodes are expected to reach a natural limit.
//

use crate::memory::arena::Arena;

use std::cell::Cell;
use std::mem::size_of;
use std::sync::atomic::AtomicPtr;

pub(crate) struct ArenaList<T, const MAX_T_COUNT: usize> {
    arena: Arena,
    head: AtomicPtr<Node<T>>,
}

// Impl
// - const fn for rebuild trigger based on approx memory usage
// - New()
// - Push()
// - Iter()
// - Rebuild()

impl<T, const MAX_T_COUNT: usize> ArenaList<T, MAX_T_COUNT> {
    const fn chunk_size() -> usize {
        /*

        T = 10
        MAX_T_COUNT = 1000
        est arena size = 10000 bytes

        chunk size = MAX_T_COUNT % 10

        */

        size_of::<Node<T>>() * MAX_T_COUNT
    }
}

struct Node<T> {
    active: Cell<u8>,
    next: AtomicPtr<Node<T>>,
}
