use std::cell::Cell;
use std::thread;

// Want to make a fake reader with a fake structure which writers must change essentially a Mutex into a ZST
// Benchmark many readers hitting pinning and unpinning and some writers changing the "value" meaning using the Mutex
// Need to measure contention and lock acquisition times

thread_local! {
    static TEST: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]

    fn thread_local_test() {}
}
