//
//
//
//
//
//
use crate::utils::ebr::local::Local;

pub(crate) struct EpochGuard {
    pub(super) local: *const Local,
}

// We do stuff with the guard under a pin (defer_destroy etc)
//

impl EpochGuard {
    //
    pub(super) fn local(&self) -> &Local {
        unsafe { &*self.local }
    }
}
