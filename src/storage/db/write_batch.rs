// NOTE: We want to time operations using the env System timer
// BUT an optimisation could be that we time 1 of each operation type within a batch
// Example:
// - 5 PUTS = We time 1 PUT and assume the rest are similar
// - 5 DELETES = We time 1 DELETE and assume the rest are similar
