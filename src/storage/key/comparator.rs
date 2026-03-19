//
use std::cmp::Ordering;
use std::sync::Arc;

pub trait Comparator: Send + Sync {
    fn compare(&self, a: &[u8], b: &[u8]) -> Ordering;
    // TODO: Add separator and successor and other signatures we may need
}

pub struct DefaultComparator {}

pub(crate) type DefaultComparatorArc = Arc<DefaultComparator>;

impl DefaultComparator {
    pub(crate) fn new() -> DefaultComparatorArc {
        Arc::new(DefaultComparator {})
    }
}

impl Comparator for DefaultComparator {
    fn compare(&self, a: &[u8], b: &[u8]) -> Ordering {
        a.cmp(b)
    }
}
