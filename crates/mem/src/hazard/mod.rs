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
// all credit to HapHazard: https://github.com/jonhoo/haphazard/blob/main/src/pointer.rs#L25

pub unsafe trait Pointer<T>
where
    Self: Sized + core::ops::Deref<Target = T>,
{
    fn into_raw(self) -> *mut T;
}
