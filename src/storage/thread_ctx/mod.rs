pub(crate) mod registry;
pub(crate) mod scratch;

use crate::storage::ebr::global::collector;
use crate::storage::ebr::local::LocalHandle;

// NOTE: We need to think about if we want a thread_context structure for storing thread-local data not just the epoch
// so hot-path metrics etc

/*

Example:

struct ThreadContext {
    ebr: LocalHandle,
    metrics: Metrics,
    sv: *const SuperVersion, //NOTE: Cached pointer with tagged generated number
    //...
}

 */

// TODO: Create thread_ctx and use it to store Local ebr participant and also cached super version pointer
// We can also use internal key scratch buffers for building lookup keys and internal keys for memtable insertions

thread_local! {
    static LOCAL: LocalHandle = collector().register()
}
