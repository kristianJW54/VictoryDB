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

use std::sync::atomic::AtomicPtr;

// ------------------------------------------------------

pub(crate) struct Node {
    header: Header,
    tower: [AtomicPtr<Node>; 0],
}

// ReprC ?
struct Header {
    // Fields
}

pub(crate) struct SkipList {
    // Fields
}

impl Default for SkipList {
    fn default() -> Self {
        SkipList {
            // Fields
        }
    }
}
