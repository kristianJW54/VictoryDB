use crate::storage::key::internal_key::LookupKey;

// Internal Iterator is the trait for which all internal iterators must implement.
//
pub(crate) trait InternalIterator {
    // Static methods

    fn Seek_to_first(&mut self);
    fn Seek(&mut self, key: LookupKey);

    // Relative methods
}
