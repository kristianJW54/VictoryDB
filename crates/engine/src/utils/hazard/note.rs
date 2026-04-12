//
//
//
//
// HzdPtr (AtomicPtr custom)
//

// There is a Replaced concept where we want to return an old pointer after a swap or CAS which we can retire but we need to preserve the
// Pointer and Family(Domain) so the Replaced can call retire withouth having to pass in P or D again

use std::sync::atomic::AtomicPtr;

// -  A hazard pointer is a single-writer multi-reader pointer that can be owned by at most one
//      thread at any time. Only the owner of the hazard pointer can set its value, while any
//      number of threads may read its value. The owner thread sets the value of a hazard
//      pointer to point to an object in order to indicate to concurrent threads — that may delete
//      such an object — that the object is not yet safe to delete
//
// -  A hazard pointer belongs to exactly one domain
//
struct HzdPtr<T> {
    // Hazard: HzdPtrRec
    // domain: &'domain Domain
}

// pub(crate) struct HazPtrRecord {
//     pub(crate) ptr: AtomicPtr<u8>,
//     pub(crate) next: AtomicPtr<HazPtrRecord>,
//     pub(crate) available_next: AtomicPtr<HazPtrRecord>,
// }
//
//
