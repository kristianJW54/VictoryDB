//
use std::cmp::Ordering;

pub trait Comparator: Send + Sync {
    fn compare(&self, a: &[u8], b: &[u8]) -> Ordering;
}

pub struct DefaultComparator {}

impl Comparator for DefaultComparator {
    fn compare(&self, a: &[u8], b: &[u8]) -> Ordering {
        a.cmp(b)
    }
}
