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
//! - [`reader`] — the GTW-302 slice-3 [`ShotFired`](gdtf_battle_sim::ShotFired) →
//!   floating-combat-text READER ([`read_shot_fired_text`]): it drains the per-round fire
//!   message, classifies each round's [`HitReport`](gdtf_battle_sim::HitReport) into the
//!   Phase-1 events derivable from it (HP damage, wound, graze, penetration verdict,
//!   DOWN / DEAD — a clean miss yields nothing), and spawns one pop per event through
//!   [`spawn_floating_text`].
//! - [`consequence`] — the GTW-302 slice-4 AUXILIARY-SIGNAL READER ([`read_consequence_fct`]):
//!   the Phase-1 pops NOT derivable from [`ShotFired`] alone but riding the dedicated
//!   consequence messages — `"Bleeding"` (AMBER) from [`Bleeding`](gdtf_battle_sim::Bleeding),
//!   `"Armor Broken"` (RED) from [`ArmorBroken`](gdtf_battle_sim::ArmorBroken). Reload pops
//!   and the numeric `"Armor -N"` are DEFERRED — no backing sim signal (see `consequence`'s
//!   module docs).
//!
//! Pure VIEW (ADR-0001): the primitive spawns + animates presenter entities only; it never
//! reads or writes the sim. [`animate_floating_text`], [`read_shot_fired_text`] and
//! [`read_consequence_fct`] are registered into the shared
//! [`PresenterSystems::Draw`](crate::PresenterSystems) band by `TopDownRendererPlugin`.

mod consequence;
mod palette;
mod reader;
mod text;

#[cfg(test)]
mod test;

pub use consequence::read_consequence_fct;
pub use palette::{FctValence, severity_color, valence_color};
pub use reader::read_shot_fired_text;
pub use text::{
    CombatText, FctEmphasis, FctRiseRate, FctStackIndex, FctTtlSeconds, FloatingCombatText,
    animate_floating_text, spawn_floating_text,
};
