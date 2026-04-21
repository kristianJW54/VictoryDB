//
//
//
//
//

use crate::key::internal_key::Ephemeral_Buffer;
use crate::thread_ctx::TCTX;
use crate::versioning::superversion::SVCache;
use mem::hazard::domain::Global;
use mem::hazard::hazard_ptr::HzdPtr;
use std::sync::atomic::AtomicPtr;

pub(crate) struct ThreadCtx {
    sv_cache: SVCache,
    key_buffer: Ephemeral_Buffer,
    // NOTE: Add PerfContext/Metrics
    // NOTE: Add IOContext/Metrics
}

impl ThreadCtx {
    pub(crate) fn new() -> Self {
        Self {
            sv_cache: SVCache::new(),
            key_buffer: Ephemeral_Buffer::new(),
        }
    }

    pub(crate) fn inner_key_buf(&self) -> &Ephemeral_Buffer {
        &self.key_buffer
    }
}

#[test]

fn hzd_ptr() {
    TCTX.with_borrow_mut(|ctx| {

        // Get the sv_cache
        // Access the generation number to check freshness
        // If fresh:
        // take sv pointer and protect() -- cheap because it should still be the same in the holder
        // Else:
        // get the global Atomic sv and store ptr and protect
    })
}
