// handles logging order/cancel messages to allow state to be recovered in the case of a crash
// state is reconstructed by simulating activity up to that point


// logging should happen at orderbook level

// should we log messages and reconstruct, or log state periodically and restart from last message?
// maybe a mixed format where both are logged as "checkpoints"

// should be toggle-able via flag

// should only log successful trades/cancels

// should be completely deterministic