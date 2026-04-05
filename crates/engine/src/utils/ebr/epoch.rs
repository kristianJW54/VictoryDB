// My EBR design for skiplists: (similar to crossbeam)
//
// - There is one Global epoch domain.
// - Each thread has a thread-local participant registered in a global intrusive list.
// - Pinning creates a Guard which points at the thread. The Guard increments a pin/guard
//   counter so the thread is considered "active" while any Guard is alive.
// - Data structures (skiplist, queue, etc.) take &Guard when they load/deref shared pointers.
//   This ensures any node that is still potentially reachable to a pinned thread is not reclaimed.
//
// - When the thread's pin count drops to zero, the participant becomes "inactive" (not pinned).
//   The thread usually remains registered in the global intrusive list until thread exit / unregister.
//
// - When a node/object is logically removed from a structure, it is "retired" and placed into a
//   deferred queue tagged with the current epoch.
// - Periodically, a thread computes the minimum announced epoch across all active participants.
//   Any retired objects with retire_epoch < min_active_epoch can be safely destroyed.
// - Destruction is performed by draining the deferred queue for those epochs (either locally,
//   or by pushing full queues onto a global queue that any thread can help execute).
//
// - This works even while the participant thread is alive and still doing work: retired objects
//   are reclaimed as soon as all active threads have advanced past their retirement epoch.
