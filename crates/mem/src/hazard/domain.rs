//
//
//
//
//
// DOCS: Describe global here and document
//
//
//
//

// We want to be able to statically create unique domains using a Singleton pattern as a trait
// with a macro to generate unique domain instances based on Jon Gjongset's implementation:
// https://github.com/jonhoo/hazard/blob/master/src/domain.rs

pub unsafe trait Singleton {}

// Macro to create unique static domain instances
//

#[macro_export]
macro_rules! static_unique_domain {
    ($v:vis static $domain:ident: HzdDomain<$family:ident>) => {
        #[allow(non_snake_case)]
        mod $domain {
            pub struct $family {
                _inner: (),
            }
            // Safety: $family can only be constructed by this module, since it contains private members
            unsafe impl $crate::hazard::domain::Singleton for $family {}
            pub static $domain: $crate::hazard::domain::HzdDomain<$family> = $crate::hazard::domain::HzdDomain::new(&$family {
                _inner: (),
            });
        }
        #[allow(unused_imports)]
        $v use $domain::$family;
        #[allow(unused_imports)]
        $v use $domain::$domain;
    }
}

#[non_exhaustive]
pub struct Global;
impl Global {
    const fn new() -> Self {
        Global
    }
}

unsafe impl Singleton for Global {}

static GLOBAL_DOMAIN: HzdDomain<Global> = HzdDomain::new(&Global::new());

pub struct HzdDomain<F> {
    hazard_pointers: HazPtrRecs,
    // Will have the retired list
    family: std::marker::PhantomData<F>,
    // Meta data...
}

impl<F> HzdDomain<F> {
    pub const fn new(family: &F) -> Self {
        Self {
            hazard_pointers: HazPtrRecs { _inner: () },
            family: std::marker::PhantomData,
        }
    }
}

// Hazard Pointer Records which is the Linked List of HzdPtrRec which are the containers for hazard pointers to load into and protect object
// pointers in

pub struct HazPtrRecs {
    _inner: (),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_families() {
        static_unique_domain!(static TEST: HzdDomain<Test>);

        struct SomeDataStructure {
            domain: &'static HzdDomain<Test>,
        }
    }
}
