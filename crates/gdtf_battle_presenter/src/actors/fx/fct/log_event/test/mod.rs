//! Unit tests for the combat-log classification layer — split by concern: the GTW-328
//! combat-event arms (fire / movement / reload / turn / rejection / injury) in [`events`],
//! the shot-outcome arms (`classify_report` reuse / miss / GTW-559 `None`-report guard /
//! GTW-386 structural lines) in [`shot_outcomes`], the GTW-572 C6 state-change arms
//! (fall / melee / death / suppression / armor-broken / the three affliction starts) in
//! [`state_changes`].

mod events;
mod shot_outcomes;
mod state_changes;
