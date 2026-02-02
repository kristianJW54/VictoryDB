use std::mem::MaybeUninit;
use std::sync::atomic::AtomicPtr;
//
//
//
// At it's core, EBR is uses a global structure to manage epochs and references to objects.
// Local participants (threads) hold references to objects and have a local cache of unlinked objects
//
// Global holds a intrusive linked list of all threads that are currently active

pub(crate) struct GlobalEBR {
    list: IntrusiveList<EBRThread>,
}

// TODO: Need to make an intrusive linked list of EBRThread

trait ThreadEntry {}

struct IntrusiveList<E: ThreadEntry> {
    head: AtomicPtr<E>, // TODO: To replace with custom Atomic structure
}

pub(crate) struct EBRThread {
    test_value: i32,
    gc_cache: [MaybeUninit<()>; 0],
    // Need:
    // Local epoch count
    // GC Cache
    // Total Pinned for threshold collection
    // Reference to the global data (Collector?) // TODO: Need to understand this more
    // Number of guards keeping this thread pineed
    // Number of active handles? // TODO: Need to understand this more
}

impl ThreadEntry for EBRThread {}
