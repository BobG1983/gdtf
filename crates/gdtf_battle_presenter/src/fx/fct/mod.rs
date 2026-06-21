//! GTW-302 (slice 2): the reusable FLOATING-COMBAT-TEXT (FCT) primitive — the genre's
//! "this much damage, here" rise-and-fade number / tag over a battlefield cell.
//!
//! This module is the GENERIC primitive ONLY; it reads NO sim message. It owns two halves:
//!
//! - [`text`] — the [`spawn_floating_text`] helper (a caller picks the [`CombatText`]
//!   string, the [`Color`](bevy::prelude::Color), the `(Cell, Level)` world anchor, and a
//!   per-cell [`FctStackIndex`] so simultaneous pops fan out) and the
//!   [`animate_floating_text`] system that RISES + FADES + despawns every live pop. A pop
//!   is spawned ONCE carrying its own [`FloatingCombatText`] state and is MUTATED in place
//!   each frame until its [`FctTtlSeconds`] lifetime finishes — the same spawn-then-TTL
//!   shape as the transient FX flash, never respawned per frame.
//! - [`palette`] — the VALENCE → color mapping: [`FctValence`] (the presenter's own
//!   damage / wound / neutral / lethal category) → [`valence_color`], plus the
//!   [`Severity`](gdtf_battle_sim::Severity)-tier → amber-family ramp [`severity_color`].
//!   The reader slices (3-4) classify a [`ShotFired`](gdtf_battle_sim::ShotFired)
//!   consequence into a valence / severity and feed the resulting color to
//!   [`spawn_floating_text`].
//! - [`reader`] — the GTW-302 slice-3 / GTW-327 slice-2
//!   [`ShotFired`](gdtf_battle_sim::ShotFired) → floating-combat-text CLASSIFICATION
//!   ([`classify_report`](reader::classify_report) / [`anchor_cell`](reader::anchor_cell)): the
//!   shared, reusable functions that classify each round's
//!   [`HitReport`](gdtf_battle_sim::HitReport) into the Phase-1 events derivable from it (HP
//!   damage, wound, graze, penetration verdict, DOWN / DEAD — a clean miss yields nothing) and
//!   find their anchor cell. GTW-327 removed the immediate-spawn `read_shot_fired_text` system;
//!   [`spawn_shot_projectiles`](super::spawn_shot_projectiles) now calls these at
//!   projectile-spawn time and threads the pops THROUGH the staggered projectile → impact
//!   pipeline so each shot's numbers appear at its own impact.
//! - [`log_event`] — the GTW-328 slice-2 COMBAT-LOG classification
//!   ([`classify_log_event`](log_event::classify_log_event)): the shared, PURE function that
//!   turns a resolved [`CombatLogEvent`](log_event::CombatLogEvent) (the five Phase-1 combat
//!   events — fire declaration / movement / shot outcome / reload / turn boundary, with each
//!   [`Entity`](bevy::prelude::Entity) ALREADY resolved to a name) into the ordered
//!   [`LogLine`](log_event::LogLine)s the bottom-left HUD log renders. The shot outcome REUSES
//!   [`classify_report`](reader::classify_report) (its FCT callers untouched); a clean miss is
//!   shown (`"<name> missed"`), NOT suppressed.
//! - [`consequence`] — the GTW-302 slice-4 AUXILIARY-SIGNAL READER ([`read_consequence_fct`]):
//!   the Phase-1 pops NOT derivable from [`ShotFired`] alone but riding the dedicated
//!   consequence messages — `"Bleeding"` (AMBER) from [`Bleeding`](gdtf_battle_sim::Bleeding),
//!   `"Armor Broken"` (RED) from [`ArmorBroken`](gdtf_battle_sim::ArmorBroken). Reload pops
//!   and the numeric `"Armor -N"` are DEFERRED — no backing sim signal (see `consequence`'s
//!   module docs).
//!
//! Pure VIEW (ADR-0001): the primitive spawns + animates presenter entities only; it never
//! reads or writes the sim. [`animate_floating_text`] and [`read_consequence_fct`] are
//! registered into the shared [`PresenterSystems::Draw`](crate::PresenterSystems) band by
//! `TopDownRendererPlugin`; the per-shot FCT pops are now spawned at the impact by
//! [`animate_impact`](super::animate_impact) off the classification this module provides.

mod consequence;
mod log_event;
mod palette;
mod reader;
mod text;

#[cfg(test)]
mod test;

pub use consequence::read_consequence_fct;
pub use log_event::{CombatLogEvent, LogLine, LogName, classify_log_event};
pub use palette::{FctValence, severity_color, valence_color};
pub(super) use reader::{ClassifiedPop, anchor_cell, classify_report};
pub use text::{
    CombatText, FctEmphasis, FctStackIndex, FloatingCombatText, animate_floating_text,
    spawn_floating_text,
};
