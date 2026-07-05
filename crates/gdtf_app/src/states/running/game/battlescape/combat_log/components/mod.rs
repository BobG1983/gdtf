//! Marker + state components for the battlescape combat-text LOG (GTW-328, slice 3,
//! bottom-left, ABOVE the weapon panel).
//!
//! The combat log is a battle-scoped `flex column` of recent-combat-event lines that scroll up
//! and fade. [`CombatLogRoot`] is the anchored container (the `OnExit` despawn + the per-update
//! append both find it by this marker); it also carries [`PanelHeightAnim`] (the GTW-328 slice-B
//! animated height that LERPS toward the natural content height instead of snapping). Each line
//! entity carries [`CombatLogLine`] (so the update can FIFO-despawn the oldest by spawn order +
//! count the visible lines) plus a [`LogLineFade`] holding its three-phase fade clock
//! (fade-in / hold / fade-out — the UI-space rise-free analogue of the presenter's world-space
//! [`FloatingCombatText`](gdtf_battle_presenter) shape) and a [`LineSlide`] holding its animated
//! position offset (the GTW-328 slice-B per-line slide that eases toward its target slot as the
//! stack reflows).
//!
//! UI/view only — these markers carry no combat rule; the lines are built from the sim's
//! combat-event messages via the shared
//! [`classify_log_event`](gdtf_battle_presenter::classify_log_event) classifier.

mod fade;
mod height;
mod markers;
mod slide;

// `combat_log/mod.rs`'s test_support ledger publicly re-exports these two markers
// through `components::`, so this re-export must widen in lockstep with their
// `support_item!` definitions (a capped re-export would hit E0365 under `test-support`).
crate::support_use!(markers::{CombatLogLine, CombatLogRoot};);

pub(crate) use fade::LogLineFade;
pub(crate) use height::PanelHeightAnim;
pub(crate) use slide::LineSlide;

#[cfg(test)]
mod test;
