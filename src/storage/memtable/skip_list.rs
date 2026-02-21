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

use std::ptr;
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

pub(crate) struct SkipList {
    // Reference to the arena will be needed
    // Fields
}

impl Default for SkipList {
    fn default() -> Self {
        SkipList {
            // Fields

        }
    }
}

#[repr(C)]
pub(crate) struct Node {
    key_len: u32, // We lose nothing by making it a u32 because AtomicUsize is 8 bytes and will force padding
    value_len: u32,
    //
    // NOTE: Crossbeam uses refs as well here - but I think this is because it needs reclamation through EBR but since
    // We're using Arena, refs on the node are not needed
    //
    // Number of levels of this node
    height: AtomicUsize, // TODO: Can we make this smaller since we aren't tracking refs?

    pub(crate) tower: [AtomicPtr<Node>; 0],
}

impl Node {
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
            .0
            .pad_to_align();

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

        // Layout::new::<Self>()
        //     .extend(Layout::array::<AtomicPtr<Self>>(height).unwrap())
        //     .unwrap()
        //     .0
        //     .pad_to_align()
    }
}

// TODO: Need to implement a layout method
// TODO: Need to implement an alloc method
// TODO: Need to understand FAM Tower and Key/Value bytes

#[repr(C)]
struct Header {
    pointers: [AtomicPtr<Node>; HEAD_HEIGHT],
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
                            height: AtomicUsize::new(1),
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
