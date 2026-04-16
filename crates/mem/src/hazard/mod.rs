pub mod domain;
pub mod hazard_ptr;

mod note;
mod scratch;

#[cfg(target_arch = "x86_64")]
pub(super) fn asymmetric_light_barrier() {
    // memory ordering sufficient - keep normal Acquire/Release
}

#[cfg(not(target_arch = "x86_64"))]
pub(super) fn asymmetric_light_barrier() {
    crate::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
}
