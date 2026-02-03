// This is scratch space for toying with different concepts such as MaybeUninit arrays, defer functions and other things.
use std::mem::MaybeUninit;

// It is important that we do not create references to something which implements drop or that can be dropped before asserting that what is being dropped is initialised memory and aligned etc
// It is therefore better to use -> & raw mut X <- in order to create a raw reference to the object which we can write to, after we can implement drop ourselves.

#[test]
fn basic_array() {
    const SIZE: usize = 10;

    // What do we gain by using MaybeUninit?
    // We have the memory and the layout already, we are allocating BUT we do NOT initialise or do any upfront work
    // This is deferred until we want to construct the object
    // So a 1_000_000 size array will not cause 1_000_000 immediate writes to memory

    let mut array = [MaybeUninit::<u8>::uninit(); SIZE];

    // We write to the first element
    unsafe {
        array[0].as_mut_ptr().write(2);
        let first_element = array[0].assume_init();
        println!("first element: {}", first_element);
    }

    // Let's try with a String which will drop?
    #[derive(Debug)]
    struct Tracked(String);

    impl Drop for Tracked {
        fn drop(&mut self) {
            println!("Dropping {}", self.0);
        }
    }

    {
        unsafe {
            let mut uninit = MaybeUninit::<Tracked>::uninit();

            // If i comment out this - the assume_init() will panic!
            // let s: &mut String = &mut *uninit.as_mut_ptr();
            // *s = "World".to_string();

            // This is correct
            uninit.write(Tracked("World".to_string()));

            // But if we immediately write a new string in place - we leak "World"
            // We should either drop_in_place (control belongs to us)
            // Or use assume_init (Control belongs to rust)
            //
            // uninit.as_mut_ptr().drop_in_place();
            //
            uninit.assume_init_drop();

            // Can I use raw ptr to write?
            let ptr = &raw mut *uninit.as_mut_ptr();
            ptr.write(Tracked("Earth".to_string()));

            // assume_init() also gives us drop -- so if we were to implement custom object handling in array - we may want to provide a different access
            // and implement drop ourselves?
            println!("string: {:?}", uninit.assume_init());

            // All writes require that the in place object be dropped or that the region is un-initialised
        }
    }
}
