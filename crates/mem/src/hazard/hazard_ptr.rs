//
//
//

use std::mem::ManuallyDrop;
use std::ptr;
use std::sync::atomic::Ordering;
use std::{marker::PhantomData, ptr::NonNull, sync::atomic::AtomicPtr};

use crate::hazard::domain::{Global, HzdDomain};

#[derive(Debug)]
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
pub struct HzdPtr<'domain, D = Global> {
    hazard: &'domain HzdPtrRec,
    pub(super) domain: &'domain HzdDomain<D>,
}

impl Default for HzdPtr<'static, Global> {
    fn default() -> Self {
        Self::new()
    }
}

impl HzdPtr<'static, Global> {
    pub fn new() -> Self {
        Self::new_in_domain(HzdDomain::global())
    }
    //

    pub fn many<const N: usize>() -> HzdPtrArray<'static, Global, N> {
        Self::many_in_domain(HzdDomain::global())
    }
}

impl<'domain, D> HzdPtr<'domain, D> {
    pub fn new_in_domain(domain: &'domain HzdDomain<D>) -> Self {
        Self {
            hazard: domain.acquire(),
            domain,
        }
    }

    pub fn many_in_domain<const N: usize>(
        domain: &'domain HzdDomain<D>,
    ) -> HzdPtrArray<'domain, D, N> {
        HzdPtrArray {
            hzd_ptr_array: domain.acquire_many::<N>().map(|ptr| {
                ManuallyDrop::new(HzdPtr {
                    hazard: ptr,
                    domain,
                })
            }),
        }
    }

    // ---------- Protect() ------------//
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

    /// protect() takes an Object T and attempts to insert it into a HzdPtrRec which has the effect of protecting the Object from reclemataion as
    /// other threads can see the Object in the HzdPtrRec in the Domain and will NOT reclaim whilst the HzdPtr is active
    ///
    /// The method seems to be a no-op but the main purpose is to ensure that the compiler checks the type signature of the lifetime
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

        super::asymmetric_light_barrier();

        let re_check_src = src.load(Ordering::Acquire);

        if !core::ptr::eq(ptr, re_check_src) {
            self.hazard.reset();
            Err(re_check_src)
        } else {
            Ok(NonNull::new(ptr).map(|ptr| (ptr, PhantomData)))
        }
    }

    pub fn reset_protection(&mut self) {
        self.hazard.reset();
    }
}

impl<T> Drop for HzdPtr<'_, T> {
    fn drop(&mut self) {
        self.hazard.reset();
        // Domain release
        todo!()
    }
}

pub struct HzdPtrArray<'domain, D, const N: usize> {
    // Manually dropped is used to prevent the HzdPtr inside from reclaiming itself, we implement a specific drop for
    // HzdPtrArray
    hzd_ptr_array: [ManuallyDrop<HzdPtr<'domain, D>>; N],
}

impl<const N: usize> Default for HzdPtrArray<'static, Global, N> {
    fn default() -> Self {
        HzdPtr::many::<N>()
    }
}

impl<'domain, D, const N: usize> HzdPtrArray<'domain, D, N> {
    //

    // This was a confusing one at first so I refer to the HapHazard docs:
    // https://github.com/jonhoo/haphazard/blob/main/src/hazard.rs#L292
    //
    // Essentially the compiler knows that the elements inside are individual elements based on the const array
    // so we mutably borrow each element inside instead of slicing in and returning SomeType(i) which the borrow checker
    // cannot assert is distinct from the other elements
    pub fn as_refs<'array>(&'array mut self) -> [&'array HzdPtr<'domain, D>; N] {
        self.hzd_ptr_array.each_mut().map(|v| &**v)
    }

    // protect_all goes through each source AtomicPtr<T> and protects it with the corresponding slot in the HzdPtrArray
    // the return is [Option; N] so that if protect() returns Null the index at that postion will be None
    pub fn protect_all<'hazard_object, T>(
        &'hazard_object mut self,
        mut sources: [&'_ AtomicPtr<T>; N],
    ) -> [Option<&'hazard_object T>; N]
    where
        T: Sync,
        D: 'static,
    {
        let mut output = [None; N];

        for (i, (hzdptr, src)) in self.hzd_ptr_array.iter_mut().zip(&mut sources).enumerate() {
            output[i] = unsafe { hzdptr.protect(src) }
        }

        output
    }

    //

    pub fn reset_protection(&mut self) {
        for ptr in self.hzd_ptr_array.iter_mut() {
            ptr.reset_protection();
        }
    }
}

// Drop implementation for HzdPtrArray

impl<D, const N: usize> Drop for HzdPtrArray<'_, D, N> {
    fn drop(&mut self) {
        self.reset_protection();
        let domain = self.hzd_ptr_array[0].domain;

        let each_ref = self.hzd_ptr_array.each_ref().map(|ptr| ptr.hazard);

        domain.release_many(each_ref)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn perfect_world() {

        // Create the API guide we want to use
    }
}
