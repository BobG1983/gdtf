//! Unit tests for the combat-log classification layer — split by concern: the GTW-328
//! combat-event arms (fire / movement / shot outcome / reload / turn / rejection / injury)
//! in [`events`], the GTW-572 C6 state-change arms (fall / melee / death / suppression /
//! armor-broken / the three affliction starts) in [`state_changes`].

mod events;
mod state_changes;
