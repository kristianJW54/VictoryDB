use super::write_batch::Batch;

// db.write(batch)
//     │
//     ├─ create Writer node on stack
//     ├─ join writer queue
//     │
//     ├─ if FOLLOWER
//     │      block on condvar
//     │      wake up when done
//     │      return
//     │
//     └─ if LEADER
//            form write group
//            assign sequence numbers
//            WAL write
//            apply group to memtables
//            signal followers
//            return
//
//
// Logic:
// db_impl  — orchestrates the whole flow on the calling thread
//    │
//    ├── write_thread — just coordination, am I leader or follower?
//    │                  if follower: block here until signalled
//    │                  if leader: return and let caller thread do the work
//    │
//    └── if leader: caller thread continues executing through db_impl
//                   accessing self directly for WAL, memtables, CFs
//
//
// Leader Cutoff
// The leader determines cutoff during batch formation based on compatibility and size limits,
// and a new leader starts either when newest_writer_ is set to null
// or when the next writer's state is explicitly set to STATE_GROUP_LEADER

use std::{ptr, sync::atomic::AtomicPtr};

use super::writer::Writer;

/// WriteThread is the coordination mechanism for multiple writes. Each calling thread will creater a writer holding a batch of operations and try to join
/// the write thread queue. The write thread will group multiple writes and determine leader/followers.
/// Once complete, it will signal to followers and drop
pub(crate) struct WriteThread {
    head: AtomicPtr<Writer>,
}

impl Default for WriteThread {
    fn default() -> Self {
        Self::new()
    }
}

impl WriteThread {
    // NOTE: Later move to config options on the write thread if we want this to be configurable

    // How many times do we want to asm!(PAUSE) on the fast path for Writer::wait()
    pub(crate) const WAIT_PAUSE_ITERATIONS: usize = 200;
    // How many time do we want to iterate and Thread::yield()
    pub(crate) const YIELD_PAUSE_ITERATIONS: usize = 64;

    pub(crate) fn new() -> Self {
        Self {
            head: AtomicPtr::new(ptr::null_mut()),
        }
    }

    pub(crate) fn join(&self, writer: &Writer) {
        // TODO: Need to understand how we cut the linked list and then reverse it to give to the leader from oldest to newest
        let _ = writer;
        todo!()
    }
}

//

#[cfg(test)]
mod tests {
    use crate::db::writer::{self, WriterState};

    use super::*;
    use std::sync::atomic::Ordering;
    use std::thread::{self};

    #[test]
    fn writer_follower_to_leader() {
        // XXX: Replace naive implementation with writer_thread methods - eventually move to integration test
        //
        let group: AtomicPtr<Writer> = AtomicPtr::new(ptr::null_mut());

        // Want:
        // leader -> follower 1 -> follower 2
        // To become:
        // follower 1 (new leader) -> follower 2

        thread::scope(|t| {
            // Leader
            t.spawn(|| {
                let batch = Batch::new();
                let mut writer_1 = Writer::new(&batch);

                assert!(group.load(Ordering::Acquire).is_null());
                // Store leader at tail
                group.store(&raw mut writer_1, Ordering::Release);

                writer_1
                    .state
                    .fetch_or(WriterState::LEADER, Ordering::Relaxed);

                if writer_1.is_leader() {
                    println!("We are leader");
                }
            });
        });
    }
}
