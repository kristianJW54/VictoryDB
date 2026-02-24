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

use std::ptr::{self, NonNull};
use std::sync::atomic::AtomicU16;
use std::{
    alloc::Layout,
    sync::atomic::{AtomicPtr, AtomicUsize},
};

use crate::storage::memory::arena::Arena;

// ------------------------------------------------------

#[derive(Debug)]
pub(crate) enum SkipListError {
    LayoutError(std::alloc::LayoutError),
}

impl From<std::alloc::LayoutError> for SkipListError {
    fn from(err: std::alloc::LayoutError) -> Self {
        SkipListError::LayoutError(err)
    }
}

// We introduce a max head height // NOTE: Later we may want this configurable
const HEAD_HEIGHT: usize = 8;

pub(super) struct SkipList {
    // Reference to the arena will be needed
    pub(super) arena: *const Arena,
    pub(super) head: Header,
    // Fields
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct Header {
    pointers: [AtomicPtr<Node>; HEAD_HEIGHT],
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

    #[inline(always)]
    unsafe fn set_key_len(node: *mut Node, key_len: u16) {
        unsafe {
            ptr::write(&raw mut (*node).key_len, key_len);
        }
    }

    #[inline(always)]
    unsafe fn set_value_len(node: *mut Node, value_len: u32) {
        unsafe {
            ptr::write(&raw mut (*node).value_len, value_len);
        }
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
            )
        }
    }

    // Pointers to get for the skiplist to handle
    //
    #[inline(always)]
    unsafe fn tower_ptr(node: *mut Node) -> *mut AtomicPtr<Node> {
        unsafe { (node as *mut u8).add(core::mem::offset_of!(Node, tower)) as *mut AtomicPtr<Node> }
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
}

// TODO: Need to implement a layout method
// TODO: Need to implement an alloc method
// TODO: Need to understand FAM Tower and Key/Value bytes

#[cfg(test)]
mod tests {
    use crate::storage::memory::{
        ArenaSize,
        allocator::{Allocator, SystemAllocator},
    };

    use super::*;

    #[test]
    fn basic_node_layout() {
        let node = Node::build_layout(2, 1, 0);
        let node2 = Node::build_layout(2, 1, 0);

        println!("Node layout: {:?}", node.as_ref().unwrap());

        // At the moment this will give me 33 layout size align 8 if i don't pad_to_align()

        let arena = Arena::new(
            ArenaSize::Test(80, 160),
            Allocator::System(SystemAllocator::new()),
        );

        unsafe {
            arena
                .alloc_raw(node.unwrap(), |ptr| {
                    // Cast to node

                    let n = ptr.as_ptr() as *mut Node;

                    ptr::write(
                        n,
                        Node {
                            key_len: 1,
                            value_len: 1,
                            height: 1,
                            tower: [AtomicPtr::new(ptr::null_mut()); 0],
                        },
                    );

                    // Write null pointers to the towers
                    let tower_off = core::mem::offset_of!(Node, tower);

                    let tower = (n as *mut u8).add(tower_off) as *mut AtomicPtr<Node>;

                    for i in 0..2usize {
                        ptr::write(tower.add(i), AtomicPtr::new(ptr::null_mut()));
                    }

                    let tower_bytes = 2 * core::mem::size_of::<AtomicPtr<Node>>();
                    let key_ptr = (n as *mut u8).add(tower_off + tower_bytes);

                    ptr::write(key_ptr, 24);

                    Ok(())
                })
                .unwrap();
        }

        println!("arena = {:?}", arena.get_current_init_slice());

        println!("memory = {:?}", arena.memory_used());

        // Now we should see if we correctly align up from the arena or if we need to pad to align

        unsafe {
            arena
                .alloc_raw(node2.unwrap(), |ptr| {
                    // simple u32 write
                    ptr.write(42);
                    Ok(())
                })
                .unwrap()
        };

        println!("arena = {:?}", arena.get_current_init_slice());
        println!("memory = {:?}", arena.memory_used());
    }
}
