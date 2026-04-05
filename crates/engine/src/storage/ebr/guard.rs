//
//
//
//
//
//

use crate::storage::ebr::local::Local;

pub(crate) struct EpochGuard {
    local: *const Local,
}
