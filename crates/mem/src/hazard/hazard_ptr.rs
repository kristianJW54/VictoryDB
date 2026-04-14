//
//
//

use std::{marker::PhantomData, ptr::NonNull, sync::atomic::AtomicPtr};

use crate::hazard::domain::Global;

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
    // hazard: HzdPtrRec,
    // domain: &'domain D,
    _f: PhantomData<D>,
    _l: PhantomData<&'domain ()>,
    ptr: AtomicPtr<u8>,
}

impl<'domain, Global> HzdPtr<'domain, Global> {
    pub(crate) fn make_hazard_ptr() -> Self {
        Self {
            _f: PhantomData,
            _l: PhantomData,
            ptr: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    pub fn protect<'hazard_object, T>(
        &'hazard_object mut self,
        ptr: &'_ AtomicPtr<T>,
    ) -> Option<&'hazard_object T> {
        let r = ptr.load(std::sync::atomic::Ordering::Relaxed);
        unsafe { Some(&*r) }
    }
}

// TODO: Implement protect_ptr()

impl<'domain, D> Drop for HzdPtr<'domain, D> {
    fn drop(&mut self) {
        unsafe {
            let p = self.ptr.load(std::sync::atomic::Ordering::Relaxed);
            if !p.is_null() {
                drop(Box::from_raw(p as *mut u8));
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn perfect_world() {
        // We want to be able to take our object A
        let mut a = AtomicPtr::new(Box::into_raw(Box::new(10i32)));

        // Then we want to be able to protect object A with a hazard pointer
        let mut hp: HzdPtr<Global> = HzdPtr::make_hazard_ptr();
        let ptr: &i32 = hp.protect(&a).expect("Non Null");

        println!("{:?}", ptr);

        // If we drop the hazard pointer, the protected object should only be reclaimed if safe
        // we should not be able to deference the hazard pointer after it has been dropped
        drop(hp);

        // Compile error
        // let _ = *ptr;
    }
}
