// This file is derived from HapHazard (https://github.com/jonhoo/haphazard/tree/main)
// Copyright (c) Jon Gjengset
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Modifications have been made.

pub mod domain;
pub mod hazard_ptr;

#[cfg(target_arch = "x86_64")]
pub(super) fn asymmetric_light_barrier() {
    // memory ordering sufficient - keep normal Acquire/Release
}

#[cfg(not(target_arch = "x86_64"))]
pub(super) fn asymmetric_light_barrier() {
    crate::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
}

// Reclaim trait to implement for T so we get explicit Drop
pub trait Reclaim {}
impl<T> Reclaim for T {}

// This type represents UNIQUE OWNERSHIP of T
// and can:
// - transfer ownership → raw pointer
// - reconstruct ownership ← raw pointer
// exactly once
//
// Honestly, Box<T> will mostly be used but we don't want to hard code this into the logic so I am deferring to the HapHazard implementation of a Pointer<T> trait

pub unsafe trait Pointer<T>
where
    Self: Sized + core::ops::Deref<Target = T>,
{
    /// Reconstruct pointer type from the given ptr
    fn into_raw(self) -> *mut T;

    /// Reconstruct this pointer type from the given `ptr`.
    ///
    /// # Safety (for callers)
    ///
    /// 1. `ptr` must be a pointer returned by `Self::into_raw`
    /// 2. `ptr` must be valid to dereference to a `T`
    /// 3. `ptr` must not have been passed to `from_raw` since it was returned from `into_raw`
    /// 4. `ptr` must not be aliased
    unsafe fn from_raw(ptr: *mut T) -> Self;
}

// Here we implement for Box<T> which is what we'll mostly be using anyway

unsafe impl<T> Pointer<T> for Box<T> {
    fn into_raw(self) -> *mut T {
        Box::into_raw(self)
    }

    unsafe fn from_raw(ptr: *mut T) -> Self {
        unsafe { Box::from_raw(ptr) }
    }
}
