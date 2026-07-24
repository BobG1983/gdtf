//! GTW-302 (slice 2): the reusable FLOATING-COMBAT-TEXT (FCT) primitive — the genre's
//! "this much damage, here" rise-and-fade number / tag over a battlefield cell — plus the
//! GTW-572 CONSEQUENCE-POP PALETTE that feeds it.
//!
//! The module owns these halves:
//!
//! - [`text`] — the [`spawn_floating_text`] helper (a caller picks the [`CombatText`]
//!   string, the [`Color`](bevy::prelude::Color), the `(Cell, Level)` world anchor, and a
//!   per-cell [`FctStackIndex`] so simultaneous pops fan out) and the
//!   [`animate_floating_text`] system that RISES + FADES + despawns every live pop. A pop
//!   is spawned ONCE carrying its own [`FloatingCombatText`] state and is MUTATED in place
//!   each frame until its [`FctTtlSeconds`](super::tuning::FctTtlSeconds) lifetime finishes.
//! - [`palette`] — the VALENCE → color mapping: [`FctValence`] → [`valence_color`], plus the
//!   [`Severity`](gdtf_battle_sim::severity::Severity)-tier → amber-family ramp [`severity_color`].
//! - [`reader`] — the SHOT-side classification
//!   ([`classify_report`](reader::classify_report) / [`anchor_cell`](reader::anchor_cell)):
//!   the presenter's ONE exhaustive view-side dispatch over a round's
//!   [`HitReport`](gdtf_battle_sim::resolve_and_apply::HitReport), threaded THROUGH the staggered
//!   projectile → impact pipeline by [`spawn_shot_projectiles`](super::spawn_shot_projectiles)
//!   so each shot's numbers appear at its own impact. DELIBERATELY outside the consequence
//!   palette (GTW-572 P9): it is multi-pop, report-driven, and pipeline-threaded.
//! - the GTW-572 CONSEQUENCE PALETTE — [`pop`] (the [`ConsequencePop`] named pop struct, its
//!   [`PopAnchor`], and the [`ConsequenceFct`] trait), [`families`] (ONE file per consequence
//!   family: bleeding / armor-broken / injury / suppression / DOT / field / on-death — each
//!   holding its classify impl + unit tests, plus the add-one-consequence recipe in the
//!   module doc), and [`stacked_reader`] (the ONE generic
//!   [`read_consequence_fct`](stacked_reader::read_consequence_fct) reader + the
//!   [`ConsequenceFctAppExt::add_consequence_fct`] compile-time registrar that replaced the
//!   six hand-rolled reader clones and their registration walls).
//! - [`slot_allocator`] — the GTW-792 LIFETIME-AWARE stacking-slot allocator: the
//!   [`FctSlotAllocator`] [`SystemParam`](bevy::ecs::system::SystemParam) that counts the pops
//!   still ALIVE on a cell (via the [`FctAnchorCell`] every pop now carries) and returns the
//!   next free [`FctStackIndex`] above them — the shared primitive that fixes cross-pipeline /
//!   cross-frame slot collisions (its consumers land in GTW-793 / GTW-794).
//! - [`log_event`] — the COMBAT-LOG family (GTW-328 / GTW-572 C5 / GTW-620): the
//!   [`CombatLogEvent`] buffered [`Message`](bevy::prelude::Message) vocabulary, the
//!   per-source FORWARDERS that write it (the [`CombatLogSource`] impls + the
//!   [`CombatLogSourceAppExt`] registrar, moved down from `gdtf_app` in GTW-620 so a new
//!   log source is a presenter-only change), and the shared, PURE [`classify_log_event`]
//!   `gdtf_app`'s ONE appender drains through (ordered `.after` the exported
//!   [`CombatLogSystems::Forward`] set). The shot outcome REUSES
//!   [`classify_report`](reader::classify_report) (never duplicated); a `None` report
//!   yields NO line (GTW-559 — a blast detonation is not a miss).
//!
//! Pure VIEW (ADR-0001): everything here spawns/animates presenter entities or phrases
//! lines over resolved data; it never computes a combat outcome and never writes the sim.

mod families;
mod log_event;
mod palette;
mod pop;
mod reader;
mod slot_allocator;
mod stacked_reader;
mod text;

#[cfg(test)]
mod test;

pub use families::{
    ArmorBrokenFct, BleedingFct, DotFct, FieldFct, InjuryFct, OnDeathFct, SuppressionFct,
};
pub use log_event::{
    CombatLogEvent, CombatLogSource, CombatLogSourceAppExt, CombatLogSystems, InjuryLogText,
    LogLine, LogName, classify_log_event, forward_live_log_source, forward_log_source,
    forward_turn_started,
};
pub use palette::{FctValence, severity_color, valence_color};
pub use pop::{ConsequenceFct, ConsequencePop, PopAnchor};
pub(super) use reader::{ClassifiedPop, anchor_cell, classify_report};
pub use slot_allocator::{FctAnchorCell, FctSlotAllocator};
pub use stacked_reader::{
    ConsequenceFctAppExt, ConsequenceFctSystems, read_consequence_fct,
    register_consequence_fct_core,
};
pub use text::{
    CombatText, FctEmphasis, FctStackIndex, FloatingCombatText, animate_floating_text,
    spawn_floating_text,
};
