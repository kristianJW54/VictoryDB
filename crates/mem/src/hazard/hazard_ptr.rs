//
//
//

use std::ptr;
use std::sync::atomic::Ordering;
use std::{marker::PhantomData, ptr::NonNull, sync::atomic::AtomicPtr};

pub(super) struct HzdPtrRec {
    pub(super) ptr: AtomicPtr<u8>,
    pub(super) next: AtomicPtr<HzdPtrRec>,
    pub(super) available: AtomicPtr<HzdPtrRec>,
}

impl HzdPtrRec {
    pub(super) fn reset(&self) {
        self.ptr.store(ptr::null_mut(), Ordering::Release);
    }

    pub(super) fn protect(&self, ptr: *mut u8) {
        self.ptr.store(ptr, Ordering::Release);
    }
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
    hazard: HzdPtrRec,
    // domain: &'domain D,
    _f: PhantomData<D>,
    _l: PhantomData<&'domain ()>,
    ptr: AtomicPtr<u8>,
}

impl<'domain, D> HzdPtr<'domain, D> {
    pub(crate) fn make_hazard_ptr() -> Self {
        Self {
            // NOTE: To be replaced by an actual call to domain to retrieve a valid HzdRec (or new)
            hazard: HzdPtrRec {
                ptr: AtomicPtr::new(ptr::null_mut()),
                next: AtomicPtr::new(ptr::null_mut()),
                available: AtomicPtr::new(ptr::null_mut()),
            },
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

    - From folly/synchronization/HazptrHolder.h
        + Folly goes straight to try_protect() but also has boolean returns


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

    We can follow HapHazard here and make use of Option and Result for control flow branching


    - From haphazard//src/hazard.rs

    -> First method which handles control flow and loop

    pub fn protect_ptr<'l, T>(
            &'l mut self,
            src: &'_ AtomicPtr<T>,
        ) -> Option<(NonNull<T>, PhantomData<&'l T>)>


    -> Second core base method which handles atomic fencing and store/release of original relaxed load of ptr
       returns Error(*mut ptr) if orginal ptr does not match the src and returns what is in the src back to the control flow

    pub unsafe fn try_protect<'l, T>(
            &'l mut self,
            ptr: *mut T,
            src: &'_ AtomicPtr<T>,
        ) -> Result<Option<&'l T>, *mut T>

    */

    /// This high method's main purpose is to ensure that the compiler checks the type signature of the lifetime
    /// of the ptr we are tyring to protect
    ///
    /// It calls into two lower level methods
    ///  + protect_ptr()
    ///      + try_protect_ptr()
    ///
    /// T must be Sync as multiple threads can store ptr in the HzdRec which all threads have access to in the Domain linked list
    ///
    /// SAFTEY: This function is unsafe because the pointer returned can be null and it is up to the caller to ensure that the
    ///         AtomicPtr wanting to be protected is a valid ptr with a valid memory location and that the returned &T can be
    ///         dereferenced
    ///
    pub unsafe fn protect<'hazard_object, T>(
        &'hazard_object mut self,
        src: &'_ AtomicPtr<T>,
    ) -> Option<&'hazard_object T>
    where
        T: Sync,
        D: 'static,
    {
        let (ptr, _proof): (_, PhantomData<&'hazard_object T>) = self.protect_ptr(src)?;
        unsafe { Some(ptr.as_ref()) }
    }

    fn protect_ptr<'hazard_object, T>(
        &'hazard_object mut self,
        src: &'_ AtomicPtr<T>,
    ) -> Option<(NonNull<T>, PhantomData<&'hazard_object T>)> {
        //

        // Get relaxed because we'll double check in try_protect_ptr()
        let mut ptr = src.load(Ordering::Relaxed);
        loop {
            match self.try_protect_ptr(ptr, src) {
                Ok(None) => break None,
                Ok(Some((ptr, _ho))) => break Some((ptr, PhantomData)),
                Err(ptr2) => {
                    ptr = ptr2;
                }
            }
        }
    }

    fn try_protect_ptr<'hazard_object, T>(
        &'hazard_object mut self,
        ptr: *mut T,
        src: &'_ AtomicPtr<T>,
    ) -> Result<Option<(NonNull<T>, PhantomData<&'hazard_object T>)>, *mut T> {
        // Protect the ptr (we will only reset if we detect change)
        self.hazard.protect(ptr as *mut u8);

        Ok(None)
    }
}

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
        let mut hp: HzdPtr<crate::hazard::domain::Global> = HzdPtr::make_hazard_ptr();
        let ptr: &i32 = unsafe { hp.protect(&a).expect("Non Null") };

        println!("{:?}", ptr);

        // If we drop the hazard pointer, the protected object should only be reclaimed if safe
        // we should not be able to deference the hazard pointer after it has been dropped
        drop(hp);

        // Compile error
        // let _ = *ptr;
    }
}
