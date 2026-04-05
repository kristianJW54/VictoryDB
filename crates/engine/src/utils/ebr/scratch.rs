// This is scratch space for toying with different concepts such as MaybeUninit arrays, defer functions and other things.
use std::cell::Cell;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;
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
    // If we were to really progress this, we would have a singleton gc which would store Arc<Global> and register thread local handles per thread

    #[derive(Debug)]
    struct Global {
        global_epoch: AtomicU32,
        local_list: Mutex<Vec<Arc<Local>>>,
    }

    struct GC {
        global: Arc<Global>,
    }

    impl GC {
        fn register(&self) -> Arc<Local> {
            Local::register(self)
        }
    }

    fn gc() -> &'static GC {
        static GC: OnceLock<GC> = OnceLock::new();
        GC.get_or_init(|| GC {
            global: Arc::new(Global {
                global_epoch: AtomicU32::new(1),
                local_list: Mutex::new(Vec::new()),
            }),
        })
    }

    // Without getting into pointer semantics, I want to simply build out thread_local pinning and epoch advancement
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

    #[derive(Debug)]
    struct Local {
        local_epoch: AtomicU32,
        guarded: AtomicBool,
        // Would add gc_cache
        global: Arc<Global>,
    }

    impl Local {
        // Want to make a register which we can start a local from
        fn register(global: &GC) -> Arc<Self> {
            let s = Self {
                local_epoch: AtomicU32::new(0),
                guarded: AtomicBool::new(false),
                // Would add gc_cache
                global: global.global.clone(),
            };
            let local = Arc::new(s);
            global.global.local_list.lock().unwrap().push(local.clone());
            local
        }

        fn pin(&self) -> PinGuard {
            // With pin we need to do some checks here
            // We need to first take the global epoch value and insert it into the local epoch
            // then we flip the gaurded bool
            // finally we can check if we can increment the global epoch
            let mut global_epoch = self.global.global_epoch.load(Ordering::Acquire);
            self.local_epoch.store(global_epoch, Ordering::Release);
            self.guarded.store(true, Ordering::Release);
            let p = PinGuard {
                local: self as *const Local,
            };

            // This should not be done in the pin - it should be done lazily by global during a collect
            for local in self.global.local_list.lock().unwrap().iter() {
                println!("local {:?}", local);
                if local.guarded.load(Ordering::Acquire) {
                    let le = local.local_epoch.load(Ordering::Acquire);
                    if le < global_epoch {
                        return p;
                    }
                }
            }

            self.global
                .global_epoch
                .compare_exchange(
                    global_epoch,
                    global_epoch + 1,
                    Ordering::Release,
                    Ordering::Relaxed,
                )
                .unwrap_or(global_epoch);

            p
        }
    }

    struct PinGuard {
        local: *const Local,
    }

    thread_local!(
        // Because gc() uses OnceLock we can be sure that one instance of gc will be created and if there is one then we get that and use register()
        static LOCAL: Arc<Local> = gc().register()
    );

    fn pin() -> PinGuard {
        LOCAL.with(|local| local.pin())
    }

    impl Drop for Local {
        fn drop(&mut self) {
            println!("Dropped Local");
        }
    }

    thread::scope(|scope| {
        scope.spawn(|| {
            LOCAL
                .try_with(|handle| {
                    println!(
                        "local thread created - epoch: {:?}",
                        handle.local_epoch.load(Ordering::Relaxed)
                    );
                })
                .unwrap();
            // We should try to pin
            pin();
            // Check local pin
            LOCAL.with(|handle| {
                println!(
                    "local thread created - epoch: {:?}",
                    handle.local_epoch.load(Ordering::Relaxed)
                );
            });
        });
        scope.spawn(|| {
            LOCAL
                .try_with(|handle| {
                    println!(
                        "second local thread created - epoch: {:?}",
                        handle.local_epoch.load(Ordering::Relaxed)
                    );
                })
                .unwrap();
            // We should try to pin
            pin();
            // Check local pin
            LOCAL.with(|handle| {
                println!(
                    "second local thread created - epoch: {:?}",
                    handle.local_epoch.load(Ordering::Relaxed)
                );
            });
        });
    });

    // Global epoch should have increased?
    println!(
        "global epoch {:?}",
        LOCAL.with(|local| local.global.global_epoch.load(Ordering::Relaxed))
    );
}
