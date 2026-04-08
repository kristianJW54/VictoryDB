//
//
//
//
//
//
//
use crate::ebr::local::{Local, LocalHandle, ParticipantEpochPtr};

use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

pub(crate) struct Global {
    // NOTE: ThreadList -> Mutex<Vec<Thread>>? Would prefer lock-free but
    // Unless benchmarking shows a significant performance gain, prefer simplicity with a lock as only superversion will be primary
    // user of this at the moment
    pub(super) participants: Mutex<Vec<ParticipantEpochPtr>>,
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
        Local::register(self)
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
