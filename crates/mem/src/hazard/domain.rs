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

use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use crate::hazard::hazard_ptr::HzdPtrRec;

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

/*

// From
// https://github.com/jonhoo/haphazard/blob/main/src/domain.rs#L917

// Macro to make new const only when not in loom.
macro_rules! new {
    ($($decl:tt)*) => {
        /// Construct a new domain with the given family type.
        ///
        /// The type checker protects you from accidentally using a `HazardPointer` from one domain
        /// _family_ (the type `F`) with an object protected by a domain in a different family.
        /// However, it does _not_ protect you from mixing up domains with the same family type.
        /// Therefore, prefer creating domains with [`unique_domain`] or [`static_unique_domain`] where
        /// possible, since they guarantee a unique `F` for every domain.
        ///
        /// See the [`Domain`] documentation for more details.
        pub $($decl)*(_: &'_ F) -> Self {
            // https://blog.rust-lang.org/2021/02/11/Rust-1.50.0.html#const-value-repetition-for-arrays
            #[cfg(not(loom))]
            let untagged = {
                // https://github.com/rust-lang/rust-clippy/issues/7665
                #[allow(clippy::declare_interior_mutable_const)]
                const RETIRED_LIST: RetiredList = RetiredList::new();
                [RETIRED_LIST; NUM_SHARDS]
            };
            #[cfg(loom)]
            let untagged = {
                [(); NUM_SHARDS].map(|_| RetiredList::new())
            };
            Self {
                hazptrs: HazPtrRecords {
                    head: AtomicPtr::new(core::ptr::null_mut()),
                    head_available: AtomicPtr::new(core::ptr::null_mut()),
                    count: AtomicIsize::new(0),
                },
                untagged,
                count: AtomicIsize::new(0),
                #[cfg(all(feature = "std", target_pointer_width = "64", not(loom)))]
                due_time: AtomicU64::new(0),
                nbulk_reclaims: AtomicUsize::new(0),
                family: PhantomData,
                shutdown: false,
            }
        }
    };
}
 */

// Macro to make new const only when not in loom.
macro_rules! new {
     ($($decl:tt)*) => {
         pub $($decl)*(_: &'_ F) -> Self {


             Self {
                 hazard_pointers: HazPtrRecs {
                     head: AtomicPtr::new(core::ptr::null_mut()),
                     avail_head: AtomicPtr::new(core::ptr::null_mut()),
                     count: AtomicUsize::new(0),
                     _inner: () },
                 family: std::marker::PhantomData,
             }
         }
     };
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
    #[cfg(not(loom))]
    new!(const fn new);
    #[cfg(loom)]
    new!(fn);

    // Acquire new HzdRec and insert it into the linked list

    pub fn acquire_new_rec(&self) -> &HzdPtrRec {
        // First build the HzdPtrRec
        let rec = Box::new(HzdPtrRec {
            ptr: AtomicPtr::new(core::ptr::null_mut()),
            next: AtomicPtr::new(core::ptr::null_mut()),
            available: AtomicPtr::new(core::ptr::null_mut()),
        });

        // Insert into the linked list
        //
        // - Get head first
        let mut head = self.hazard_pointers.head.load(Ordering::Acquire);

        // We need to loop as we try to CAS the current head with the new rec
        //

        loop {
            // TODO: Finish
            break;
        }

        todo!()
    }
}

// Hazard Pointer Records which is the Linked List of HzdPtrRec which are the containers for hazard pointers to load into and protect object
// pointers in

pub struct HazPtrRecs {
    head: AtomicPtr<HzdPtrRec>,
    avail_head: AtomicPtr<HzdPtrRec>,
    count: AtomicUsize,
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

        // This should not be allowed
        // static_unique_domain!(static TEST2: HzdDomain<Test>);

        // struct SomeOtherDataStructure {
        //     domain: &'static HzdDomain<Test>,
        // }
    }
}
