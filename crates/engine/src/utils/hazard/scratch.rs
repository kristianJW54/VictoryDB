//
//
//
//
// Objects protected by hazard pointers are derived from a base_class that provides member functions
// - retire()
// When A is removed: A.retire() is called to pass responsibility of reclamation to the hazard pointer
// A will only be reclaimed when it is no longer protected by a hazard pointer

use std::sync::atomic::{AtomicPtr, Ordering};

// Want to implement object base class for any type OR request that is has a super trait of Retireable which
// makes sure that the object can be retired
trait HazardObjectBaseClass {
    // type Domain?
    //
    fn retire(this: *mut Self) {
        let _ = (); // TODO
    }
    // fn retire_in(this: *mut Self, domain: &Domain); ??
}

// Need to think about this
impl<T> HazardObjectBaseClass for T {}

//
// Hazard pointer holder:
// Holds and owns a hazard pointer
struct HazardPointerHolder<T> {
    Hazard: HazardPtr<T>, //
}

impl<T> Default for HazardPointerHolder<T> {
    fn default() -> Self {
        Self {
            Hazard: HazardPtr::default(),
        }
    }
}

impl<T> HazardPointerHolder<T> {
    // Becuase we are using the std::AtomicPtr we can't be certain that the ptr inside is valid. We can only check if it's null or not.
    // Later we can add a custom AtomicPtr which ensures that ptr's can only be created from Box<T>
    unsafe fn load<'holder>(&mut self, ptr: &'_ AtomicPtr<T>) -> Option<&'holder T> {

        // Try to load the atomic ptr
        //
    }
}

pub(crate) struct HazardPtr<T> {
    ptr: *mut T,
}

impl<T> Default for HazardPtr<T> {
    fn default() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
        }
    }
}

// Example
//
//

#[test]
fn correct_api() {
    //
    //
    let x: AtomicPtr<u32> = AtomicPtr::new(Box::into_raw(Box::new(12 as u32)));

    // Create the hazard holder
    let mut h = HazardPointerHolder::default();
    // Load the hazard pointer
    let protected_x: &u32 = unsafe { h.load(&x) };

    drop(h);
    // Invalidates the hazard pointer
    let _ = *protected_x; // <- UB

    // Writer

    let old = x.swap(Box::into_raw(Box::new(13)), Ordering::SeqCst);
    HazardObjectBaseClass::retire(old);
}
