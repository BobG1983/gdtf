//! GTW-317 PASS-THROUGH-DEAD integration tests: a round passes THROUGH a corpse.
//!
//! The march now consults the real `is_dead` predicate threaded through
//! `resolve_coarse` (`fire/compose.rs::resolve_round` builds it from the TARGET
//! query's CURRENT `LifeState`). A ganger at [`LifeState::Dead`] is transparent: the
//! round flies on to the next blocker (next occupant / cover / wall / nothing). A
//! live occupant — including a [`LifeState::Downed`] one — still stops the round;
//! only `Dead` is skipped (`docs/combat/resolution.md` §9 corpse-skip discipline).
//!
//! These tests exercise the REAL `fire()` volley over the two disjoint queries with
//! an injected seeded [`ShotRng`]/[`SeverityRng`] pair (GTW-14; render-free, zero pixels), mirroring
//! `tests/landed_hit.rs` and the in-crate `fire/` tests. Two occupants are
//! placed in a straight East-facing line so a tight-cone burst flies (essentially)
//! the same ray each round; the front occupant's round-1 death is written to its
//! `LifeState` BEFORE round 2 runs, so round 2 reads the fresh corpse and passes
//! through it to the live occupant behind.
//!
//! Every `app.world_mut()` / `World` mutation is in a TEST BODY (`bevy-traps.md` #7
//! carve-out); no function here takes `&mut World` / `&World`.

mod downed_stop;
mod harness;
mod penetration;
