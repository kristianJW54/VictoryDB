//
//
//
//
//

use crate::ebr::global::tls_collector;
use crate::ebr::local::LocalHandle;
use crate::key::internal_key::Ephemeral_Buffer;
use crate::versioning::superversion::SVCache;

pub(crate) struct ThreadCtx {
    ebr: LocalHandle,
    sv_cache: SVCache,
    key_buffer: Ephemeral_Buffer,
    // NOTE: Add PerfContext/Metrics
    // NOTE: Add IOContext/Metrics
}

impl ThreadCtx {
    pub(crate) fn new() -> Self {
        Self {
            ebr: tls_collector().register(),
            sv_cache: SVCache::new(),
            key_buffer: Ephemeral_Buffer::new(),
        }
    }

    pub(crate) fn ebr_handle(&self) -> &LocalHandle {
        &self.ebr
    }

    pub(crate) fn inner_key_buf(&self) -> &Ephemeral_Buffer {
        &self.key_buffer
    }
}
