//
//
//

use std::sync::atomic::AtomicPtr;

pub(crate) struct HzdPtrRec {
    ptr: AtomicPtr<u8>,
    next: AtomicPtr<HzdPtrRec>,
    available: AtomicPtr<HzdPtrRec>,
}

//
//
//
// -  A hazard pointer is a single-writer multi-reader pointer that can be owned by at most one
//      thread at any time. Only the owner of the hazard pointer can set its value, while any
//      number of threads may read its value. The owner thread sets the value of a hazard
//      pointer to point to an object in order to indicate to concurrent threads — that may delete
//      such an object — that the object is not yet safe to delete
//
// -  A hazard pointer belongs to exactly one domain
//

// Hazard Pointer is a container object which acts as a handle to an inner container which persists in a domains linked list
// the inner container is a record which holds the pointer to the protected object
struct HzdPtr<'domain, D> {
    hazard: HzdPtrRec,
    domain: &'domain D,
}

impl<'domain, D> HzdPtr<'domain, D> {
    pub(crate) fn make_hazard_ptr() -> Self {
        todo!()
    }
}
