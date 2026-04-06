#[cfg(test)]
mod tests {

    use crate::key::comparator::InternalKeyComparator;
    use crate::key::internal_key::{InternalKey, LookupKey, OperationType};
    use crate::memory::allocator::*;
    use crate::memory::*;
    use crate::memtable::memtable::*;

    #[test]
    fn memtable_basic_insert_and_get() {
        let mem = Memtable::new(
            0,
            ArenaSize::Custom(640, 640),
            Allocator::System(SystemAllocator::new()),
            InternalKeyComparator::new(),
        );

        // Put a few keys in the memtable

        let key_1 = InternalKey::new(b"51.1.User1001", 1, OperationType::Put);
        let key_2 = InternalKey::new(b"51.1.User1001", 2, OperationType::Put);
        let key_3 = InternalKey::new(b"51.1.User1001", 3, OperationType::Put);
        let key_4 = InternalKey::new(b"51.1.User1001", 4, OperationType::Delete);
        let wrong_key = InternalKey::new(b"51.1.User1002", 5, OperationType::Put);

        mem.insert(key_1.as_ref(), b"value_1");
        mem.insert(key_2.as_ref(), b"value_2");
        mem.insert(key_3.as_ref(), b"value_3");
        mem.insert(key_4.as_ref(), b"");
        mem.insert(wrong_key.as_ref(), b"value_4");

        // Get the value for most recent seq no of 5
        let search_key = LookupKey::new(b"51.1.User1001", 8);
        let result = mem.get(search_key);
        assert!(matches!(result, MemReturn::Deleted));

        // Get the value for snapshot seq no of 3
        let search_key = LookupKey::new(b"51.1.User1001", 3);
        let result = mem.get(search_key);
        assert!(matches!(result, MemReturn::Value(b"value_3")));
    }

    #[test]
    fn memtable_memory_usage() {

        // Test filling up a memtable and checking chunk usage is working
    }
}
