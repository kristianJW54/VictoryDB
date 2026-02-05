use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicPtr, AtomicUsize};
//
//

pub(crate) struct GlobalEBR {
    list: ThreadList,
    pub(crate) epoch: AtomicUsize,
}

// TODO: Need to make an intrusive linked list of EBRThread

struct ThreadList {
    head: AtomicPtr<EBRThread>, // TODO: To replace with custom Atomic structure
}

pub(crate) struct EBRThread {
    test_value: i32,
    gc_cache: [MaybeUninit<()>; 0],
    // Need:
    // Local epoch count
    // GC Cache
    // Total Pinned for threshold collection
    // Reference to the global data (Collector?) // TODO: Need to understand this more
    // Number of guards keeping this thread pined
    // Number of active handles? // TODO: Need to understand this more
}
