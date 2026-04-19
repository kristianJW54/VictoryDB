//
//
//
//
//
// DOCS: Describe global here and document
//
//
//
/// ## Reclamation
///
/// Domains are the coordination mechanism used for reclamation. When an object is retired into a
/// domain, the retiring thread will (sometimes) scan the domain for objects that are now safe to
/// reclaim (i.e., drop). Objects that cannot yet be reclaimed because there are active readers are
/// left in the domain for a later retire to check again. This means that there is generally a
/// delay between when an object is retired (i.e., marked as deleted) and when it is actually
/// reclaimed (i.e., [`drop`](core::mem::drop) is called). And if there are no more retires, the
/// objects may not be reclaimed until the owning domain is itself dropped.
///
///
// We want to be able to statically create unique domains using a Singleton pattern as a trait
// with a macro to generate unique domain instances based on Jon Gjongset's implementation:
// https://github.com/jonhoo/hazard/blob/master/src/domain.rs
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use crate::hazard::hazard_ptr::HzdPtrRec;

const LOCK_BIT: usize = 1;

// Make AtomicPtr usable with loom API.
trait WithMut<T> {
    fn with_mut<R>(&mut self, f: impl FnOnce(&mut *mut T) -> R) -> R;
}
impl<T> WithMut<T> for core::sync::atomic::AtomicPtr<T> {
    fn with_mut<R>(&mut self, f: impl FnOnce(&mut *mut T) -> R) -> R {
        f(self.get_mut())
    }
}

pub unsafe trait Singleton {}

// Macro to create unique static domain instances
//
#[macro_export]
macro_rules! unique_domain {
    () => {{
        fn create_domain() -> Domain<impl Singleton> {
            use ::core::sync::atomic::{AtomicBool, Ordering};
            struct UniqueFamily;
            // Safety: nowhere else can construct an instance of UniqueFamily to pass to
            // Domain::new, and we protect the construction by the `USED` boolean.
            unsafe impl Singleton for UniqueFamily {}
            static USED: AtomicBool = AtomicBool::new(false);
            if USED.compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed).is_ok() {
                Domain::new(&UniqueFamily)
            } else {
                panic!("`unique_domain!` macro cannot be executed more than once to maintain the `Singleton` constraints.")
            }
        }
        create_domain()
    }};
}

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

    pub(super) fn acquire_new_rec(&self) -> &HzdPtrRec {
        // First build the HzdPtrRec
        let rec = Box::into_raw(Box::new(HzdPtrRec {
            ptr: AtomicPtr::new(core::ptr::null_mut()),
            next: AtomicPtr::new(core::ptr::null_mut()),
            available: AtomicPtr::new(core::ptr::null_mut()),
        }));

        // Insert into the linked list
        //
        // - Get head first
        let mut head = self.hazard_pointers.head.load(Ordering::Acquire);

        // We need to loop as we try to CAS the current head with the new rec
        //

        loop {
            // NOTE: Not sure why this was used in HapHazard as it is supposed to help with Loom
            unsafe { &mut *rec }.next.with_mut(|p| *p = head);

            match self.hazard_pointers.head.compare_exchange_weak(
                head,
                rec,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    self.hazard_pointers.count.fetch_add(1, Ordering::SeqCst);
                    break unsafe { &*rec };
                }
                Err(changed_head) => {
                    head = changed_head;
                }
            }
        }
    }

    pub(super) fn acquire(&self) -> &HzdPtrRec {
        self.acquire_many::<1>()[0]
    }

    // Acquire_many returns an array of HzdPtrRecs
    // The reason we return and array and not the head of the acquired linked list is because the caller can do this:
    // hp[0].protect(ptr1);
    // hp[1].protect(ptr2);
    // hp[2].protect(ptr3);
    //
    pub(super) fn acquire_many<const N: usize>(&self) -> [&HzdPtrRec; N] {
        //
        //
        // Explanation
        //
        // We want to fill an array of 4
        // We try to get available HzdPtrRec's from the available_next() in Records but it only has 2
        // So we have to allocate 2 in order to fill the array. (The newly allocated rec's don't immediately go in the available_next but they do go in the main linked list)
        //
        // Iteration 1:
        // + available_next --------> A --------> B --------> null
        //                       Head ^
        //                            | -> rec
        //                            | Tail = Head
        //                            +---> Head available_next
        //
        // Iteration 2:
        // + available_next --------> A --------> B --------> null
        //                                   Head ^
        //                                        | -> rec
        //                                        | Tail = Head
        //                                        +---> Head available_next
        //
        // Iteration 3:
        // + available_next --------> A --------> B --------> null
        //                                                 Head ^
        //                                                      | -> rec
        //                                                      | Tail = Head
        //                                                      +---> Head available_next
        //
        // Iteration 4:
        // + available_next --------> A --------> B --------> null
        //                                                 Head ^
        //                                                      |
        //                                                      +
        //                                                      C -> acquire_new()
        //                                                 Tail ^
        //                                                      | -> rec
        //                                                      | Tail = rec
        //                                                      + ----> available_next
        //
        // Iteration 5:
        // + available_next --------> A --------> B --------> null
        //                                                 Head ^
        //                                                      |
        //                                                      +
        //                                                      C --------> D --> acquire_new()
        //                                                             Tail ^
        //                                                                  | -> rec
        //                                                                  | Tail = rec
        //                                                                  + ----> available_next
        //
        //
        // End result: [&HzdPtrRec, 4] = A --> B --> C --> D
        //
        // These are all on the head linked list but will be pushed back into available list once they are no longer protecting anything

        // First try to acquire available
        debug_assert!(N >= 1);
        let (mut head, n) = self.try_acquire_available::<N>();

        assert!(n <= N);

        // While loop

        todo!()
    }

    fn try_acquire_available<const N: usize>(&self) -> (*const HzdPtrRec, usize) {
        debug_assert!(N >= 1);
        // NOTE: HapHazard does this debug_assert_eq! and I don't know why yet
        // debug_assert_eq!(core::ptr::null::<HazPtrRec>() as usize, 0);

        loop {
            let avail_head = self.hazard_pointers.avail_head.load(Ordering::Acquire);
            if avail_head.is_null() {
                return (avail_head, 0);
            }

            // Here we want to try and get a lock on the head ptr with a LOCK_BIT

            // We can use fetch_or() which gives us back the original ptr value and uses map_addr() to tag the usize bits while preserving provenance
            // this is part of the strong provenance API in rust
            // By comparing the original ptr we get back with the LOCK_BIT we can see if someone else has the lock because the return value will
            // have the LOCK_BIT set if it was already locked whereas if we are the first to lock the original ptr will be returned without the LOCK_BIT set
            let locked = self
                .hazard_pointers
                .avail_head
                .fetch_or(LOCK_BIT, Ordering::Acquire);

            if locked.addr() & LOCK_BIT == 0 {
                // We have the lock and can proceed safely
                //
                // We pass in the original head_avail which is the untagged ptr so we don't need to mask out the LOCK_BIT
                // and because the locked ptr is still in self.hazard_pointers.avail_head, we can safely traverse.
                // Once we're done, we will store::release back into self.hazard_pointers.avail_head which will unlock
                let (ptr, n) = unsafe { self.try_acquire_available_locked::<N>(avail_head) };
                debug_assert!(n >= 1, "Head available was not null");
                debug_assert!(n <= N);
                //
                //
                return (ptr, n);
            } else {
                // The head is locked, we need to wait for it to be unlocked
                // HapHazard uses this:
                #[cfg(not(any(loom, feature = "std")))]
                core::hint::spin_loop();
                #[cfg(any(loom, feature = "std"))]
                crate::sync::yield_now();

                //
            }
        }
    }

    /// #SAFETY:
    ///
    /// The caller must ensure that the `ptr` is a valid `HzdPtrRec` pointer and that the caller
    /// has the lock on the `avail_head` before calling this function.
    unsafe fn try_acquire_available_locked<const N: usize>(
        &self,
        stolen_head: *const HzdPtrRec,
    ) -> (*const HzdPtrRec, usize) {
        //
        // We want to traverse the available_next on the HzdPtrRec for n elements we need or until next is null
        // We've acquired a lock on the start of an available list so we can safely traverse it
        //
        // Global available list
        // + head available --> A|LOCKED --> B --> C --> D --> E --> null
        //          we lock here ^      n = 3            |
        //                       |-----------------|--> next
        //                     Head               Tail   ^ unlock and store next as new head back on global list
        //
        // New Global available list
        // + head available --> C --> D --> E --> null
        //
        debug_assert!(N >= 1);

        let mut tail = stolen_head;
        let mut n = 1;

        // SAFETY:
        // The caller has the lock on `avail_head`, so `tail` is a valid `HzdPtrRec` pointer.
        // Relaxed ordering because we hold the lock
        let mut next = unsafe { &*tail }.available.load(Ordering::Relaxed);

        while !next.is_null() && n < N {
            //

            debug_assert_eq!((next as usize) | LOCK_BIT, 0);
            tail = next;

            next = unsafe { &*tail }.available.load(Ordering::Relaxed);
            n += 1;
        }

        // We need to store element after the end of our list we acquired back to the global list
        self.hazard_pointers
            .avail_head
            .store(next, Ordering::Release);

        // And we null the tail so it doesn't point to next
        unsafe { &*tail }
            .next
            .store(std::ptr::null_mut(), Ordering::Relaxed);

        (stolen_head, n)
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

    #[test]
    fn test_acquire_new_rec() {
        let rec = GLOBAL_DOMAIN.acquire_new_rec();
        assert_eq!(
            GLOBAL_DOMAIN.hazard_pointers.count.load(Ordering::Relaxed),
            1
        );
        println!("{:?}", rec);
    }

    #[test]
    fn ptr_locking() {
        const LOCK_BIT: usize = 1;

        let _ = GLOBAL_DOMAIN.acquire_new_rec();

        let hzdptr = GLOBAL_DOMAIN
            .hazard_pointers
            .avail_head
            .load(Ordering::Acquire);

        assert_eq!(hzdptr.addr(), 0);

        // NOTE: We need to make sure when locking ptr we keep provenance
        //
        // https://doc.rust-lang.org/std/ptr/index.html#provenance
        //
        // "pointers need to somehow be more than just their addresses: they must have provenance."
        // - A pointer value in Rust semantically contains the following information:
        //     + The address it points to, which can be represented by a usize.
        //     + The provenance it has, defining the memory it has permission to access.
        //       Provenance can be absent, in which case the pointer does not have permission to access any memory.
        //
        // From this discussion, it becomes very clear that a usize cannot accurately represent a pointer, and converting from a pointer
        // to a usize is generally an operation which only extracts the address.
        // Converting this address back into pointer requires somehow answering the question: which provenance should the resulting pointer have?
        //
        // https://doc.rust-lang.org/std/ptr/index.html#using-strict-provenance
        // Using strict provenance methods we can create a tagged pointer without having to do wrapping_add() tricks
        //

        let locked_ptr = hzdptr.map_addr(|ptr| ptr | LOCK_BIT);

        assert_eq!(locked_ptr.addr(), 1);

        // We can also use AtomicPtr::fetch_or()

        let atom = AtomicPtr::new(hzdptr);

        // Lock the atom
        let o = atom.fetch_or(LOCK_BIT, Ordering::Acquire);
        println!("were we locked? {:?}", o.addr() & LOCK_BIT != 0);

        let a = atom.fetch_or(LOCK_BIT, Ordering::Acquire);
        let n = a.addr() & LOCK_BIT != 0;
        println!("we are locked? {:?}", n);
    }

    #[test]
    fn array_map() {
        // Example of using array map to collect items into array with closure
        let mut i = 0;
        let r = [(); 3].map(|_| {
            i += 1;
            i
        });

        for (i, v) in r.into_iter().enumerate() {
            assert_eq!(v, i + 1);
        }
    }
}
