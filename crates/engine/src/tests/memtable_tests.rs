#[cfg(test)]
mod tests {

    use crate::key::comparator::InternalKeyComparator;
    use crate::key::ephemeral_key::{EphemeralInternalKey, EphemeralKey};
    use crate::key::internal_key::OperationType;
    use crate::key::lookup_key::{LookUpInternalKey, LookUpKey};

    use crate::memtable::memtable::*;
    use mem::allocator::*;
    use mem::arena::*;

    #[test]
    fn memtable_basic_insert_and_get() {
        let mem = Memtable::new(
            0,
            ArenaPolicy {
                block_size: 640,
                cap: 640,
            },
            Allocator::System(SystemAllocator::new()),
            InternalKeyComparator::new(),
        );

        // Put a few keys in the memtable

        let k_1: LookUpInternalKey = LookUpKey::new(b"51.1.User1001", 1, OperationType::Put);
        let k_2: LookUpInternalKey = LookUpKey::new(b"51.1.User1001", 2, OperationType::Put);
        let k_3: LookUpInternalKey = LookUpKey::new(b"51.1.User1001", 3, OperationType::Put);
        let k_4: LookUpInternalKey = LookUpKey::new(b"51.1.User1001", 4, OperationType::Delete);
        let k_wrong: LookUpInternalKey = LookUpKey::new(b"51.1.User1002", 5, OperationType::Put);

        mem.insert(k_1.as_ref(), b"value_1");
        mem.insert(k_2.as_ref(), b"value_2");
        mem.insert(k_3.as_ref(), b"value_3");
        mem.insert(k_4.as_ref(), b"");
        mem.insert(k_wrong.as_ref(), b"value_4");

        // Get the value for most recent seq no of 5
        let search_key: LookUpInternalKey = LookUpKey::new(b"51.1.User1001", 8, OperationType::Max);
        let result = mem.get(search_key.as_ref());
        assert!(matches!(result, MemReturn::Deleted));

        // Get the value for snapshot seq no of 3
        let search_key: LookUpInternalKey = LookUpKey::new(b"51.1.User1001", 3, OperationType::Max);
        let result = mem.get(search_key.as_ref());
        assert!(matches!(result, MemReturn::Value(b"value_3")));
    }

    #[test]
    fn memtable_ephemeral_key_tests() {
        let mem = Memtable::new(
            0,
            ArenaPolicy {
                block_size: 640,
                cap: 640,
            },
            Allocator::System(SystemAllocator::new()),
            InternalKeyComparator::new(),
        );

        // Insert a few Ephemeral keys

        let mut ek: EphemeralInternalKey = EphemeralKey::new();

        ek.with_ephemeral_key(b"51.1.User1001", 1, OperationType::Put, |k| {
            mem.insert(k, b"value_1");
        });

        ek.with_ephemeral_key(b"51.1.User1001", 2, OperationType::Put, |k| {
            mem.insert(k, b"value_2");
        });
        ek.with_ephemeral_key(b"51.1.User1001", 3, OperationType::Put, |k| {
            mem.insert(k, b"value_3");
        });
        ek.with_ephemeral_key(b"51.1.User1001", 4, OperationType::Put, |k| {
            mem.insert(k, b"value_4");
        });
        ek.with_ephemeral_key(b"51.1.User1002", 5, OperationType::Put, |k| {
            mem.insert(k, b"value_5");
        });

        // Now we look up with an Ephemeral key

        let result = ek.with_ephemeral_key(b"51.1.User1001", 3, OperationType::Max, |v| {
            // Lookup the key and return the value
            let r = mem.get(v);
            match r {
                MemReturn::Value(value) => value,
                _ => b"",
            }
        });

        assert_eq!(result, b"value_3");
    }

    #[test]
    fn memtable_memory_usage() {

        // Test filling up a memtable and checking chunk usage is working
    }
}
