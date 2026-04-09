//
//
//
//
//
//
//
use crate::utils::ebr::global::{Collector, Global};
use crate::utils::ebr::guard::EpochGuard;

use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::{cell::Cell, num::Wrapping, sync::atomic::AtomicU64};

pub(super) struct ParticipantEpochPtr(*const CachePadded<AtomicU64>);

unsafe impl Send for ParticipantEpochPtr {}

pub(super) struct Local {
    //
    guard_count: Cell<usize>,
    pin_count: Cell<Wrapping<usize>>,
    epoch: CachePadded<AtomicU64>,
    domain: Arc<Global>,
    // NOTE: If we want to support multi DB Instances, then we need to change the domain to collector
    // because we would not be using TLS on multi instances and would need to use explicit handles
    // and collector's lifetime would have to be managed
    //
    // defer: Vec<()>, //NOTE: If we measure contention at the global level with deferred function storing
    // then we can add local deferred functions caching and flushing to global
}

impl Local {
    pub(super) fn register(domain: Arc<Global>) -> LocalHandle {
        //
        // Build local
        let local = Box::new(Local {
            guard_count: Cell::new(0),
            pin_count: Cell::new(Wrapping(0)),
            epoch: CachePadded {
                value: AtomicU64::new(0),
            },
            domain: domain.clone(),
        });

        // Get the raw pointer to store in the handle
        let ptr = Box::into_raw(local);

        // Register the local's epoch pointer with the global participants list
        domain
            .participants
            .lock()
            .unwrap()
            .push(ParticipantEpochPtr(unsafe { &(*ptr).epoch }));

        // Return the handle with the raw pointer
        LocalHandle { local: ptr }
    }

    pub(super) fn pin(&self) -> EpochGuard {
        let guard = EpochGuard {
            local: self as *const Local,
        };

        // Pinning logic
        //
        // First we must take the global epoch value and store it
        //

        let global_epoch = self.domain.epoch.load(Ordering::Relaxed);

        self.epoch.value.store(global_epoch, Ordering::Release);

        self.domain
            .epoch
            .compare_exchange(
                global_epoch,
                global_epoch + 1,
                Ordering::Release,
                Ordering::Relaxed,
            )
            .unwrap_or(global_epoch);

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
    pub(crate) fn new(domain: Arc<Global>) -> Self {
        Local::register(domain)
    }

    pub(super) fn pin(&self) -> EpochGuard {
        unsafe { (*self.local).pin() }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[repr(align(64))]
pub(super) struct CachePadded<T> {
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

#[cfg(test)]
mod tests {
    use crate::{thread_ctx::thread_ctx, utils::ebr::pin};

    use super::*;

    #[test]
    fn simple_pin() {
        let guard = pin();

        // Print the global epoch
        // Print the local epoch
        let local = unsafe { &*(guard.local) };

        // Print local through TLS
        thread_ctx(|ctx| {
            let local = unsafe { &*(ctx.ebr_handle().local) };
            println!(
                "local epoch (TLS): {}",
                local.epoch.value.load(Ordering::Relaxed)
            );
        });

        println!(
            "global epoch: {}",
            local.domain.epoch.load(Ordering::Relaxed)
        );

        println!("local epoch: {}", local.epoch.value.load(Ordering::Relaxed));
    }
}
