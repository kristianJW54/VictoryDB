#![allow(dead_code)]

pub(crate) mod allocator;
pub(crate) mod arena;

// Constants

// Sealed Marker types for Arena policies
pub(crate) trait ArenaPolicy {
    const CHUNK_SIZE: usize;
    const MAX_ARENA_SIZE: usize;
}
