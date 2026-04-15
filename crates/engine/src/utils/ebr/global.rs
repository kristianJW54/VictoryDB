//
//
//
//
//
//
//
use crate::utils::ebr::local::{Local, LocalHandle};

use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

// Global epoch advancement:
//
// - The global epoch may advance from E → E+1 if NO thread is pinned with
//   local_epoch < E.
// - Pinned threads are allowed to lag behind the global epoch.
// - A thread observes newer epochs only by UNPINNING and PINNING again.
//
// So all threads must have observed the current global epoch.
//

pub(crate) struct Global {
    // NOTE: ThreadList -> Mutex<Vec<Thread>>? Would prefer lock-free but
    // Unless benchmarking shows a significant performance gain, prefer simplicity with a lock as only superversion will be primary
    // user of this at the moment
    //
    // NOTE: Can we make this Lcok-free linked list if we don't use reclamation on the participants?
    pub(super) participants: Mutex<Vec<*const Local>>,
    pub(super) epoch: AtomicU64,
    defer: Mutex<Vec<()>>, // Global deferred functions (will be a pointer to destruct)
}

impl Global {
    pub(crate) fn new() -> Self {
        Self {
            participants: Mutex::new(Vec::new()),
            epoch: AtomicU64::new(0),
            defer: Mutex::new(Vec::new()),
        }
    }
}

// A default static collector instance

pub(crate) struct Collector {
    global: Arc<Global>,
}

unsafe impl Send for Collector {}
unsafe impl Sync for Collector {}

impl Collector {
    pub(crate) fn new() -> Self {
        Self {
            global: Arc::new(Global::new()),
        }
    }

    pub(crate) fn register(&self) -> LocalHandle {
        LocalHandle::new(self.global.clone())
    }
}

impl Clone for Collector {
    fn clone(&self) -> Self {
        Self {
            global: self.global.clone(),
        }
    }
}

pub(crate) fn tls_collector() -> &'static Collector {
    static COLLECTOR: std::sync::OnceLock<Collector> = std::sync::OnceLock::new();
    COLLECTOR.get_or_init(|| Collector::new())
}
