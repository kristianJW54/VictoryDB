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

pub(crate) struct WriteBatch {}
