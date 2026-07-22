//! The clippy-arg-count `SystemParam` read bundles ([`LeftClickReads`] / [`TurnReads`]) —
//! they change when a decision gains a resource read.

use bevy::prelude::*;
use gdtf_battle_sim::{
    battle::PlayerFaction, prelude::OccupancyGrid, tuning::CombatTuning,
    vertical::VerticalLinkGraph, visibility::SquadVisibility,
};

use crate::{InspectTarget, SelectedFireMode, selection::SelectedShooter};

/// The read-only resources [`decide_left_click`](super::decide_left_click) consults, grouped into ONE [`bevy::ecs::system::SystemParam`]
/// so a consuming system's parameter list stays under clippy's argument-count gate (the sim's
/// `BattleGridsParam` / the intent drain's `ActWriters` precedent).
///
/// Grouping the cohesive `Res<…>` reads into one param keeps
/// [`left_click_act`](crate::left_click_act) at five parameters. A transparent system-param
/// bundle of framework resources + landed newtypes — not itself a wrapped domain scalar.
///
/// The [`InspectTarget`] is DELIBERATELY NOT in this bundle (GTW-300): the click systems take it
/// as a [`ResMut<InspectTarget>`] (they WRITE its pin via [`apply_pin`](super::apply_pin)), and a `Res` of the same
/// resource in this bundle would be a `Res` + `ResMut` aliasing conflict (B0002). So
/// [`decide_left_click`](super::decide_left_click) / [`decide_pin`](super::decide_pin) receive the inspect target as a separate `&` argument.
#[derive(bevy::ecs::system::SystemParam)]
pub struct LeftClickReads<'w> {
    /// The mouse button state — the Left press edge gates the whole decision.
    pub(in crate::pointer::selection) mouse: Res<'w, ButtonInput<MouseButton>>,
    /// The coarse occupancy grid — the occupant + `is_blocked` reads.
    pub(super) occupancy:                    Res<'w, OccupancyGrid>,
    /// The selected fire mode — the FIRE branch's mode + the `can_fire` cone-mult.
    pub(super) fire_mode:                    Res<'w, SelectedFireMode>,
    /// The combat tuning the shared `can_fire` guard reads.
    pub(super) tuning:                       Res<'w, CombatTuning>,
    /// The player's own faction — the friend/foe gate for every branch.
    pub(super) player:                       Res<'w, PlayerFaction>,
    /// The vertical-link graph — the GTW-356 OQ-4 gate: a click on a vertical-link tile is
    /// NOT a move target (the player switches storey + clicks a destination tile; the route
    /// auto-stitches through the link).
    pub(super) links:                        Res<'w, VerticalLinkGraph>,
    /// The squad fog — the GTW-11 targeting-fog gate: a click on a cell that is NOT
    /// squad-VISIBLE refuses the fire commit (the new [`NoOp`](super::LeftClickOutcome::NoOp) rung
    /// before FIRE). Read as
    /// `Option<Res<…>>` so the decision FAILS CLOSED when the resource is absent (a harness
    /// with no sim plugin, or a frame before the battle seeds it) — a non-VISIBLE cell can
    /// never be fired into. The presenter's `cell_squad_visible` consumes it through the
    /// SAME shared read the reticle + hint use, so the three never disagree.
    pub(super) squad_visibility:             Option<Res<'w, SquadVisibility>>,
}

/// The read-only resources the turn-to-face surfaces consult, grouped into ONE [`bevy::ecs::system::SystemParam`]
/// so each surface's parameter list stays under clippy's argument-count gate.
///
/// Grouping the cohesive `Res<…>` reads (the hovered cell, the player faction, and the current
/// selection) into one param keeps
/// [`right_click_turn_to_face`](crate::right_click_turn_to_face) (mouse) and
/// [`gamepad_turn`](crate::gamepad::gamepad_turn) (East) at four parameters each. A transparent
/// system-param bundle REUSED by BOTH turn surfaces so they read the SAME inputs identically.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TurnReads<'w> {
    /// The inspect target — its LIVE hovered cell is the turn target (turning faces the cursor,
    /// independent of any panel pin).
    pub hovered:  Res<'w, InspectTarget>,
    /// The player's own faction — the turn surfaces act only on a player-faction selection.
    pub player:   Res<'w, PlayerFaction>,
    /// The current selection — the actor that turns.
    pub selected: Res<'w, SelectedShooter>,
}
