//! [`BattleView`] — the top-level battle snapshot aggregate + its [`TurnView`]
//! (GTW-734).

use serde::{Deserialize, Serialize};

use super::{
    fog::FogView, ganger::GangerView, panel::PanelButtonView, selection::SelectionView,
    stat::FactionNet, terrain::TerrainSummaryView,
};

/// The turn **state** — which faction is active and which one the player controls.
///
/// A QA client reads whose turn it is (and which side is "us") off this. Serde default
/// shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TurnView {
    /// The faction whose turn is currently active.
    pub active: FactionNet,
    /// The faction the human player controls.
    pub player: FactionNet,
}

impl TurnView {
    /// Build a turn view from the active and player factions.
    #[must_use]
    pub const fn new(active: FactionNet, player: FactionNet) -> Self {
        Self { active, player }
    }
}

/// The whole battle **snapshot** — the reply to a
/// [`GetBattleState`](crate::envelope::QaRequest::GetBattleState).
///
/// Aggregates the curated read-model: every [`GangerView`], the terrain summary (with
/// its door / emplacement token handout), the focus-navigable HUD button token handout,
/// the player fog, the current selection, and the turn state. Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BattleView {
    /// Every ganger on the field, as a curated card.
    pub gangers:   Vec<GangerView>,
    /// The interactive terrain summary + door / emplacement token handout.
    pub terrain:   TerrainSummaryView,
    /// Every focus-navigable battlescape HUD button, with its focus token, Tab-chain
    /// ordinal, and label (GTW-789) — so a client can [`SetFocus`] onto a real panel
    /// button and drive / observe GTW-782's keyboard focus ring.
    ///
    /// [`SetFocus`]: crate::intent::NetIntent::SetFocus
    pub buttons:   Vec<PanelButtonView>,
    /// The player squad's fog-of-war snapshot.
    pub fog:       FogView,
    /// The current ganger selection.
    pub selection: SelectionView,
    /// The turn state.
    pub turn:      TurnView,
}

impl BattleView {
    /// Build a battle snapshot from its parts.
    #[must_use]
    pub const fn new(
        gangers: Vec<GangerView>,
        terrain: TerrainSummaryView,
        buttons: Vec<PanelButtonView>,
        fog: FogView,
        selection: SelectionView,
        turn: TurnView,
    ) -> Self {
        Self {
            gangers,
            terrain,
            buttons,
            fog,
            selection,
            turn,
        }
    }
}
