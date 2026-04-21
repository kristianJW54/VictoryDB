//
//
//
//
//
use std::ptr::NonNull;

use mem::hazard::domain::Global;
use mem::hazard::hazard_ptr::HzdPtr;

use crate::column_family::cf::ColumnFamilyData;
use crate::memtable::memtable::{Immutable, Memtable, Mutable};

pub(crate) struct Superversion {
    cf: NonNull<ColumnFamilyData>, // Circular reference to parent
    mem: NonNull<Memtable<Mutable>>,
    imm: NonNull<Memtable<Immutable>>,
    // NOTE: Version
}

// SuperVersion Cache to be stored in Thread Local Storage which is effectively static for the lifetime of the programme
pub(crate) struct SVCache {
    pub(crate) hzd: HzdPtr<'static, Global>,
    pub(crate) generation: u64,
    pub(crate) sv: *const Superversion,
}

impl SVCache {
    pub(crate) fn new() -> Self {
        Self {
            hzd: HzdPtr::new(),
            generation: 0,
            sv: std::ptr::null(),
        }
    }

    // TODO: Implement safe methods to load and check generation number, return SV from

    // Methods operating or deferencing the ptr MUST use a pin()
}
