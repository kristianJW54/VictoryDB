use std::sync::atomic::AtomicU64;

pub(crate) mod guard;

pub(super) struct GlobalEpoch {
    // NOTE: ThreadList -> RwLock<Vec<Thread>>? Would prefer lock-free but
    // Unless benchmarking shows a significant performance gain, prefer simplicity with a lock as only superversion will be primary
    // user of this at the moment
    pub(super) epoch: AtomicU64,
}
