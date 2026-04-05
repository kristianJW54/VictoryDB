use std::sync::Mutex;

// Re-usable heap allocation for iterators
//
// To begin with we will have a simple Vec-based pool. Each DBIter will borrow from this pool.
// As a future optimization, we will implement a thread-local iter pool as a cache of reusable iter allocs.
//
// IterAllocPool will live in DBImpl
pub(crate) struct IterAllocPool {
    iters: Mutex<Vec<IterAlloc>>,
}

// IterAlloc is a re-usable heap allocation for an iterator. Lives in DBIter
pub(crate) struct IterAlloc {
    // TOOD: Create fields for the vecs we'll need
}
