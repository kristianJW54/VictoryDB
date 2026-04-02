pub mod comparator;
pub mod internal_key;

// TODO: Handling User key allocation
// NOTE: On the write path we simply encode the internal key and write directly into memtable arena
// For read path, we need a temporary buffer to hold the internal key
// We can try stack approach for small keys, heap for large keys

pub(crate) const MAX_KEY_SIZE: usize = u16::MAX as usize;

pub(crate) const INITIAL_KEY_BUFFER_CAP: usize = 64;
pub(crate) const SMALL_KEY_THRESHOLD: usize = 128;
pub(crate) const MEDIUM_KEY_THRESHOLD: usize = 1024;
pub(crate) const MAX_BUFFER_RETAINED: usize = 4096;
