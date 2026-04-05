// We want to capture metrics for the in-memory usage of the storage engine
//
// - Memory
// - Timings/Latencies
// - Usage
//
// Because Arena is held by the Memtable, we do need to have a separate metrics file for Arena as Memtable uses Arena memory and can
// call into it to gather metrics on Arena memory usage

//------------------------------------

// TODO: Think of metric structure
