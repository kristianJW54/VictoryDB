//
//
//
//
//

use crate::storage::ebr::global::collector;
use crate::storage::ebr::local::LocalHandle;
use crate::storage::key::internal_key::InternalKeyBuffer;

pub(crate) struct ThreadCtx {
    ebr: LocalHandle,
    // NOTE: Add super version cache
    key_buffer: InternalKeyBuffer,
}

impl ThreadCtx {
    pub(crate) fn new() -> Self {
        Self {
            ebr: collector().register(),
            key_buffer: InternalKeyBuffer::new(),
        }
    }

    pub(crate) fn inner_key_buf(&self) -> &InternalKeyBuffer {
        &self.key_buffer
    }
}
