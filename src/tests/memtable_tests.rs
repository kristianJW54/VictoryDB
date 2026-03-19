use crate::storage::key::comparator::DefaultComparator;
use crate::storage::memory::allocator::*;
use crate::storage::memory::*;
use crate::storage::memtable::memtable::*;

#[test]
fn memtable_basic_insert_and_get() {
    let mem = Memtable::new(
        0,
        ArenaSize::Test(320, 320),
        Allocator::System(SystemAllocator::new()),
        DefaultComparator::new(),
    );

    // Put a few keys in the memtable

    // 1. Table 51 | Index 1 | Primary Key User 1001 |
    // 2. Table 51 | Index 2 | Primary Key User 1002 |
    // 3. Table 51 | Index 3 | Primary Key User 1003 |
    //

    mem.insert(b"51.1.User1001", b"Dave");
    mem.insert(b"51.2.User1002", b"Amy");
    mem.insert(b"51.3.User1003", b"Elly");

    // Get the values back and verify
    assert_eq!(mem.get(b"51.1.User1001"), Some(b"Dave".as_slice()));
    assert_eq!(mem.get(b"51.2.User1002"), Some(b"Amy".as_slice()));
    assert_eq!(mem.get(b"51.3.User1003"), Some(b"Elly".as_slice()));
}
