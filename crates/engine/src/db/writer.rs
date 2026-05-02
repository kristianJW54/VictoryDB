use std::{
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, AtomicU8, Ordering},
    thread::{self, Thread},
};

use crate::{
    column_family::cf,
    db::{write_batch::Batch, write_thread::WriteThread},
};

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
    batch: NonNull<Batch>,
    state: AtomicU8,
    next: AtomicPtr<Writer>,
    thread_handle: Thread,
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

        while self.state.load(Ordering::Acquire) & WriterState::COMPLETE == 0 {
            thread::park();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn writer_state() {
        // Am i a follower
        let follow = WriterState::FOLLOWER;

        println!("{:08b}", follow);
        println!("{:08b}", WriterState::FOLLOWER);
        println!("{:08b}", follow | WriterState::FOLLOWER);

        println!("follower? -> {}", follow & WriterState::FOLLOWER != 0);
    }

    #[test]
    fn waiting_and_parking() {
        let scope = thread::scope(|t| {
            //
            // Make the writer and batch inside the thread scope
            let batch = Batch::new();
            let mut writer = Writer::new(&batch);

            let write_ptr = AtomicPtr::new(&raw mut writer);

            t.spawn(move || {
                thread::sleep(Duration::from_millis(1000));

                //
                let w = write_ptr.load(Ordering::Acquire);

                unsafe {
                    (*w).state
                        .fetch_or(WriterState::COMPLETE, Ordering::Release)
                };
                //
                unsafe {
                    (*w).thread_handle.unpark();
                }
            });

            println!("Blocking");
            writer.wait_and_block();
            println!("Unblocked");

            // Here the follower can block and wait
        });
    }
}
