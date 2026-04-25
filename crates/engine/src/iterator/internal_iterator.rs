// Internal Iterator is the trait for which all internal iterators must implement.
//
pub(crate) trait InternalIterator {
    fn seek_to_first(&mut self);
    fn seek(&mut self, key: &[u8]);
    fn valid(&self) -> bool;

    // Relative methods
    fn next(&mut self);
    fn key(&self) -> &[u8];
    fn value(&self) -> &[u8];
}
