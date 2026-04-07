//
//
//
// InnerKey is used as the underlying ptr storage and inline logic for keys. Different wrappers implementations will wrap inner key
// and provide inline const bounds with heap fallback
//

use std::ptr::{NonNull, null, null_mut};

use crate::key::{encode_into, internal_key::OperationType};

// Const INLINE sizes
#[cfg(target_pointer_width = "64")]
pub(super) const ITER_INLINE: usize = 39; // NOTE: Taken from RocksDB benchmarking proving the optimal inline size for iter keys is 39 bytes

#[cfg(target_pointer_width = "64")]
pub(super) const LOOKUP_INLINE: usize = 192;

pub(super) struct InnerKey<const INLINE: usize> {
    _inline: [u8; INLINE],
    len: usize,
    external: Option<NonNull<u8>>,
}

impl<const N: usize> InnerKey<N> {
    pub(super) fn new() -> Self {
        Self {
            _inline: [0u8; N],
            len: 0,
            external: None,
        }
    }

    pub(super) fn as_slice(&self) -> &[u8] {
        match self.external {
            Some(n) => unsafe { std::slice::from_raw_parts(n.as_ptr(), self.len) },
            None => self._inline[..self.len].as_ref(),
        }
    }

    pub(super) fn set_inline(&mut self, len: usize) {
        debug_assert!(len <= N);

        self.len = len;
        self.external = None;
    }

    // TODO: Does this need to be unsafe fn?
    pub(super) fn set_external(&mut self, len: usize, ptr: *mut u8) {
        self.len = len;
        // TODO: Need to add Safety comment + test that we are actual safe here
        self.external = unsafe { Some(NonNull::new_unchecked(ptr)) }
    }

    pub(super) fn encode_inline(&mut self, user_key: &[u8], seq_no: u64, op: OperationType) {
        let total = user_key.len() + 8;
        debug_assert!(total <= N);

        self.set_inline(total);
        encode_into(&mut self._inline[..total], user_key, seq_no, op);
    }
}
