pub(crate) mod global;
pub(crate) mod guard;
pub(crate) mod local;

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
