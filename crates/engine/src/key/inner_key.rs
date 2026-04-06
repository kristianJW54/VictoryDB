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
    data: NonNull<u8>,
}

impl<const N: usize> InnerKey<N> {
    pub(super) fn new() -> Self {
        let mut inline = [0u8; N];
        let mut this = Self {
            _inline: inline,
            len: 0,
            data: NonNull::dangling(), // Temporarily dangling
        };

        let ptr = this._inline.as_mut_ptr();
        this.data = NonNull::new(ptr).unwrap();
        this
    }

    pub(super) fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.data.as_ptr(), self.len) }
    }

    pub(super) fn is_inlined(&self) -> bool {
        self._inline.as_ptr() == self.data.as_ptr()
    }

    pub(super) fn set_inline(&mut self, len: usize) {
        debug_assert!(len <= N);

        self.len = len;
        self.data = NonNull::new(self._inline.as_mut_ptr()).unwrap();
    }

    // TODO: Does this need to be unsafe fn?
    pub(super) fn set_external(&mut self, len: usize, ptr: *mut u8) {
        self.len = len;
        // TODO: Need to add Safety comment + test that we are actual safe here
        self.data = unsafe { NonNull::new_unchecked(ptr) }
    }

    pub(super) fn encode_inline(&mut self, user_key: &[u8], seq_no: u64, op: OperationType) {
        let total = user_key.len() + 8;
        debug_assert!(total <= N);

        self.set_inline(total);
        encode_into(&mut self._inline[..total], user_key, seq_no, op);
    }

    pub(super) fn encode_external(
        &mut self,
        ptr: *mut u8,
        user_key: &[u8],
        seq_no: u64,
        op: OperationType,
    ) {
        //
        self.set_external(user_key.len() + 8, ptr);
    }
}
