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

    /*

    1. First we get the pointer from the given AtomicPtr<T> as relaxed because we will double-check in the loop

    2. We then loop and try_protect() the pointer, also giving the src AtomicPtr<T> to compare against

    - From folly/synchronization/HazptrHolder.h

    template <typename T, typename Func>
      FOLLY_ALWAYS_INLINE T* protect(const Atom<T*>& src, Func f) noexcept {
        T* ptr = src.load(std::memory_order_relaxed);
        while (!try_protect(ptr, src, f)) {
          /* Keep trying */
        }
        return ptr;
      }

    - From haphazard//src/hazard.rs
        + In Rust world, the HapHazard library has a protect() method which calls protect_ptr() but i'm assuming
          that the let(ptr, _proof) <- Signature here makes the compiler check the proof via signature so the lifetime is the same as 'l

    pub unsafe fn protect<'l, T>(&'l mut self, src: &'_ AtomicPtr<T>) -> Option<&'l T>
        where
            T: Sync,
            F: 'static,
        {
            // NOTE: The type ascription here ensures that `protect_ptr` indeed returns a lifetime of
            // `'l` as we expect. It is a no-op, but will catch cases where `protect_ptr` changes in
            // the future.
            let (ptr, _proof): (_, PhantomData<&'l T>) = self.protect_ptr(src)?;
            Some(unsafe { ptr.as_ref() })
        }

    3. In trying to protect ptr we must use a fence and acquire store

    template <typename T, typename Func>
      FOLLY_ALWAYS_INLINE bool try_protect(
          T*& ptr, const Atom<T*>& src, Func f) noexcept {
        /* Filtering the protected pointer through function Func is useful
           for stealing bits of the pointer word */
        auto p = ptr;
        reset_protection(f(p));
        /*** Full fence ***/ folly::asymmetric_thread_fence_light(
            std::memory_order_seq_cst);
        ptr = src.load(std::memory_order_acquire);
        if (FOLLY_UNLIKELY(p != ptr)) {
          reset_protection();
          return false;
        }
        return true;
      }


    */

    pub fn protect<'hazard_object, T>(
        &'hazard_object mut self,
        src: &'_ AtomicPtr<T>,
    ) -> Option<&'hazard_object T> {
        // Logic
        //
        // Load the given AtomicPtr<T> into the hazard pointer
        // To do so we must first load the AtomicPtr<T> to get the stored ptr
        // Then we need to store the ptr in the hazard pointer
        // And load the pointer again to check that the ptr hasn't changed

        let mut ptr = src.load(std::sync::atomic::Ordering::Relaxed);
        loop {
            match self.try_protect(ptr, src) {
                _ => break,
            }
        }

        //
        let r = src.load(std::sync::atomic::Ordering::Relaxed);
        unsafe { Some(&*r) }
    }

    fn try_protect<T>(&mut self, ptr: *mut T, src: &'_ AtomicPtr<T>) -> Option<()> {
        todo!()
    }
}

// TODO: Implement the protect stack

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
