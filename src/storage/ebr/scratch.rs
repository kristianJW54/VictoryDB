// This is scratch space for toying with different concepts such as MaybeUninit arrays, defer functions and other things.
use std::cell::Cell;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::thread;
// It is important that we do not create references to something which implements drop or that can be dropped before asserting that what is being dropped is initialised memory and aligned etc
// It is therefore better to use -> & raw mut X <- in order to create a raw reference to the object which we can write to, after we can implement drop ourselves.

#[test]
fn basic_array() {
    #[derive(Debug)]
    struct Tracked(String);

    impl Drop for Tracked {
        fn drop(&mut self) {
            println!("Dropping Tracked({:?})", self.0);
        }
    }

    const SIZE: usize = 10;

    // What do we gain by using MaybeUninit?
    // We have the memory and the layout already, we are allocating BUT we do NOT initialise or do any upfront work
    // This is deferred until we want to construct the object
    // So a 1_000_000 size array will not cause 1_000_000 immediate writes to memory

    let mut array: [MaybeUninit<Tracked>; SIZE] = std::array::from_fn(|_| MaybeUninit::uninit());

    // We write to the first element
    unsafe {
        // Here we allocate heap memory for "Hello" and array[0] stores the pointer + len + cap to the heap memory
        // Rust does not consider this a live value yet because it is still MaybeUninit
        array[0].write(Tracked("Hello".to_string()));

        // Here we assume init by taking the pointer bits and becoming the sole owner of the memory - array[0] still has the pointer bytes but they are useless
        // So we must ensure that assume_init_read is called once
        // let first_element = array[0].assume_init_read();
        // println!("first element: {:?}", first_element);
        // println!("first element: {:?}", first_element);
        // let first_element_2 = array[0].assume_init_read(); // UB

        // It is better maybe and more readable to use mem::replace
        let now = std::mem::replace(&mut array[0], MaybeUninit::uninit()).assume_init();
        println!("now?: {:?}", now);
        println!("array[0] {:?}", array[0]);
    }

    // assume_init copies out the pointer bits and replaces it with a MaybeUninit::uninit()
    // assume_init_read does the same but it does not replace the value - it effectively copies out the pointer bits making it the sole owner of the memory

    // Let's try to overwrite the first element.
    unsafe {
        array[0].as_mut_ptr().write(Tracked("World".to_string()));
        let first_element = array[0].assume_init_read();
        println!("first element: {:?}", first_element);
    }
}

#[test]
fn thread_local() {
    // I want to have a piece of data stored on the heap and be able to store refernce pointers in thread local stacks
    // Once a thread local has finished reading or if it wants to write to it we must ensure that the pointer is not dropped until other threads that might be
    // referencing it have unpinned it
    /*


     Thread Lifetime

    |----------------------------------------------------------------------------------------------------|
    | Pin Scope 1                          |          |                                                  |
    | |-------------------> END            | Unpinned |                                                  |
    |               Pin Scope 2            |  State   |                                                  |
    |               |----------------> END |          |                                                  |
    |                                      |  Reclaim | Pin Scope 3                                      |
    |                                      |          | |----------------> END                           |
    |----------------------------------------------------------------------------------------------------|

    Only inside the scope of a guard can a thread hold shared pointers

    Local epoch only advances once all guard pins == 0

    Thread lifetime
    ──────────────────────────────────────────────────────────────>

            ┌──────────── pinned region ────────────┐
            │                                       │
    [unpinned] ── pin() ──► [pinned] ── unpin() ──► [unpinned]
      epoch = 0              epoch = E               epoch = 0
                              (latched once)

    Legend:
    - epoch = 0        → thread is quiescent (not pinned)
    - epoch = E        → thread is pinned and advertises epoch E
    - nested pin()     → does NOT change epoch
    - epoch only updates when transitioning unpinned → pinned

      */

    // Start with a simple global epoch
    static GLOBAL: AtomicU32 = AtomicU32::new(1);

    // Without getting into pointer semantics - i want to simply have local threads incrementing their own epoch correctly
    // and in line with nested guard scopes
    //
    // Global epoch advancement:
    //
    // - The global epoch may advance from E → E+1 if NO thread is pinned with
    //   local_epoch < E.
    // - Pinned threads are allowed to lag behind the global epoch.
    // - A thread observes newer epochs only by UNPINNING and PINNING again.
    //
    // So all threads must have observed the current global epoch.
    //

    // Local here ->
}
