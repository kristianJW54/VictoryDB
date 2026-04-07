//
//
//
//
//

use crate::ebr::global::collector;
use crate::ebr::local::LocalHandle;
use crate::key::internal_key::Ephemeral_Buffer;

pub(crate) struct ThreadCtx {
    ebr: LocalHandle,
    // NOTE: Add super version cache
    key_buffer: Ephemeral_Buffer,
    // NOTE: Add PerfContext/Metrics
    // NOTE: Add IOContext/Metrics
}

impl ThreadCtx {
    pub(crate) fn new() -> Self {
        Self {
            ebr: collector().register(),
            key_buffer: Ephemeral_Buffer::new(),
        }
    }

    pub(crate) fn inner_key_buf(&self) -> &Ephemeral_Buffer {
        &self.key_buffer
    }
}
