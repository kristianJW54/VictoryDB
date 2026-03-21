pub mod comparator;
pub mod internal_key;

// TODO: Handling User key allocation
// NOTE: On the write path we simply encode the internal key and write directly into memtable arena
// For read path, we need a temporary buffer to hold the internal key
// We can try stack approach for small keys, heap for large keys
