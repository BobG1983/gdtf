//! The CONSEQUENCE-FAMILY palette (GTW-572): one file per consequence family — its marker,
//! its [`ConsequenceFct`](super::pop::ConsequenceFct) classify impl, and its unit tests.
//!
//! # Adding a consequence — the whole recipe
//!
//! 1. **One family file here**: a zero-sized marker + an `impl ConsequenceFct` (its
//!    `Signal` is the sim fact message; `classify` is the pure signal → pop mapping) + the
//!    family's classify unit tests IN THE SAME FILE (P11 — a new family edits no
//!    pre-existing test file beyond its registration line). Wire it in this `mod.rs`.
//! 2. **One registrar line** in `TopDownRendererPlugin`
//!    (`plugin/topdown/fx.rs::register_consequence_fct_families`):
//!    `app.add_consequence_fct::<YourFamily>()`. Nothing else — the generic reader, the
//!    shared per-frame stack counter, and the gates are already wired.
//! 3. **If it also logs** (a combat-log line): stay in THIS crate (GTW-620) — one
//!    [`CombatLogSource`](super::log_event::CombatLogSource) impl in
//!    `log_event/sources.rs`, one `CombatLogEvent` variant, one classify arm in
//!    [`classify_log_event`](super::log_event::classify_log_event), and one
//!    `add_combat_log_source` registrar line in
//!    `plugin/topdown/combat_log.rs`.
//!
//! The families: [`BleedingFct`] / [`ArmorBrokenFct`] (the two aux pops formerly fused in
//! one `consequence.rs` reader), [`InjuryFct`] (severity-ramp color), [`SuppressionFct`],
//! [`DotFct`], [`FieldFct`], and [`OnDeathFct`] (the one BOLD family). OUT of the palette
//! by design (P9): the shot pipeline (`reader.rs::classify_report` — the shared multi-pop
//! shot classifier) and the fall FX reader (`fx/fall.rs` — a bespoke flash + tween + one-shot
//! `"Fell"` pop, not a `ConsequenceFct` classify mapping). Both stay their own readers, but
//! both now claim their pop's stacking slot from the shared `FctSlotAllocator` (GTW-794).

mod armor_broken;
mod bleeding;
mod dot;
mod field;
mod injury;
mod on_death;
mod suppression;

#[cfg(test)]
mod test;

pub use armor_broken::ArmorBrokenFct;
pub use bleeding::BleedingFct;
pub use dot::DotFct;
pub use field::FieldFct;
pub use injury::InjuryFct;
pub use on_death::OnDeathFct;
pub use suppression::SuppressionFct;
