//! The shared index ↔ [`ModeKind`] ↔ label mapping of the fire-mode control — the one
//! ordering the spawn, press-listener, active-sync, visibility, and cost-line siblings
//! all reason against. Split out of the monolithic `mode_panel.rs` (GTW-583).

use gdtf_battle_sim::weapon::ModeKind;

/// The three fire-mode segment indices, in DISPLAY (left-to-right) order: Single / Burst /
/// Full. The index ↔ [`ModeKind`] mapping the press listener + the active-sync + the
/// visibility driver share.
pub(in crate::states::running::game::battlescape) const MODE_ORDER: [ModeKind; 3] =
    [ModeKind::Single, ModeKind::Burst, ModeKind::Full];

/// The segment INDEX of a [`ModeKind`] in [`MODE_ORDER`] (the active-sync direction).
pub(super) fn mode_index(kind: ModeKind) -> usize {
    MODE_ORDER.iter().position(|k| *k == kind).unwrap_or(0)
}

/// The [`ModeKind`] of a segment INDEX in [`MODE_ORDER`] (the press-listener direction),
/// or [`None`] for an out-of-range index (defensive; the control has exactly three).
pub(super) fn mode_for_index(index: usize) -> Option<ModeKind> {
    MODE_ORDER.get(index).copied()
}

/// The DISPLAYED firemode label for `kind` — `"single"` / `"burst"` for those modes (the
/// sim's canonical [`ModeKind`] [`Display`](std::fmt::Display) label), and the SHORTER
/// `"auto"` for [`ModeKind::Full`] (the GTW-298 presentation map: at the legible control
/// font the full sim label `"full-auto"` clipped in the narrow firemode cell, so it shows
/// as `"auto"`).
///
/// A presentation-only override local to this firemode control: it does NOT change the
/// sim's [`ModeKind`] [`Display`]. Single / Burst pass through unchanged.
pub(super) fn mode_label(kind: ModeKind) -> String {
    match kind {
        ModeKind::Full => "auto".to_owned(),
        other => other.to_string(),
    }
}
