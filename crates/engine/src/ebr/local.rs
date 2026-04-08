//
//
//
//
//
//
//
use crate::ebr::global::Collector;
use crate::ebr::guard::EpochGuard;

use std::ops::Deref;
use std::{cell::Cell, num::Wrapping, sync::atomic::AtomicU64};

pub(crate) struct ParticipantEpochPtr(*const CachePadded<AtomicU64>);

unsafe impl Send for ParticipantEpochPtr {}

pub(super) struct Local {
    //
    guard_count: Cell<usize>,
    pin_count: Cell<Wrapping<usize>>,
    epoch: CachePadded<AtomicU64>,
    collector: *const Collector,
    // NOTE: If we want to support multi DB Instances, then we need to change how collector is stored
    // because we would not be using TLS on multi instances and would need to use explicit handles
    // and collector's lifetime would have to be managed
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
                collector,
            })),
        }
    }

    pub(super) fn pin(&self) -> EpochGuard {
        let guard = EpochGuard {
            local: self as *const Local,
        };

        // Pinning logic

        guard
    }
}

#[derive(Clone)]
pub(crate) struct LocalHandle {
    local: *const Local,
}

// SAFETY: LocalHandle::pin must only be called by the thread that owns this Local.
// Other threads may observe the atomic epoch field indirectly through collector scans,
// but must not mutate Local's Cell-based fields.
unsafe impl Send for LocalHandle {}
unsafe impl Sync for LocalHandle {}

impl LocalHandle {
    pub(crate) fn new(collector: &Collector) -> Self {
        Local::register(collector)
    }

    pub(super) fn pin(&self) -> EpochGuard {
        unsafe { (*self.local).pin() }
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
