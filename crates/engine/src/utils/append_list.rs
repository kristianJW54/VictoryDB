//
//
// Concurrent Lock-Free Append-Only List//
// IMPORTANT: This List must NOT be used as a replacement for a regular linked list. Nodes in the list will continue to grow and
// not be deleted until arena is dropped. For that reason, this must be used when Nodes are expected to reach a natural limit.
//

use crate::memory::ArenaSize;
use crate::memory::allocator::{Allocator, SystemAllocator};
use crate::memory::arena::Arena;

use std::alloc::Layout;
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, AtomicPtr};

//
//
pub(crate) struct ArenaList<T: Sized> {
    arena: Arena,
    head: AtomicPtr<Node<T>>,
}

// Impl
// - New()
// - Push()
// - Iter()
// - Rebuild()

impl<T> ArenaList<T> {
    //
    pub(crate) fn new(nodes_per_chunk: usize, max_nodes: usize) -> Self {
        //
        debug_assert!(nodes_per_chunk <= max_nodes);

        let cap = size_of::<Node<T>>() * max_nodes;
        let chunk_size = size_of::<Node<T>>() * nodes_per_chunk;

        Self {
            arena: Arena::new(
                ArenaSize::Custom(chunk_size, cap),
                Allocator::System(SystemAllocator::new()),
            ),
            head: AtomicPtr::new(std::ptr::null_mut()),
        }
    }
}

#[repr(C)]
struct Node<T: Sized> {
    next: AtomicPtr<Node<T>>,
    data: T,
    active: AtomicBool,
}

// Impl
// - Alloc
// - Init_node()?
// - Load_next()?

impl<T> Node<T> {
    fn new(data: T) -> Self {
        Self {
            next: AtomicPtr::new(std::ptr::null_mut()),
            data,
            active: AtomicBool::new(true),
        }
    }

    fn alloc(arena: &Arena, data: T) {
        //
        let node = Node::new(data);

        let layout = Layout::new::<Node<T>>();

        match unsafe { arena.alloc_raw_fallback(layout) } {
            Ok(_) => {}
            Err(e) => println!("I want to try and rebuild the arena: {:?}", e),
        }
    }
}
