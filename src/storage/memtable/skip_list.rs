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

use std::sync::atomic::{AtomicPtr, AtomicUsize};

use crate::storage::memory::arena::Arena;

// ------------------------------------------------------

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

    tower: [AtomicPtr<Node>; 0],
}

impl Node {}

// TODO: Need to implement a layout method
// TODO: Need to implement an alloc method
// TODO: Need to understand FAM Tower and Key/Value bytes

#[repr(C)]
struct Header {
    pointers: [AtomicPtr<Node>; HEAD_HEIGHT],
}
