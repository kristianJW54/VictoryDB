pub(crate) mod range_del;
pub(crate) mod span;

use crate::storage::range::range_del::RangeDeleteOp;

pub(crate) enum RangeOp<'a> {
    Delete(RangeDeleteOp<'a>),
    // TODO: Add RangeKeyOps (SET) ?
    // ...
}
