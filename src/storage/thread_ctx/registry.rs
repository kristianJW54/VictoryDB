//
//
//
//
//

use crate::storage::ebr::local::LocalHandle;
use crate::storage::key::internal_key::InternalKeyBuffer;

pub(crate) struct ThreadCtx {
    ebr: LocalHandle,
    // NOTE: Add super version cache
    key_buffer: InternalKeyBuffer,
}

// TODO: Figure out where we want to house init OnceLock functions + what fields `should be initialized once
