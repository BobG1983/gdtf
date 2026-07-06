//! GTW-549 (child GTW-551 → GTW-17) — DATA-DRIVEN WEAPON ATTACHMENTS, the PHASE-5 example /
//! liveness sweep, proven on the REAL `setup_battle_on_request` → `BattleSimPlugin` spawn +
//! `apply_pending_attachments` post-spawn path. Complements `attachment_parse_effects`
//! (which covers Aim / Stability / `ExtraAmmo` / `ReloadTime` / Silence / identity / the
//! Silenced dual-producer gate); this file pins the remaining ticket clauses:
//!
//! - **`GainFireMode` adds a mode** — a `GainFireMode` attachment appends its `FireModeSpec` to
//!   the weapon's `FireMode` selector on the real spawn (the mode count grows by one).
//! - **Aim raises Accuracy** — the headline lever, re-asserted here against a distinctive inline
//!   baseline (a sight boosts AIM, not stability).
//! - **`ExtraAmmo` raises the magazine** — the capacity grows on the real spawn.
//! - **Silence still gates suppression** — a `Silence` attachment makes a point-blank shot
//!   produce NO `SuppressionApplied` where an identical un-silenced shot does (the PRESERVED
//!   producer gate), proving the effect wires the tag both producer gates read.
//! - **Empty-slots identity** — an empty `attachments` list spawns a weapon with no attachment
//!   effects (no added mode, no `Silenced`, un-raised Accuracy).
//! - **Hot-reload** — re-resolving an EDITED attachment spec through the registry changes the
//!   applied stat on the next spawn (the registry is the LIVE source of truth the loader's
//!   hot-reload rebuilds; app-side the `Modified` event drives the rebuild — asserted in
//!   `gdtf_app`'s `resolve::attachments` unit tests).
//!
//! Loader tests do NOT pin shipped magnitudes — every assert checks presence / relative
//! direction / count against a distinctive inline baseline, never a shipped number.

mod effects;
mod harness;
mod registry_edit;
mod suppression_gate;
