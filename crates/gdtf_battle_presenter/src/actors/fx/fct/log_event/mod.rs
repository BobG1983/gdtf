//! GTW-328 (slice 2) / GTW-572 (C5–C6): the SHARED combat-event → text CLASSIFICATION layer
//! for the battlescape combat-text LOG.
//!
//! The combat log is the bottom-left HUD strip of recent combat events that scroll up and
//! fade. This module owns the log's presenter-side vocabulary and phrasing:
//!
//! - [`event`] — [`CombatLogEvent`], a buffered [`Message`](bevy::prelude::Message)
//!   (GTW-572 C5): thin app-side per-source FORWARDER systems each drain ONE sim fact
//!   message, resolve every [`Entity`](bevy::prelude::Entity) to a [`LogName`] at that
//!   boundary, and write one resolved event; the ONE app-side APPENDER drains the event
//!   buffer, classifies through [`classify_log_event`], and spawns the lines. Adding a log
//!   source = one forwarder + one classify arm — the old seven-reader
//!   drain-loop/`.clear()` lock-step is gone.
//! - [`line`] — [`LogLine`], the one rendered-line value (text + swatch + emphasis).
//! - [`classify`] — [`classify_log_event`], the single PURE phrasing + palette layer (the
//!   ONE view-side match per family — GTW-572 P3). The shot outcome REUSES the FCT
//!   [`classify_report`](super::reader::classify_report) verbatim (never duplicated); a
//!   `None` report yields NO line (GTW-559 — a blast detonation seed is not a miss, and
//!   only the blast seed produces `None` on this path); a genuine clean miss is NEVER
//!   suppressed (`"<name> missed"`).
//!
//! GTW-572 C6 (the Q2 ruling — FINAL): the log covers ALL state changes. Falls, melee
//! damage, on-death kills, suppression-applied, and armor-broken each GAIN a line; the
//! DOT / field / bleed afflictions log ONCE at affliction start (their per-tick drain
//! signals never log). Where a sim signal lacked the line's data the SIM was extended with
//! a fact (never a rendered line): [`MeleeStruck`](gdtf_battle_sim::MeleeStruck),
//! [`DotAfflicted`](gdtf_battle_sim::DotAfflicted),
//! [`FieldAfflicted`](gdtf_battle_sim::FieldAfflicted),
//! [`BleedStarted`](gdtf_battle_sim::BleedStarted), and the ganger on
//! [`SuppressionApplied`](gdtf_battle_sim::SuppressionApplied).
//!
//! Pure VIEW (ADR-0001): presenter-owned phrasing + palette decisions over resolved data;
//! it reads no sim state and writes nothing.

mod classify;
mod event;
mod line;

#[cfg(test)]
mod test;

pub use classify::classify_log_event;
pub use event::{CombatLogEvent, InjuryLogText, LogName};
pub use line::LogLine;
