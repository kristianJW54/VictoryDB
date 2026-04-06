//
//
//
//
//
//
//
use crate::ebr::local::{Local, LocalHandle};

use std::sync::Mutex;
use std::sync::atomic::AtomicU64;

// Global will be a static global epoch instance per db instance - we want to intialise one and only once

pub(super) struct GlobalEpoch {
    // NOTE: ThreadList -> Mutex<Vec<Thread>>? Would prefer lock-free but
    // Unless benchmarking shows a significant performance gain, prefer simplicity with a lock as only superversion will be primary
    // user of this at the moment
    pub(super) participants: Mutex<Vec<LocalHandle>>,
    pub(super) epoch: AtomicU64,
    defer: Mutex<Vec<()>>, // Global deferred functions (will be a pointer to destruct)
}

// Wrap the Global Epoch in a newtype and intialise it once
pub(crate) struct Collector(GlobalEpoch);

pub(crate) fn collector() -> &'static Collector {
    static COLLECTOR: std::sync::OnceLock<Collector> = std::sync::OnceLock::new();
    COLLECTOR.get_or_init(|| {
        Collector(GlobalEpoch {
            participants: Mutex::new(Vec::new()),
            epoch: AtomicU64::new(0),
            defer: Mutex::new(Vec::new()),
        })
    })
}

impl Collector {
    pub(crate) fn register(&self) -> LocalHandle {
        Local::register(self)
    }
}
