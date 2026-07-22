//! GTW-329 POINT-BLANK regression test: a shooter with an enemy in the
//! immediately-adjacent cell along its facing CAN strike that enemy.
//!
//! The bug: a shot at a point-blank (immediately-adjacent) enemy "never connects"
//! when the enemy is at a lower stance (crouching / prone). The march bands the §2
//! clearance test against the round's height only at the voxel ENTRY boundary. For a
//! steep, very-short shot the round's z changes a lot WITHIN one cell: a standing
//! shooter's high muzzle enters the adjacent cell still in the MID/HIGH band, clears
//! the lower-stanced enemy at that entry boundary, and then dives past it into the
//! ground — even though the round physically descends through the enemy's band before
//! it leaves the cell.
//!
//! The fix (`march/vector.rs` + `march/dda.rs`, GTW-329): band the per-voxel
//! clearance test against the LOWEST band the round occupies anywhere INSIDE the
//! voxel (the lower of its entry and exit bands — its height is monotone across a
//! cell), so a point-blank shot that dips into the enemy's band mid-cell connects.
//! Flat shots are unchanged (entry band == exit band).
//!
//! Two layers, both on the REAL code path:
//!  * a low-level `march_vector` sweep over every facing × shooter-stance ×
//!    target-stance point-blank combination (the precise mechanism — RED pre-fix for
//!    every lower-stanced target), and
//!  * the public `fire()` volley for the representative standing-vs-prone case (the
//!    end-to-end path the act layer drives).
//!
//! Both are DETERMINISTIC: a ZERO base spread collapses the cone to the central axis,
//! so whether the round connects is purely geometric, independent of the RNG stream.
//! Every `app.world_mut()` mutation is in a TEST BODY (`bevy-traps.md` #7 carve-out
//! (a)); no function here takes `&mut World` / `&World`.

mod harness;
mod march_connects;
mod volley_strike;
