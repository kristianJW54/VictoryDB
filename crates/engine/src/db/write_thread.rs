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

use std::sync::atomic::Ordering;
use std::{ptr, sync::atomic::AtomicPtr};

use crate::db::writer::WriterState;

use super::write_batch::Batch;
use super::writer::Writer;

/// WriteThread is the coordination mechanism for multiple writes. Each calling thread will creater a writer holding a batch of operations and try to join
/// the write thread queue. The write thread will group multiple writes and determine leader/followers.
/// Once complete, it will signal to followers and drop
///
///
/// SAFETY:
///
/// WriteThread stores raw pointers to stack-owned Writer nodes.
///
/// A Writer passed to join() must remain alive until join() returns. join()
/// may publish the pointer to other threads, but it will not return for a
/// follower until the writer has reached a terminal state, and it will not
/// allow the writer to be dropped while another thread may still access it.
///
/// Therefore any Writer pointer reachable from the queue points to a live
/// Writer.
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

    fn link_writer(&self, writer: *mut Writer) -> bool {
        debug_assert!(unsafe { (*writer).state.load(Ordering::Relaxed) & WriterState::INIT != 0 });

        let mut current_newest_writer = self.head.load(Ordering::Relaxed);

        loop {
            // XXX: We can put write stall logic and control flow here

            // Link to the previously newest writer.
            // Produces a newest->older intrusive stack.
            unsafe {
                (*writer)
                    .link_older
                    .store(current_newest_writer, Ordering::Relaxed);
            }

            // CAS on current newest writer
            match self.head.compare_exchange_weak(
                current_newest_writer,
                writer,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(ptr) => return ptr.is_null(),
                Err(ptr) => {
                    current_newest_writer = ptr;
                    continue;
                }
            }
        }
    }

    pub(crate) fn join(&self, writer: &Writer) {
        //
        // Raw pointer form used for the intrusive queue. Lifetime is governed by
        // WriteThread::join's stack-writer invariant.
        let w = ptr::from_ref(writer).cast_mut();

        let linked_writer = self.link_writer(w);

        if linked_writer {
            debug_assert!(writer.is_leader());

            // Continue as Leader
            //
            //
        } else {
            writer.wait();
        }
    }
}

//

#[cfg(test)]
mod tests {
    use crate::db::writer::WriterState;

    use super::*;
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::thread::{self};
    use std::time::Duration;

    // TODO: Need to make this deterministic with while loop so we can enforce thread join order
    #[test]
    fn writer_follower_to_leader() {
        // XXX: Replace naive implementation with writer_thread methods - eventually move to integration test
        //
        let group: AtomicPtr<Writer> = AtomicPtr::new(ptr::null_mut());

        // Want:
        // leader -> follower 1 -> follower 2
        // To become:
        // follower 1 (new leader) -> follower 2

        // To make this deterministic we'll make each spawned thread sleep so we can control the order
        // We are testing logic->follower with third follower blocking on leader change

        // Assertion state
        let follower_1_state = AtomicU8::new(0);
        let follower_2_state = AtomicU8::new(0);

        thread::scope(|t| {
            // Leader
            t.spawn(|| {
                let batch = Batch::new();
                let mut writer_1 = Writer::new(&batch);

                // No wait - we want this to be leader

                group.store(&raw mut writer_1, Ordering::Release);

                // Set as leader
                writer_1
                    .state
                    .fetch_or(WriterState::LEADER, Ordering::Release);

                // Now wait for 1000ms to simulate processing group write and then set next leader
                thread::sleep(Duration::from_millis(1000));

                // We don't need to unpark because the next follower is the one we want to make leader
                // normally we'd traverse the linked list and process the group before either nulling the global head or
                // assigning new leader

                let follower = writer_1.group_next.load(Ordering::Acquire);

                assert!(!follower.is_null());
                unsafe {
                    (*follower)
                        .state
                        .fetch_or(WriterState::LEADER, Ordering::Release);
                    (*follower).thread_handle.unpark();
                }

                //
            });

            // Follower 1 (next leader)
            t.spawn(|| {
                let batch = Batch::new();
                let mut writer_2 = Writer::new(&batch);

                thread::sleep(Duration::from_millis(10));

                match group.compare_exchange(
                    group.load(Ordering::Acquire),
                    &raw mut writer_2,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                ) {
                    Ok(ptr) => {
                        // We have pointer to the leader - we need to set it's back_link to us
                        unsafe {
                            (*ptr)
                                .group_next
                                .store(&raw mut writer_2, Ordering::Relaxed);
                        }
                        // Set our next pointer to ptr we just stole from group head
                        writer_2.link_older.store(ptr, Ordering::Relaxed);
                    }
                    Err(_) => panic!(),
                }

                // Now block
                writer_2.wait_and_block();
                //

                // If we do become leader (which we should) check, simulate work and unpark followers
                if writer_2.is_leader() {
                    // Simulate work

                    thread::sleep(Duration::from_millis(1000));

                    // assert out back link is not null
                    assert!(!writer_2.group_next.load(Ordering::Relaxed).is_null());
                    let follower = writer_2.group_next.load(Ordering::Relaxed);

                    unsafe {
                        (*follower)
                            .state
                            .fetch_or(WriterState::COMPLETE, Ordering::Release);
                        if (*follower).state.load(Ordering::Acquire) & WriterState::LOCKED_WAITING
                            != 0
                        {
                            (*follower).thread_handle.unpark();
                        }
                    }
                }
                // Before we exit - log our state for assertion
                follower_1_state.store(writer_2.state.load(Ordering::Relaxed), Ordering::Relaxed);
            });

            // Follower 2
            t.spawn(|| {
                let batch = Batch::new();
                let mut writer_3 = Writer::new(&batch);

                thread::sleep(Duration::from_millis(20));

                match group.compare_exchange(
                    group.load(Ordering::Acquire),
                    &raw mut writer_3,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                ) {
                    Ok(ptr) => {
                        // We have pointer to the leader - we need to set it's back_link to us
                        unsafe {
                            (*ptr)
                                .group_next
                                .store(&raw mut writer_3, Ordering::Release);
                        }
                        // Set our next pointer to ptr we just stole from group head
                        writer_3.link_older.store(ptr, Ordering::Release);
                    }
                    Err(_) => panic!(),
                }

                // Now block
                writer_3.wait_and_block();
                //

                // Before we exit - log our state for assertion
                follower_2_state.store(writer_3.state.load(Ordering::Relaxed), Ordering::Relaxed);
            });
        });

        // assertions:
        assert!(follower_1_state.load(Ordering::Relaxed) & WriterState::LEADER != 0);
        assert!(follower_2_state.load(Ordering::Relaxed) & WriterState::COMPLETE != 0);
    }
}
