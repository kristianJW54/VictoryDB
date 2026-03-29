#[cfg(test)]
mod tests {

    use crate::storage::iterator::internal_iterator::InternalIterator;
    use crate::storage::key::comparator::InternalKeyComparator;
    use crate::storage::key::internal_key::{
        InternalKey, InternalKeyRef, LookupKey, OperationType,
    };
    use crate::storage::memory::allocator::*;
    use crate::storage::memory::*;
    use crate::storage::memtable::memtable::*;

    #[test]
    fn seek_to_for_memtable() {
        let mem = Memtable::new(
            0,
            ArenaSize::Test(640, 640),
            Allocator::System(SystemAllocator::new()),
            InternalKeyComparator::new(),
        );
        // Put a few keys in the memtable

        let key_1 = InternalKey::new(b"51.1.User1001", 1, OperationType::Put);
        let key_2 = InternalKey::new(b"51.1.User1001", 2, OperationType::Put);
        let key_3 = InternalKey::new(b"51.1.User1001", 3, OperationType::Put);
        let key_4 = InternalKey::new(b"51.1.User1001", 4, OperationType::Delete);
        let wrong_key = InternalKey::new(b"51.1.User1002", 5, OperationType::Put);

        //
        mem.insert(key_1.as_ref(), b"value_1");
        mem.insert(key_2.as_ref(), b"value_2");
        mem.insert(key_3.as_ref(), b"value_3");
        mem.insert(key_4.as_ref(), b"");
        mem.insert(wrong_key.as_ref(), b"value_4");

        // Now we want to iterate through the memtable with InternalIterator

        let mut int_iter = mem.iter();

        int_iter.seek_to_first();
        println!(
            "{}:{:?}",
            InternalKeyRef::from(int_iter.key()),
            String::from_utf8_lossy(int_iter.value())
        );
    }
}
