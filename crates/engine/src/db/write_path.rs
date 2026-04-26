//
//
//
//
use super::write_batch::WriteBatch;

use std::marker::PhantomData;

pub(crate) struct WriterContext {}

pub(crate) struct SynOptions {} // -> fysnc bool etc maybe env options

pub(crate) struct Writer {
    ctx: WriterContext,
    sync: SynOptions,
}

// Impl

impl Writer {
    pub(crate) fn apply_batch(&self, batch: &WriteBatch) {
        //
        // What work to do here?
        //

        #[cfg(feature = "buffered_key_writer")]
        {
            self.apply_buffered(batch);
        }
        #[cfg(feature = "arena_direct")]
        {
            self.apply_arena_direct(batch);
        }

        //
        //
    }

    // TODO: Top level API's worth exploring

    // fn log_data(data &[u8], options?) -> Write only to the WAL and not to mem useful for testing
    //
    // fn merge(key: &[u8], value: &[u8], options?)
    //
    //

    fn apply_buffered(&self, batch: &WriteBatch) {}

    fn apply_arena_direct(&self, batch: &WriteBatch) {}
}
