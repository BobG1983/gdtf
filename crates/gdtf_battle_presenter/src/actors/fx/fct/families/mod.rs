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
//!    (`plugin/renderer.rs::register_consequence_fct_families`):
//!    `app.add_consequence_fct::<YourFamily>()`. Nothing else — the generic reader, the
//!    shared per-frame stack counter, and the gates are already wired.
//! 3. **If it also logs** (a combat-log line): one forwarder impl in
//!    `gdtf_app`'s `combat_log/systems/sources.rs` (+ its one registrar line in the
//!    combat-log plugin) and one classify arm in
//!    [`classify_log_event`](super::log_event::classify_log_event).
//!
//! The families: [`BleedingFct`] / [`ArmorBrokenFct`] (the two aux pops formerly fused in
//! one `consequence.rs` reader), [`InjuryFct`] (severity-ramp color), [`SuppressionFct`],
//! [`DotFct`], [`FieldFct`], and [`OnDeathFct`] (the one BOLD family). OUT of the palette
//! by design (P9): the shot pipeline (`reader.rs::classify_report` — the shared multi-pop
//! shot classifier) and the fall FX reader (`fx/fall.rs` — glyph/shake FX, not a stacked
//! pop).

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
