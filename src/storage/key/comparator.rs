//
use std::cmp::Ordering;
use std::sync::Arc;

pub trait Comparator: Send + Sync {
    fn compare(&self, a: &[u8], b: &[u8]) -> Ordering;
    // TODO: Add separator and successor and other signatures we may need
}

pub(crate) struct DefaultComparator {}

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

pub(crate) struct InternalKeyComparator {}

pub(crate) type InternalKeyComparatorArc = Arc<InternalKeyComparator>;

impl InternalKeyComparator {
    pub(crate) fn new() -> InternalKeyComparatorArc {
        Arc::new(InternalKeyComparator {})
    }
}

impl Comparator for InternalKeyComparator {
    fn compare(&self, a: &[u8], b: &[u8]) -> Ordering {
        let (user_key, trailer) = a.split_at(&a.len() - 8);
        let (b_user_key, b_trailer) = b.split_at(&b.len() - 8);
        match user_key.cmp(b_user_key) {
            Ordering::Equal => trailer.cmp(b_trailer),
            other => other,
        }
    }
}
