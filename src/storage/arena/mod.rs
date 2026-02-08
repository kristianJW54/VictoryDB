#![allow(dead_code)]

pub(crate) mod allocator;
mod arena;

//
//
//
//
//
//
//

pub const KB: isize = 1024;
pub const MB: isize = 1024 * 1024;

const DEFAULT_ARENA_SIZE: isize = 64 * MB;
const SMALL_ARENA_SIZE: isize = 16 * MB;

pub(crate) enum ArenaSize {
    // Default is 64mb
    Small = SMALL_ARENA_SIZE,
    Default = DEFAULT_ARENA_SIZE,
}
