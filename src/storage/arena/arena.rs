// Arena runs the management of it's memory allocation given to it by the allocator
//
//

// The Arena will be used like a simple bump allocator for it's memory region
//
// For reference I used:
// https://fitzgen.com/2019/11/01/always-bump-downwards.html
// https://www.williamfedele.com/blog/arena-allocators
//
// This describes a good rust approach to alignment rounding and a recommendation of bumping downward as an optimisation
// RocksDB also uses an arena allocator but allocates from either end - front being aligned and back being unaligned
// For now, we'll use a simple bump allocator that allocates from the front and carefully control alignment of the skip nodes and byte data - if we notice improvements to be made then we can change
// Arena implementation to use a more efficient allocator
//
// Because we'll be allocating T (such as skiplist Nodes) and bytes (already aligned) we need to makes sure that what we write to in the heap is aligned
//

use std::sync::atomic::{AtomicPtr, AtomicUsize};

#[derive(Debug)]
pub(crate) struct Chunk {
    chunk: Box<[u8]>,
    bump: AtomicUsize,
}

#[derive(Debug)]
pub(crate) struct Arena {
    current_chunk: AtomicPtr<Chunk>,
    chunks: Vec<Chunk>,
    allocation: usize,
    used: AtomicUsize,
}
