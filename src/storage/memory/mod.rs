#![allow(dead_code)]

pub(crate) mod allocator;
pub(crate) mod arena;

// Constants
const KB: usize = 1000;
const MB: usize = KB;

const TEST_ARENA_CAP: usize = 20;
const DEFAULT_ARENA_CAP: usize = 64 * MB;
const SMALL_ARENA_CAP: usize = 16 * MB;
const MEDIUM_ARENA_CAP: usize = 32 * MB;
const MAX_ARENA_BLOCK_SIZE: usize = 128 * MB;

// Block sizes
const TEST_ARENA_BLOCK_SIZE: usize = 10;
const DEFAULT_ARENA_BLOCK_SIZE: usize = 2 * MB;
const LARGE_ARENA_BLOCK_SIZE: usize = 8 * MB;
const MEDIUM_ARENA_BLOCK_SIZE: usize = 4 * MB;
const SMALL_ARENA_BLOCK_SIZE: usize = 1 * MB;

pub(crate) enum ArenaSize {
    Test(usize, usize),
    Default,
    Small,
    Medium,
    Large,
}

impl ArenaSize {
    pub fn to_policy(self) -> ArenaPolicy {
        match self {
            ArenaSize::Test(block, cap) => ArenaPolicy {
                block_size: block,
                cap: cap,
            },
            ArenaSize::Default => ArenaPolicy {
                block_size: DEFAULT_ARENA_BLOCK_SIZE,
                cap: DEFAULT_ARENA_CAP,
            },
            ArenaSize::Small => ArenaPolicy {
                block_size: SMALL_ARENA_BLOCK_SIZE,
                cap: SMALL_ARENA_CAP,
            },
            ArenaSize::Medium => ArenaPolicy {
                block_size: MEDIUM_ARENA_BLOCK_SIZE,
                cap: MEDIUM_ARENA_CAP,
            },
            ArenaSize::Large => ArenaPolicy {
                block_size: LARGE_ARENA_BLOCK_SIZE,
                cap: DEFAULT_ARENA_CAP,
            },
        }
    }
}

//
#[derive(Debug)]
pub(crate) struct ArenaPolicy {
    pub(crate) block_size: usize,
    pub(crate) cap: usize,
}
