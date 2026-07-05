//! GTW-234 AC3 (dispatch path) — a VALID `MoveRequested` dispatch moves the actor to the
//! dest AND drops its Tu by EXACTLY the destination terrain's looked-up move cost (a
//! relation to the tuning leaf, never a pinned magnitude). Plus GTW-234 AC7 — the move
//! dispatch co-schedules with `sync_moved_gangers` in `SimSystems::Simulate`.

mod support;

mod bump_stop;
mod dispatch;
mod downed_block;
mod suppression;
