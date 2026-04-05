use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicPtr, AtomicUsize};
//
//

pub(crate) struct GlobalEBR {
    list: ThreadList,
    pub(crate) epoch: AtomicUsize,
}

struct ThreadList {
    head: AtomicPtr<EBRThread>,
}

pub(crate) struct EBRThread {
    test_value: i32,
    gc_cache: [MaybeUninit<()>; 0],
    // Need:
    // Local epoch count
    // GC Cache
    // Total Pinned for threshold collection
    // Reference to the global data (Collector?)
    // Number of guards keeping this thread pined
    // Number of active handles?
}
