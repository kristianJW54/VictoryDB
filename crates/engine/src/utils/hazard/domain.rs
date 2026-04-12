//
//
//
//
//
//
//
//
//

use std::sync::OnceLock;

unsafe trait Family {}

struct Global;
unsafe impl Family for Global {}

// Single static instance of HzdDomain<Global>
//
fn make_global() -> &'static HzdDomain<Global> {
    static GLOBAL_DOMAIN: OnceLock<HzdDomain<Global>> = OnceLock::new();
    GLOBAL_DOMAIN.get_or_init(|| HzdDomain {
        _family: std::marker::PhantomData,
    })
}

pub(crate) struct HzdDomain<F: Family> {
    _family: std::marker::PhantomData<F>,
}

impl HzdDomain<Global> {}
