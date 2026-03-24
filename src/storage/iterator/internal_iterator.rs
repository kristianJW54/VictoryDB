use crate::storage::key::internal_key::LookupKey;

// Internal Iterator is the trait for which all internal iterators must implement.
//
pub(crate) trait InternalIterator {
    // Static methods

    fn seek_to_first(&mut self);
    // fn seek(&mut self, key: LookupKey);
    // fn valid(&self) -> bool;

    // Relative methods
    // fn next(&mut self) -> Option<&[u8]>;
    // fn key(&self) -> Option<&[u8]>;
    // fn value(&self) -> Option<&[u8]>;
}
