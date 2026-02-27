// SkipList

// For the skip list node we will be using the flexible array member (FAM) concept from C99.
// I reference this https://john-millikin.com/rust-and-dynamically-sized-thin-pointers for a rust implementation
//
// Header will hold the height of the tower, key len, value len and flags
// Tower [ptr;0] will then server as a marker ptr for the tower atomic pointers
//
// ┌─────────────────────┐
// │ Node header         │
// ├─────────────────────┤
// │ tower[0]            │ level 0
// │ tower[1]            │ level 1
// │ tower[2]            │ level 2
// │ ...                 │ up to height
// ├─────────────────────┤
// │ key bytes           │ key_len
// ├─────────────────────┤
// │ value bytes / ptr   │ val_len or sizeof(ptr)
// └─────────────────────┘

use std::array;
use std::ops::Deref;
use std::ptr::{self, NonNull};
use std::sync::atomic::AtomicUsize;
use std::{alloc::Layout, sync::atomic::AtomicPtr};

use crate::storage::memory::arena::Arena;

// ------------------------------------------------------

#[derive(Debug)]
pub(crate) enum SkipListError {
    LayoutError(std::alloc::LayoutError),
    Arena(crate::storage::memory::arena::ArenaError),
}

impl From<std::alloc::LayoutError> for SkipListError {
    fn from(err: std::alloc::LayoutError) -> Self {
        SkipListError::LayoutError(err)
    }
}

impl From<crate::storage::memory::arena::ArenaError> for SkipListError {
    fn from(err: crate::storage::memory::arena::ArenaError) -> Self {
        SkipListError::Arena(err)
    }
}

// We introduce a max head height // NOTE: Later we may want this configurable
const HEAD_HEIGHT: usize = 8;

#[repr(C)]
pub(super) struct Header {
    pointers: [AtomicPtr<Node>; HEAD_HEIGHT],
}

impl Header {
    pub(crate) fn new() -> Self {
        let array: [AtomicPtr<Node>; HEAD_HEIGHT] =
            array::from_fn(|_| AtomicPtr::new(ptr::null_mut()));
        Self { pointers: array }
    }
}

#[repr(C)]
pub(crate) struct Node {
    // NOTE: Crossbeam uses refs as well here - but I think this is because it needs reclamation through EBR but since
    // We're using Arena, refs on the node are not needed
    //
    // Number of levels of this node
    height: u16, // TODO: Can we make this smaller since we aren't tracking refs?
    key_len: u16,
    value_len: u32,
    //
    pub(crate) tower: [AtomicPtr<Node>; 0],
}

impl Node {
    //
    //
    fn build_layout(
        height: usize,
        key_len: usize,
        value_len: usize,
    ) -> Result<Layout, SkipListError> {
        // Get basic layout for the node
        let mut layout = Layout::new::<Self>();

        // Now we now extend for the height of the tower
        layout = layout
            .extend(Layout::array::<AtomicPtr<Node>>(height)?)
            .map_err(SkipListError::LayoutError)?
            .0;

        // Now we add the key and value bytes length as part of the layout to be allocated
        // These are just u8s so should be simple with no padding
        layout = layout
            .extend(Layout::array::<u8>(key_len)?)
            .map_err(SkipListError::LayoutError)?
            .0;
        layout = layout
            .extend(Layout::array::<u8>(value_len)?)
            .map_err(SkipListError::LayoutError)?
            .0;

        Ok(layout)
    }

    #[inline]
    unsafe fn init_node(ptr_memory: NonNull<u8>, height: u16, key_len: u16, value_len: u32) {
        let node = ptr_memory.as_ptr() as *mut Node;

        unsafe {
            ptr::write(
                node,
                Node {
                    height,
                    key_len,
                    value_len,
                    tower: [AtomicPtr::new(ptr::null_mut()); 0],
                },
            );

            for i in 0..height as usize {
                Self::tower_ptr(node)
                    .add(i)
                    .write(AtomicPtr::new(ptr::null_mut()));
            }

            // TODO: We could also initialize the key and value bytes to zero here OR leave MaybeUninit but we would have to ensure that
            // we only assumit_init() when we know the key and value are initialized
            // TODO: If we do leave MaybeUninit, how do we use assume_init() when we want to read the key and value bytes?
        }
    }

    // Pointers to get for the skiplist to handle
    //
    #[inline(always)]
    unsafe fn tower_ptr(node: *mut Node) -> *mut AtomicPtr<Node> {
        unsafe { (node as *mut u8).add(core::mem::offset_of!(Node, tower)) as *mut AtomicPtr<Node> }
    }

    #[inline(always)]
    unsafe fn tower_level(node: *mut Node, index: usize) -> *const AtomicPtr<Node> {
        debug_assert!(index <= unsafe { (*node).height as usize });
        unsafe { Self::tower_ptr(node).add(index) }
    }

    #[inline(always)]
    unsafe fn key_ptr(node: *mut Node) -> *mut u8 {
        let key_ptr = unsafe {
            (Self::tower_ptr(node) as *mut u8)
                .add((*node).height as usize * std::mem::size_of::<AtomicPtr<Node>>())
        };
        key_ptr
    }

    #[inline(always)]
    unsafe fn value_ptr(node: *mut Node) -> *mut u8 {
        let value_ptr = unsafe { (Self::key_ptr(node) as *mut u8).add((*node).value_len as usize) };
        value_ptr
    }

    // TODO: Can we be clearer about the init_node?
    // TODO: Think about where this is called and used internally
    unsafe fn alloc(
        arena: &Arena,
        height: u16,
        key_len: u16,
        value_len: u32,
    ) -> Result<*mut Node, SkipListError> {
        debug_assert!(height as usize <= HEAD_HEIGHT);
        let layout = Self::build_layout(height as usize, key_len as usize, value_len as usize)?;
        unsafe {
            let ptr = arena.alloc_raw(layout)?;
            Self::init_node(ptr, height, key_len, value_len);
            return Ok(ptr.as_ptr() as *mut Node);
        };
    }
}

// NOTE:
// For the SkipList we want to make sure that certain fields which are concurrently accessed often are given their own cache line
// A great explanation and gathering of sources is in crossbema -> https://github.com/crossbeam-rs/crossbeam/blob/master/crossbeam-utils/src/cache_padded.rs#L150
//
// For now, we will default to aligning to 64 bytes and over time consider using more alignment for different sources

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[repr(align(64))]
struct CachePadded<T> {
    value: T,
}

unsafe impl<T> Send for CachePadded<T> {}
unsafe impl<T> Sync for CachePadded<T> {}

impl<T> Deref for CachePadded<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

// We need data for the SkipList such as:
// - Seed for random number generation
// - Entries in the skip list
// - Max level

struct Data {
    seed: AtomicUsize,
    entries: AtomicUsize,
    max_level: AtomicUsize,
}

// VictoryDB SkipList is backed by an aligned arena.
// TODO: describe and use diagram

// SkipList
pub(super) struct SkipList {
    head: Header,
    data: CachePadded<Data>,
}

#[cfg(test)]
mod tests {
    use crate::storage::memory::{
        ArenaSize,
        allocator::{Allocator, SystemAllocator},
    };

    use super::*;

    #[test]
    fn basic_node_layout() {
        //
        let arena = Arena::new(
            ArenaSize::Test(80, 160),
            Allocator::System(SystemAllocator::new()),
        );

        // Now we want to alloc a node

        let node = unsafe { Node::alloc(&arena, 1, 1, 0).unwrap() };
        unsafe {
            ptr::write(Node::key_ptr(node), 24);
        }
        let node2 = unsafe { Node::alloc(&arena, 1, 1, 0).unwrap() };
        unsafe {
            ptr::write(Node::key_ptr(node2), 89);
        }
        println!("arena new = {:?}", arena.get_current_init_slice());
    }
}
