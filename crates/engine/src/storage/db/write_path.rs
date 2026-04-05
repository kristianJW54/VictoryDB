//
//
//
//

pub(crate) trait WritePath {}

pub(crate) struct ArenaDirect {
    internalKeyBuffer: Vec<u8>, // FIXME: Change to actual type
}
impl WritePath for ArenaDirect {}

pub(crate) struct Buffered {}
impl WritePath for Buffered {}

#[cfg(feature = "arena_direct")]
pub(crate) type ArenaDirectWriter = Writer<ArenaDirect>;

#[cfg(feature = "buffered_key_writer")]
pub(crate) type BufferedWriter = Writer<Buffered>;

pub(crate) struct Writer<W: WritePath> {
    _path: std::marker::PhantomData<W>,
}
