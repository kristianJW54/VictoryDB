// There is two stages/forms of batching:
//
// 1. The initial threads call to operations such as Put, Delete etc
// 2. The grouping of thread operations
//
//
// Thread A: Apply(batch[a])
// Thread B: Apply(batch[b])
// Thread C: Apply(batch[c])
//
//  queue = [A, B, C]
//
// Leader takes:
//     batch[a] + batch[b] + batch[c]
//     → combined execution
//
// for each writer in group:
//    for each record in writer.batch:
//        apply
//
//  API:
//     Set(a)
//     Set(b)
//     Set(c)
//
// becomes:
//
//     Batch[a]
//     Batch[b]
//     Batch[c]
//
// then:
//
// Writer Queue:
//     [Batch[a], Batch[b], Batch[c]]
//
// Leader:
//     executes all 3 in one go
//
//
// NOTE: Do we want two queues? One for data commit and one for WAL commit?
//
// Batches use a compact binary representation where all operations are encoded sequentially into a byte slice
// the binary representation is so that batches can form the records of the WAL without any additional changes
//
//
// | --------- 12 byte header ----------|--------- Operations ---------|
// | Seq No (8 bytes) | Count (4 bytes) | Operation 1 ... Operation 2...
//
//
//
// Due to the fact that batches are loaded into a writer-queue where they are grouped and then committed, they are cross-threaded so pooling batch
// memory becomes difficult as we must maintain shared ownership of the batch data across threads
// For example: we may apply batch enqueueing it to the commit pipeline and before returning it to the pool for re-use we must ensure no threads are still using it

// From pebble.go
//
// A commitPipeline manages the stages of committing a set of mutations
// (contained in a single Batch) atomically to the DB. The steps are
// conceptually:
//
//  1. Write the batch to the WAL and optionally sync the WAL
//  2. Apply the mutations in the batch to the memtable
//
// These two simple steps are made complicated by the desire for high
// performance. In the absence of concurrency, performance is limited by how
// fast a batch can be written (and synced) to the WAL and then added to the
// memtable, both of which are outside the purview of the commit
// pipeline. Performance under concurrency is the primary concern of the commit
// pipeline, though it also needs to maintain two invariants:
//
//  1. Batches need to be written to the WAL in sequence number order.
//  2. Batches need to be made visible for reads in sequence number order. This
//     invariant arises from the use of a single sequence number which
//     indicates which mutations are visible.
//
// Taking these invariants into account, let's revisit the work the commit
// pipeline needs to perform. Writing the batch to the WAL is necessarily
// serialized as there is a single WAL object. The order of the entries in the
// WAL defines the sequence number order. Note that writing to the WAL is
// extremely fast, usually just a memory copy. Applying the mutations in a
// batch to the memtable can occur concurrently as the underlying skiplist
// supports concurrent insertions. Publishing the visible sequence number is
// another serialization point, but one with a twist: the visible sequence
// number cannot be bumped until the mutations for earlier batches have
// finished applying to the memtable (the visible sequence number only ratchets
// up). Lastly, if requested, the commit waits for the WAL to sync. Note that
// waiting for the WAL sync after ratcheting the visible sequence number allows
// another goroutine to read committed data before the WAL has synced. This is
// similar behavior to RocksDB's manual WAL flush functionality. Application
// code needs to protect against this if necessary.
//
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
// WAL sync happens concurrently with applying to the memtable (see

// commitPipeline.syncLoop).
//
//
// Thread A: write(batch)
// Thread B: write(batch)

// Writer queue:
//     [A, B]

// Leader:
//     total_records = 3
//     base_seq = 100

// Execution:
//     memtable.apply(batch A)
//         insert seq 100
//         insert seq 101

//     memtable.apply(batch B)
//         insert seq 102

// Publish:
//     visible_seq = 102

// Signal:
//     wake A and B → success

pub(crate) struct Batch {}

// TODO: Do we want apply_batch on the memtable? and then memtable can handle the insert and handle if direct or not
