use std::marker::PhantomData;

pub(crate) struct DbImpl {
    _p: PhantomData<()>,
}

impl DbImpl {
    // Temp Methods to be replaced by full implementation
    pub(crate) fn rotate_mem() {}
}
