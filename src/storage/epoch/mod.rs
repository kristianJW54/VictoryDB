use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use std::{cell::Cell, num::Wrapping, sync::atomic::AtomicU64};

pub(crate) mod guard;
pub(crate) mod scratch;

/*

Thread Lifetime

|----------------------------------------------------------------------------------------------------|
| Pin Scope 1                          |          |                                                  |
| |-------------------> END            | Unpinned |                                                  |
|               Pin Scope 2            |  State   |                                                  |
|               |----------------> END |          |                                                  |
|                                      |  Reclaim | Pin Scope 3                                      |
|                                      |          | |----------------> END                           |
|----------------------------------------------------------------------------------------------------|

Only inside the scope of a guard can a thread hold shared pointers


Thread lifetime
──────────────────────────────────────────────────────────────>

       ┌──────────── pinned region ────────────┐
       │                                       │
[unpinned] ── pin() ──► [pinned] ── unpin() ──► [unpinned]
 epoch = 0              epoch = E               epoch = 0
                         (latched once)

Legend:
- epoch = 0        → thread is quiescent (not pinned)
- epoch = E        → thread is pinned and advertises epoch E
- nested pin()     → does NOT change epoch
- epoch only updates when transitioning unpinned → pinned

 */

thread_local! {
    static LOCAL: LocalHandle = collector().register()
}

// Global will be a static global epoch instance per db instance - we want to intialise one and only once

pub(super) struct GlobalEpoch {
    // NOTE: ThreadList -> Mutex<Vec<Thread>>? Would prefer lock-free but
    // Unless benchmarking shows a significant performance gain, prefer simplicity with a lock as only superversion will be primary
    // user of this at the moment
    pub(super) participants: Mutex<Vec<&'static Local>>,
    pub(super) epoch: AtomicU64,
    defer: Mutex<Vec<()>>, // Global deferred functions (will be a pointer to destruct)
}

// Wrap the Global Epoch in a newtype and intialise it once
pub(super) struct Collector(GlobalEpoch);

fn collector() -> &'static Collector {
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
    fn register(&self) -> LocalHandle {
        Local::register(self)
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

pub(super) struct Local {
    //
    guard_count: Cell<usize>,
    pin_count: Cell<Wrapping<usize>>,
    epoch: CachePadded<AtomicU64>,
    // defer: Vec<()>, //NOTE: If we measure contention at the global level with deferred function storing
    // then we can add local deferred functions caching and flushing to global
}

// SAFETY:
// Local is owned by a single thread.
// The fields guard_count, pin_count, and deferred are accessed only by
// that owning thread via TLS.
// Other threads may only observe atomic fields such as epoch.
// Therefore shared references to Local are safe.
unsafe impl Sync for Local {}

impl Local {
    fn register(collector: &Collector) -> LocalHandle {
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

pub(super) struct LocalHandle {
    local: *const Local,
}
