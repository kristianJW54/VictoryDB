use std::{
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, AtomicU8, Ordering},
    thread::{self, Thread},
};

use crate::db::{write_batch::Batch, write_thread::WriteThread};

#[non_exhaustive]
pub(super) struct WriterState;

impl WriterState {
    pub const INIT: u8 = 1 << 0;
    pub const LEADER: u8 = 1 << 1;
    pub const FOLLOWER: u8 = 1 << 2;
    pub const LOCKED_WAITING: u8 = 1 << 3;
    pub const COMPLETE: u8 = 1 << 4;
}

/// Writer is the calling threads write which holds a batch of operations.
///
/// A writer node is created on each Db operation (Put/Delete/Merge .. etc) and
/// will insert into the tail of the write thread becoming either the leader of a group of batches or a follower
///
/// The batch pointer is non-owning. The caller retains ownership and
/// responsibility for the Batch lifetime. The Writer destructor does
/// not drop the batch.
///
/// # Safety
///
/// Caller must guarantee batch outlives this Writer
pub(crate) struct Writer {
    pub(super) batch: NonNull<Batch>,
    pub(super) state: AtomicU8,
    pub(super) next: AtomicPtr<Writer>,
    pub(super) link_older: AtomicPtr<Writer>,
    pub(super) thread_handle: Thread,
}

// SAFETY: Writer fields accessed cross-thread are either atomic
// or protected by the thread parking mechanism
unsafe impl Sync for Writer {}

impl Writer {
    pub(crate) fn new(batch: &Batch) -> Self {
        Self {
            batch: NonNull::from(batch),
            state: AtomicU8::new(0),
            next: AtomicPtr::new(ptr::null_mut()),
            link_older: AtomicPtr::new(ptr::null_mut()),
            thread_handle: thread::current(),
        }
    }

    /// wait() is used when the calling thread of a write has joined the write_thread and becomes a follower in the group.
    ///
    /// It must wait and block until the leader completes the write pipeline.
    ///
    /// The wait() method is implemented on the Writer and not on the WriteThread because Writer must be able to create a CondVar on
    /// demand and pass in it's local state to the Mutex in order to be signalled.
    pub(crate) fn wait(&self) {
        debug_assert!(
            self.state.load(std::sync::atomic::Ordering::Relaxed) & WriterState::FOLLOWER != 0
        );

        // We have joined on the write_thread and are a follower in the write group. We must wait until the leader is done with the write.
        // There are three stages we can efficiently wait to avoid the heavy syscall on Condvar each time. We start with the first stage and go through
        // until we fallback to Condvar or the write is complete at any point during.
        //
        //
        // Synchronisation is maintained through the state machine which is checked on each loop and in each stage
        //
        // 1. loop 200 times using a "pause" for 1 micro sec
        // 2. Thread::yield()
        // 3. Thread parking (rocks uses Mutex and CondVar)
        //
        // This is inspired by Rocks code see: https://github.com/facebook/rocksdb/blob/763401b595c8c1647908356e42525aadd0b90eae/db/write_thread.cc#L64

        for _ in 0..200 {
            if self.state.load(Ordering::Acquire) & WriterState::COMPLETE != 0 {
                return;
            }
            std::hint::spin_loop();
        }

        // PERF: Include performance timings/collection here

        for _ in 0..WriteThread::YIELD_PAUSE_ITERATIONS {
            // XXX: Later if benchmarking shows contention, we can do what rocks did and add a predictive credit
            // based yield to determine if we should yield or fall through to block
            if self.state.load(Ordering::Acquire) & WriterState::COMPLETE != 0 {
                return;
            }
            thread::yield_now();
        }

        // Fall through to block
        self.wait_and_block();
    }

    #[inline]
    fn wait_and_block(&self) {
        self.state
            .fetch_or(WriterState::LOCKED_WAITING, Ordering::Release);

        while self.state.load(Ordering::Acquire) & (WriterState::COMPLETE | WriterState::LEADER)
            == 0
        {
            thread::park();
        }
    }

    #[inline(always)]
    pub(crate) fn is_leader(&self) -> bool {
        self.state.load(Ordering::Relaxed) & WriterState::LEADER != 0
    }
}

#[cfg(test)]
mod tests {
    use std::{thread::scope, time::Duration};

    use super::*;

    #[test]
    fn writer_state() {
        let batch = Batch::new();
        let writer = Writer::new(&batch);

        writer.state.store(WriterState::LEADER, Ordering::Relaxed);

        assert!(writer.is_leader());
    }

    #[test]
    fn waiting_and_blocking() {
        let batch = Batch::new();
        let writer = Writer::new(&batch);

        thread::scope(|t| {
            t.spawn(|| {
                thread::sleep(Duration::from_millis(1000));

                // In order for us to be able to use the writer as reference here and not get into borrow or thread boundry compilation mayhem
                // we must ensure that Writer lives for longer than the thread scope
                writer
                    .state
                    .fetch_or(WriterState::COMPLETE, Ordering::Release);

                if writer.state.load(Ordering::Acquire) & WriterState::LOCKED_WAITING != 0 {
                    writer.thread_handle.unpark();
                }
            });
            writer.wait_and_block();
            assert!(writer.state.load(Ordering::Acquire) & WriterState::COMPLETE != 0);
        });
    }

    #[test]
    fn follower_promoted_to_leader() {
        let batch = Batch::new();
        let writer = Writer::new(&batch);

        thread::scope(|t| {
            t.spawn(|| {
                thread::sleep(Duration::from_millis(10));
                // simulate current leader handing off leadership
                writer
                    .state
                    .fetch_or(WriterState::LEADER, Ordering::Release);
                if writer.state.load(Ordering::Acquire) & WriterState::LOCKED_WAITING != 0 {
                    writer.thread_handle.unpark();
                }
            });

            writer.wait_and_block();

            // assert woke as leader not complete
            assert!(writer.state.load(Ordering::Acquire) & WriterState::LEADER != 0);
            assert!(writer.state.load(Ordering::Acquire) & WriterState::COMPLETE == 0);
        });
    }
}
