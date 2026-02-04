// This is scratch space for toying with different concepts such as MaybeUninit arrays, defer functions and other things.
use std::mem::MaybeUninit;

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
