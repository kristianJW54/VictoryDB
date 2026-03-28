// A Span represents a contiguous region of the user keyspace `[start, end)`
// over which the set of active range operations (e.g. range deletions)
// does not change.
//
// Conceptually, range tombstones may overlap arbitrarily:
//
//     [d, g)@10
//     [f, j)@9
//
// During iteration, these are logically fragmented into regions where the
// active set of tombstones is constant:
//
//     [d, f) → {10}
//     [f, g) → {10, 9}
//     [g, j) → {9}
//
// Each such region is a Span.
//
// In practice, spans are NOT stored directly. Instead:
// - The storage layer holds raw range tombstones `(start, end, seqno)`.
// - The iterator maintains an "active set" of tombstones while scanning.
// - A span is derived on-the-fly as the region until the next change
//   (either a tombstone start or end).
//
// Spans enable efficient iteration by allowing the engine to skip entire
// regions of keys where the visibility rules are identical, rather than
// evaluating each key individually.
//
// Note: Some systems materialize spans with an explicit set of tombstones
// (e.g. `Keys[]`), but this implementation may instead track the active set
// incrementally without allocating per-span structures.
//

use crate::storage::range::RangeOp;

pub(crate) struct Span<'a> {
    pub(crate) start: &'a [u8],
    pub(crate) end: &'a [u8],
    pub(crate) span_set: &'a [RangeOp<'a>],
}

// TODO: Algorithms and topics to search for span creation
// sweep line interval algorithm
// interval overlap sweep
// merge overlapping intervals streaming
// event-based interval processing
