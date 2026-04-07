#[cfg(test)]
mod tests {

    use crate::iterator::internal_iterator::InternalIterator;
    use crate::key::comparator::InternalKeyComparator;
    use crate::key::internal_key::{InternalKeyRef, OperationType};
    use crate::key::internal_key::{LookUpInternalKey, LookUpKey};
    use crate::memory::allocator::*;
    use crate::memory::*;
    use crate::memtable::memtable::*;

    #[test]
    fn seek_to_for_memtable() {
        let mem = Memtable::new(
            0,
            ArenaSize::Custom(640, 640),
            Allocator::System(SystemAllocator::new()),
            InternalKeyComparator::new(),
        );
        // Put a few keys in the memtable

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

        // Now we want to iterate through the memtable with InternalIterator

        let mut int_iter = mem.iter();

        int_iter.seek_to_first();
        assert_eq!(int_iter.internal_key(), InternalKeyRef::from(k_4.as_ref()));

        // Now try next()

        let assert = vec![
            InternalKeyRef::from(k_4.as_ref()),
            InternalKeyRef::from(k_3.as_ref()),
            InternalKeyRef::from(k_2.as_ref()),
            InternalKeyRef::from(k_1.as_ref()),
        ];

        assert_eq!(int_iter.internal_key(), assert[0]);

        for i in 1..4 {
            int_iter.next();
            assert_eq!(int_iter.internal_key(), assert[i]);
        }

        // Now want to test seek()

        let mut int_iter_2 = mem.iter();
        let lk: InternalKeyRef = InternalKeyRef::from(k_2.as_ref());
        int_iter_2
            .seek(LookUpInternalKey::new(lk.user_key, lk.seq_no, OperationType::Max).as_ref());
        assert_eq!(
            int_iter_2.internal_key(),
            InternalKeyRef::from(k_2.as_ref())
        );

        // Seek to key and test look up key on higher seq_no
        int_iter_2.seek(LookUpInternalKey::new(lk.user_key, 4, OperationType::Max).as_ref());
        assert_eq!(int_iter_2.internal_key().op, OperationType::Delete as u8)
    }
}
