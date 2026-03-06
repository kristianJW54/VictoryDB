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

use std::ops::Deref;
use std::ptr::{self, NonNull};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{alloc::Layout, sync::atomic::AtomicPtr};
use std::{array, slice};

use crate::storage::comparator::{Comparator, DefaultComparator};
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
const MAX_HEAD_HEIGHT: usize = 8;

#[repr(C)]
pub(super) struct Header {
    pointers: [AtomicPtr<Node>; MAX_HEAD_HEIGHT],
}

impl Header {
    pub(crate) fn new() -> Self {
        let array: [AtomicPtr<Node>; MAX_HEAD_HEIGHT] =
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
    unsafe fn node_at_tower_level(node: *mut Node, index: usize) -> *const AtomicPtr<Node> {
        debug_assert!(index <= unsafe { (*node).height as usize });
        unsafe { Self::tower_ptr(node).add(index) }
    }

    // SAFETY: We still leave the caller responsible for ensuring that the pointers are compared from a valid arena allocation and not arbitrary pointers.
    #[inline]
    unsafe fn tower_height(node: *mut Node) -> usize {
        // Find the difference between the tower ptr and the key_ptr and then divide by 8
        //
        // SAFETY: Because the two pointers are aligned and are from the same arena allocation we can safely subtract them.
        // Key ptr will always be after the tower ptr in memory, so the difference will be positive and represent the tower height.
        unsafe {
            (Node::key_ptr(node).addr() - Node::tower_ptr(node).addr())
                / std::mem::size_of::<AtomicPtr<Node>>()
        }
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
        debug_assert!(height as usize <= MAX_HEAD_HEIGHT);
        let layout = Self::build_layout(height as usize, key_len as usize, value_len as usize)?;
        unsafe {
            let ptr = arena.alloc_raw(layout)?;
            Self::init_node(ptr, height, key_len, value_len);
            return Ok(ptr.as_ptr() as *mut Node);
        };
    }

    pub(super) fn get_key_bytes<'a>(node: *mut Node) -> &'a [u8] {
        unsafe { slice::from_raw_parts(Node::key_ptr(node), (*node).key_len as usize) }
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

pub(super) struct Data {
    pub(super) seed: AtomicUsize,
    pub(super) entries: AtomicUsize,
    pub(super) max_level: AtomicUsize,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            seed: AtomicUsize::new(0),
            entries: AtomicUsize::new(0),
            max_level: AtomicUsize::new(MAX_HEAD_HEIGHT),
        }
    }
}

pub(super) struct TraversalCtx {
    // Searched node is Some when we find the node we're searching for - useful for insertions where we can detect duplicates
    pub(super) searched_node: Option<NonNull<Node>>,
    // Predecessors need to be AtomicPtr<Node> because we need to access the node and modify it's next pointers
    pub(super) predecessors: [*mut AtomicPtr<Node>; MAX_HEAD_HEIGHT],
    // Successors only need to be *const Node because we only need to modify our own next pointers
    pub(super) successors: [*const Node; MAX_HEAD_HEIGHT],
}

impl TraversalCtx {
    pub(crate) fn new() -> Self {
        Self {
            searched_node: None,
            predecessors: [ptr::null_mut(); MAX_HEAD_HEIGHT],
            successors: [ptr::null(); MAX_HEAD_HEIGHT],
        }
    }
}

impl Default for TraversalCtx {
    fn default() -> Self {
        Self::new()
    }
}

// VictoryDB SkipList is backed by an aligned arena.
// TODO: describe and use diagram

// SkipList
pub(super) struct SkipList {
    pub(super) head: Header,
    data: CachePadded<Data>,
    // We use Arc here because the Comparator is global law for ordering and must be shared across memtables and ssTables
    comparator: Arc<dyn Comparator>,
    // Metrics?
}

impl Default for SkipList {
    fn default() -> Self {
        Self::new(Arc::new(DefaultComparator {}))
    }
}

impl SkipList {
    pub(super) fn new(comparator: Arc<dyn Comparator>) -> Self {
        let data = CachePadded {
            value: Data {
                seed: AtomicUsize::new(0),
                entries: AtomicUsize::new(0),
                max_level: AtomicUsize::new(0),
            },
        };
        Self {
            head: Header::new(),
            data,
            comparator,
            // Metrics?
        }
    }

    fn search(&self, key: &[u8]) -> TraversalCtx {
        //

        // We need an outer loop so that we can reattempt the search if we encounter a concurrent modification
        unsafe {
            'outer: loop {
                let mut t = TraversalCtx::default();

                // Load the level to decrement from in while loop
                let mut level = self.data.max_level.load(Ordering::Relaxed);

                // We can optimize by skipping levels which have no immediate successors and start straight away at the traversal level
                while level >= 1
                    && self
                        .head
                        .pointers
                        .get_unchecked(level - 1)
                        .load(Ordering::Relaxed)
                        .is_null()
                {
                    level -= 1;
                }

                // We are at a level which we can move right
                // Store the predecessor to keep track and update when we reach the end of the level
                //
                // TODO: Need pointer to the tower of the node
                let mut pred = self.head.pointers;

                while level >= 1 {
                    level -= 1;

                    // We need to get the current level's node
                    let mut curr = pred.get_unchecked(level).load(Ordering::Relaxed);

                    // We want to continue moving right until we find a key greater than the search key
                    while !curr.is_null() {
                        //
                    }
                }

                t.predecessors[level] = pred;

                todo!()
            }
        }

        //

        //

        todo!()
    }

    // TODO: Need to implement operations for SkipList
    // - Search
    // - Insert
    // - Range?
    // - Random Height Generation
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

        // Validate the key_ptr being 24
        unsafe {
            // If we did Box::from(T) here, we would break the memory ownership - arena owns this memory, so we don't want to hand it to Box because
            // it would try to drop the memory and destruct it invalidating our arena
            // We need to either return a reference slice OR we copy out the bytes into a vec and then into box owned heap
            let s = std::slice::from_raw_parts(
                Node::key_ptr(node) as *const u8,
                (*node).key_len as usize,
            );
            assert_eq!(s, &[24u8; 1]);
            let b = s.to_vec().into_boxed_slice();

            // Now if we drop the box, our arena is still valid
            drop(b); // If we did Box::from(s) it may not immediately cause a panic BUT the premise is that we do not want to take ownership of arena memory
            let s2 = std::slice::from_raw_parts(
                Node::key_ptr(node) as *const u8,
                (*node).key_len as usize,
            );
            assert_eq!(s2, &[24u8; 1]);
        };

        let node2 = unsafe { Node::alloc(&arena, 1, 1, 0).unwrap() };
        unsafe {
            ptr::write(Node::key_ptr(node2), 89);
        }
        println!("arena new = {:?}", arena.get_current_init_slice());
    }

    #[test]
    fn level_access() {
        let arena = Arena::new(
            ArenaSize::Test(80, 160),
            Allocator::System(SystemAllocator::new()),
        );
        let skip = SkipList::new(Arc::new(DefaultComparator {}));

        // Let's get the base level
        let base = unsafe { skip.head.pointers.get_unchecked(0) };
        assert!(
            base.load(Ordering::Relaxed).is_null(),
            "base level should be null"
        );

        // Allocate a node at base level
        let node = unsafe { Node::alloc(&arena, 1, 5, 2).unwrap() };

        unsafe {
            ptr::copy(
                "hello".as_bytes().as_ptr(),
                Node::key_ptr(node),
                (*node).key_len as usize,
            );
        }

        // Write node into the skip list at the base level
        let _ = base
            .compare_exchange(
                base.load(Ordering::Relaxed),
                node,
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .unwrap();

        // Verify the node was written at base level

        // Check if key is in arena and then check if we can fetch the key from the node
        let key_ptr = unsafe { Node::key_ptr(node) };
        let key = unsafe {
            String::from_utf8_lossy(std::slice::from_raw_parts(
                key_ptr,
                (*node).key_len as usize,
            ))
        };

        assert_eq!(key, "hello");
    }

    #[test]
    fn node_tower_height() {
        let arena = Arena::new(
            ArenaSize::Test(80, 160),
            Allocator::System(SystemAllocator::new()),
        );
        let node = unsafe { Node::alloc(&arena, 1, 5, 2).unwrap() };

        unsafe {
            assert_eq!(Node::tower_height(node), 1);
        }

        let node2 = unsafe { Node::alloc(&arena, 3, 5, 2).unwrap() };
        unsafe {
            assert_eq!(Node::tower_height(node2), 3);
        }
    }
}
