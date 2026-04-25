//
// SSTable - Sorted segment tables
//
// hold sorted key -> values
//
//
//
// Blocks:
// Chunks of contiguous key-value entries inside an SSTable
//
// SSTable file:
//
// [Block 0]
// [Block 1]
// [Block 2]
// ..
// [Index Block]
// [Footer]

// Block 0:
// Key1 - Value1
// Key2 - Value2
// Key3 - Value3

const SLICE_LEN: usize = 5;
const MEM_SIZE: usize = 25;

// Fake block
struct Block(Vec<u8>);

impl Default for Block {
    fn default() -> Self {
        Self(Vec::with_capacity(MEM_SIZE))
    }
}
