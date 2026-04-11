// We need a hazard ptr

use std::marker::PhantomData;
use std::sync::OnceLock;

// Owner of this hazard pointer is telling all threads:
// As long as this ptr points to Object T, do not reclaim
struct HzdPtr<'domain, D = Global> {
    hazard: &'domain HzdPtrRecord,
    _data: PhantomData<D>,
    _lifetime: PhantomData<&'domain ()>,
}

struct HzdPtrRecord {}

//
//
//
//
// Domains
// We can have multiple domains, each with its own hazard pointers and retired objects
// To do this threads must

//
// Domain manages a set of hazard pointers and set of retired objects

pub(crate) struct Global;
impl Global {
    fn new() -> Self {
        Global
    }
}

pub(crate) fn init_global_domain() -> &'static HzdDomain<Global> {
    static SHARED_DOMAIN: OnceLock<HzdDomain<Global>> = OnceLock::new();
    SHARED_DOMAIN.get_or_init(|| HzdDomain::new())
}

pub(crate) struct HzdDomain<T> {
    _data: PhantomData<T>,
}

impl<T> HzdDomain<T> {
    fn new() -> Self {
        Self { _data: PhantomData }
    }
}
