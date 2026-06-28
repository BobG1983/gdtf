//! The DEV-ONLY procgen-visualizer STEP / AUTO controls (GTW-434 C1) — `Changed<Interaction>`
//! readers that advance the [`ProcgenViz`] model's reveal count.
//!
//! A press on the STEP button reveals ONE more prefab quad; a press on the AUTO button
//! reveals the WHOLE placement sequence at once. Both read `Changed<Interaction>` (the GTW-122
//! menu precedent — bevy-traps #6: `ui_focus_system` writes `Pressed`, so a `Changed` filter
//! fires only the frame the press lands), and both guard on the model resource existing
//! (`bevy-traps.md` #1). The draw layer ([`super::draw`]) then re-syncs the revealed quads.
//! The whole module is `#[cfg(debug_assertions)]`-gated by its parent.

use bevy::{prelude::*, ui::Interaction};

use crate::states::running::procgen_viz::{
    components::{AutoButton, StepButton},
    model::ProcgenViz,
};

/// Whether a [`bevy::ui::Interaction`] is a fresh press to act on (only
/// [`Pressed`](Interaction::Pressed) counts — the menu's `is_press` precedent).
const fn is_press(interaction: Interaction) -> bool {
    matches!(interaction, Interaction::Pressed)
}

/// STEP — on a fresh press of the STEP button, reveal ONE more prefab quad (C1).
///
/// Reads the STEP buttons' [`Interaction`](bevy::ui::Interaction) filtered
/// `Changed<Interaction>` so only the frame the press lands fires, and advances the
/// [`ProcgenViz`] model's reveal count via [`ProcgenViz::step`] (clamped at the sequence
/// length). Guarded `run_if(resource_exists::<ProcgenViz>)` by the scene plugin + an
/// `Option`-free `ResMut` (the resource is guaranteed present by the run condition).
/// Param-only (`bevy-traps.md` #7).
pub(in crate::states::running::procgen_viz) fn step_on_press(
    buttons: Query<&Interaction, (Changed<Interaction>, With<StepButton>)>,
    mut model: ResMut<ProcgenViz>,
) {
    if buttons.iter().copied().any(is_press) {
        model.step();
    }
}

/// AUTO — on a fresh press of the AUTO button, reveal EVERY prefab quad at once (C1).
///
/// Reads the AUTO buttons' [`Interaction`](bevy::ui::Interaction) filtered
/// `Changed<Interaction>` and runs the placement sequence to completion via
/// [`ProcgenViz::reveal_all`]. Guarded `run_if(resource_exists::<ProcgenViz>)`. Param-only
/// (`bevy-traps.md` #7).
pub(in crate::states::running::procgen_viz) fn auto_on_press(
    buttons: Query<&Interaction, (Changed<Interaction>, With<AutoButton>)>,
    mut model: ResMut<ProcgenViz>,
) {
    if buttons.iter().copied().any(is_press) {
        model.reveal_all();
    }
}
