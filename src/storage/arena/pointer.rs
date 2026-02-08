// Custom arena pointers
// Used mainly to debug and enforce compile time safety
// We can track arena id and generations
// Also we can enforce deref to raw ptr takes a guard which we can make from in flight operations on the memtable
// meaning, storing the pointers is fine but using it must take a guard.
//
// This can be done by either combining the a custom AtomicPtr similar to crossbeam, which must take a guard when dereferencing.
// Or we can just make it so a guard is passed to any deref on this type

// Working design
struct ArenaPointer<T> {
    ptr: NonNull<T>, // Should T be a custom AtomicPtr similar to crossbeam
                     // arena id?
}

// When we deref this we would enforce the need for a guard
//
// // TODO: May not need custom arena pointers - so reason about this
