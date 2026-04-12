//
//
//
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
struct HzdPtr {
    hazard: HzdPtrRec
    domain: &'domain Domain
}

impl HzdPtr {

    pub(crate) fn make_hazard_ptr() -> Self {

    }

}
