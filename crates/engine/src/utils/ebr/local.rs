//
//
//
//
//
//
//
use crate::utils::ebr::global::{Collector, Global};
use crate::utils::ebr::guard::EpochGuard;

use std::f64::consts::PI;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic;
use std::sync::atomic::Ordering;
use std::{cell::Cell, num::Wrapping, sync::atomic::AtomicU64};

pub(super) struct Local {
    //
    handle_count: Cell<usize>,
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
    //
    // An interval we can use with % to determine on pin count increment if we should collect
    const PIN_COLLECT: usize = 64;

    pub(super) fn register(domain: Arc<Global>) -> LocalHandle {
        //
        // Build local
        let local = Box::new(Local {
            handle_count: Cell::new(1),
            guard_count: Cell::new(0),
            pin_count: Cell::new(Wrapping(0)),
            epoch: CachePadded {
                value: AtomicU64::new(0),
            },
            domain: domain.clone(),
        });

        let ptr = Box::into_raw(local);

        // Register the local's epoch pointer with the global participants list
        domain.participants.lock().unwrap().push(ptr);

        // Return the handle with the raw pointer
        LocalHandle { local: ptr }
    }

    #[inline]
    pub(super) fn global(&self) -> &Global {
        &self.domain
    }

    #[inline]
    pub(super) fn is_pinned(&self) -> bool {
        self.guard_count.get() > 0
    }

    #[inline]
    pub(super) fn pin(&self) -> EpochGuard {
        let guard = EpochGuard {
            local: self as *const Local,
        };

        // Pinning logic
        //

        // First we must increment the guard count which will tell us if we are the first guard then
        // we can safely load the global epoch value and store it in our local epoch
        let guard_count = self.guard_count.get();
        //
        self.guard_count.set(guard_count.checked_add(1).unwrap());

        if guard_count == 0 {
            // NOTE: Check out crossbeams wild compiler optimization's here:
            // https://github.com/crossbeam-rs/crossbeam/blob/master/crossbeam-epoch/src/internal.rs#L409
            // I tried my hardest to understand this, but my simple use-case just doesn't warrant it
            // (I tell myself this rather than admit that I feel stupid)
            //
            // Alas, my basic version
            let global_epoch = self.domain.epoch.load(Ordering::Acquire);
            self.epoch.value.store(global_epoch, Ordering::Release);
            atomic::fence(Ordering::SeqCst);
        }

        // Increment the pin count and check if we should collect
        let pin_count = self.pin_count.get();
        self.pin_count.set(pin_count + Wrapping(1));

        if pin_count.0 % Self::PIN_COLLECT == 0 {
            // Call global.collect()
        }

        guard
    }

    #[inline]
    pub(super) fn unpin(&self) {
        let guard_count = self.guard_count.get();
        self.guard_count.set(guard_count - 1);

        // If we were the last guard, we should reset our Local Epoch
        if guard_count == 1 {
            self.epoch.value.store(0, Ordering::Release);
            // Also if we are the last LocalHandle about to be dropped then we should handle clean up of our Local
            if self.handle_count.get() == 0 {
                // TODO: Handle cleanup
            }
        }
    }

    #[inline]
    pub(super) fn acquire_handle(&self) {
        let handle_count = self.handle_count.get();
        debug_assert!(handle_count >= 1);
        self.handle_count.set(handle_count + 1);
    }

    #[inline]
    pub(super) fn release_handle(this: *const Self) {
        let guard_count = unsafe { (*this).guard_count.get() };
        let handle_count = unsafe { (*this).handle_count.get() };

        debug_assert!(handle_count >= 1);

        unsafe {
            if guard_count == 0 {
                (*this).handle_count.set(handle_count - 1);
            }
        }

        if guard_count == 0 && handle_count == 1 {
            unsafe {
                todo!()
                // TODO: Self::finalise()
            }
        }
    }

    #[cold]
    pub(super) fn finalise(this: *const Self) {
        // We need to check that we hold no guards or handles
        unsafe {
            debug_assert!((*this).guard_count.get() == 0);
            debug_assert!((*this).handle_count.get() == 0);
        }

        let global = unsafe { (*this).global() };

        unsafe {
            // TODO: Think of a better way to do this
            global.participants.lock().unwrap().retain(|p| *p != this);
        }
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

    pub(super) fn local(&self) -> &Local {
        unsafe { &*self.local }
    }
}

impl Drop for LocalHandle {
    fn drop(&mut self) {
        unsafe {
            Local::release_handle(&*self.local);
        }
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
        let local = guard.local();

        // Print local through TLS
        thread_ctx(|ctx| {
            let local = ctx.ebr_handle().local();
            println!(
                "local epoch (TLS): {}",
                local.epoch.value.load(Ordering::Relaxed)
            );
        });

        println!(
            "global epoch: {}",
            local.domain.epoch.load(Ordering::Relaxed)
        );

        println!("local guard count: {}", local.guard_count.get());
        println!("local epoch: {}", local.epoch.value.load(Ordering::Relaxed));
    }
}
