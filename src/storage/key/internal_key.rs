//
//
//
//
//
//

// TODO: Finish the internal key logic
pub(crate) struct InternalKeyRef<'a>(&'a [u8]);

const INLINE_IK_SIZE: usize = 20;

//
// InternalKey is a temporary struct used for internal key operations. Mainly on read and search operations but the InternalKey can
// also be used by the writer on the write path if Arena Direct is not selected. This way a temp scratch buffer or inline key will be created on
// write operations also.
#[repr(C)]
pub(crate) struct InternalKey {
    len: u32,
    inline: [u8; INLINE_IK_SIZE],
    heap: Option<Box<[u8]>>,
}
