// Range Delete Tombstones are simply:
// [start, end) @ seqno
// Delete all keys in the range [start, end) that are older than seqno
//
// They are represented in bytes as:
// Internal Key [start, op_type << seqno] -> Value [end]

// Spans are used to answer:
// What tombstones apply over THIS region of keyspace
//
// So for tombstones:
// [d, g)#10
// [f, j)#9

// Becomes:
// [d, f) → {#10}
// [f, g) → {#10, #9}
// [g, j) → {#9}
//
//
// From pebble db:
// # 3: a-----------m
// # 2:      f------------s
// # 1:          j---------------z
//
// Span: a-f:{(#3,RANGEDEL)}
// Span: f-j:{(#3,RANGEDEL) (#2,RANGEDEL)}
// Span: j-m:{(#3,RANGEDEL) (#2,RANGEDEL) (#1,RANGEDEL)}
// Span: m-s:{(#2,RANGEDEL) (#1,RANGEDEL)}

pub(crate) struct RangeDeleteOp<'a> {
    pub(crate) end: &'a [u8],
    pub(crate) seqno: u64,
}

// TODO: Algorithms and topics to search for span creation
// sweep line interval algorithm
// interval overlap sweep
// merge overlapping intervals streaming
// event-based interval processing
