//
//
//
//
//
//
//

use crate::ebr::global::Collector;

use std::ops::Deref;
use std::{cell::Cell, num::Wrapping, sync::atomic::AtomicU64};

pub(super) struct Local {
    //
    guard_count: Cell<usize>,
    pin_count: Cell<Wrapping<usize>>,
    epoch: CachePadded<AtomicU64>, // TODO: Do we need this? If we are not using local defer function caches?
                                   //
                                   // defer: Vec<()>, //NOTE: If we measure contention at the global level with deferred function storing
                                   // then we can add local deferred functions caching and flushing to global
}

impl Local {
    pub(super) fn register(collector: &Collector) -> LocalHandle {
        LocalHandle {
            local: Box::into_raw(Box::new(Local {
                guard_count: Cell::new(0),
                pin_count: Cell::new(Wrapping(0)),
                epoch: CachePadded {
                    value: AtomicU64::new(0),
                },
            })),
        }
    }
}

// We drive everything through the LocalHandle so we can safely share references across threads and control access to Local fields
#[derive(Clone)]
pub(crate) struct LocalHandle {
    local: *const Local,
}

unsafe impl Send for LocalHandle {}
unsafe impl Sync for LocalHandle {}

impl LocalHandle {
    pub(crate) fn new(collector: &Collector) -> Self {
        Self {
            local: Local::register(collector).local,
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[repr(align(64))]
struct CachePadded<T> {
    value: T,
}

unsafe impl<T> Send for CachePadded<T> {}
unsafe impl<T> Sync for CachePadded<T> {}

impl<T> Deref for CachePadded<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
