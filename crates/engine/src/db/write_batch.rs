//
//
// NOTE: Do we want two queues? One for data commit and one for WAL commit?
//
// Batches use a compact binary representation where all operations are encoded sequentially into a byte slice
// the binary representation is so that batches can form the records of the WAL without any additional changes
//
// Batch:
// | --------- 12 byte header ----------|--------- Operations ---------|
// | Seq No (8 bytes) | Count (4 bytes) | Operation 1 ... Operation 2...
//
//
// Operation:
// | op_type (1 byte) | cf_if (4 bytes) | key_len (1 byte) | key ... | value_len (1 byte) | value ... |
//
//
// A batch holds a set of operations to be committed atomically as part of the write path.
// Each operation is binary encoded and appended to a contiguous Vec<u8> buffer.
// The buffer begins with a 12-byte header:
//   - 8 bytes: starting sequence number (assigned at commit time)
//   - 4 bytes: operation count
//
// Batches are created both implicitly (e.g. DB::put) and explicitly by users.
// A single DB::put() creates a batch containing one operation, allowing the
// write path to uniformly operate on batches regardless of origin.
//
// Example (Pseudo code):
//
// DB::put("key1", "value1");
//
// // Internally:
//
// fn put(&self, key: &[u8], value: &[u8]) {
//     let mut batch = Batch::new();
//     batch.put(DEFAULT_CF, key, value);
//     self.write(batch);
// }
//
// // Later in the write path:
//
// fn write(&self, mut batch: Batch) {
//     let base_seq = self.seq.fetch_add(batch.count() as u64);
//     batch.set_seq_and_count(base_seq);
//
//     // WAL write
//     wal.write(&batch.data);
//
//     // Apply to memtable
//     let mut seq = base_seq;
//     for rec in batch.iter() {
//         mem.insert(rec.key, seq, rec.kind, rec.value);
//         seq += 1;
//     }
//
//     self.visible_seq.store(seq - 1);
// }
//
//
// Due to the fact that batches are loaded into a writer-queue where they are grouped and then committed, they are cross-threaded so pooling batch
// memory becomes difficult as we must maintain shared ownership of the batch data across threads
// For example: we may apply batch enqueueing it to the commit pipeline and before returning it to the pool for re-use we must ensure no threads are still using it
//
// --------------------------------------------------------------------------------------
// The full outline of the commit pipeline operation is as follows:
//
//	with commitPipeline mutex locked:
//	  assign batch sequence number
//	  write batch to WAL
//	(optionally) add batch to WAL sync list
//	apply batch to memtable (concurrently)
//	wait for earlier batches to apply
//	ratchet read sequence number
//	(optionally) wait for the WAL to sync
//
// As soon as a batch has been written to the WAL, the commitPipeline mutex is
// released allowing another batch to write to the WAL. Each commit operation
// individually applies its batch to the memtable providing concurrency. The
// WAL sync happens concurrently with applying to the memtable
// --------------------------------------------------------------------------------------
//
// As a default, a batch is initialised with 1KB (taken from Pebble - https://github.com/cockroachdb/pebble/blob/a3b8dfe9/batch.go#L38)
//
const DEFAULT_BATCH_INIT_SIZE: usize = 1 << 10; // NOTE: This is where we'd like to get to if we pool batches
const MAX_BATCH_SIZE: usize = 1 << 20;
const NON_POOL_BATCH_INIT_SIZE: usize = 1 << 6; // NOTE: For now we start small (cache line) and grow if needed as we allocate on each batch for now

pub(crate) struct Batch {
    data: Vec<u8>,
    // content_flags
    // protection_info
    // save_points
    // wal_term_point
    // max_bytes
}

// A record in a batch will have an operation type and a column family ID followed by varstring key and value.
//
// Get_op()
// PutCF()
// DeleteCF()
//

// TODO: Do we want apply_batch on the memtable? and then memtable can handle the insert and handle if direct or not
//

impl Batch {
    //
    pub(crate) fn new() -> Self {
        Self {
            data: Vec::with_capacity(NON_POOL_BATCH_INIT_SIZE),
        }
    }

    pub(crate) fn new_with_capacity(cap: usize) -> Self {
        // NOTE: This, I don't like. Would like to limit big batches and maybe ensure the caller
        // knows that using max batches will encur direct flushable memtables
        assert!(cap <= MAX_BATCH_SIZE);
        Self {
            data: Vec::with_capacity(cap),
        }
    }

    // TODO: Finish
    pub(crate) fn put<K, V>(&self, key: K, value: V)
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        println!("preparing batch");
        self.put_bytes(key.as_ref(), value.as_ref())
    }

    pub(crate) fn put_bytes(&self, key: &[u8], value: &[u8]) {
        println!("adding operation bytes")
    }

    // TOOD: Add()

    // NOTE: Can we defer creation until commit and then build the vec?
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn batch_init_size() {
        println!("batch size {}", DEFAULT_BATCH_INIT_SIZE);
        println!("single op {}", NON_POOL_BATCH_INIT_SIZE);
        println!("max {}", MAX_BATCH_SIZE);
    }

    #[test]
    fn input_test() {
        let word = "word";
        let batch = Batch::new();

        batch.put(word, "");

        // DB::put(word, value: "");
    }
}
