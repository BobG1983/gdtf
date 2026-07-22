//! The three [`SystemParam`] read/write bundles the inspect-panel repaint declares: its
//! own node lookups, the sim-resource reads, and the faction-tint inputs. Split out of
//! the monolithic `update.rs` (GTW-583); the repaint rationale lives on the parent
//! `update` module.

use bevy::{prelude::*, text::TextColor as UiTextColor};
use gdtf_battle_input::InspectTarget;
use gdtf_battle_presenter::ShownSquadVisibility;
use gdtf_battle_sim::{
    battle::PlayerFaction, cover::CoverLedger, prelude::OccupancyGrid, visibility::SquadVisibility,
};
use gdtf_ui::{ProgressBarFill, theme::GdtfTheme};

use crate::states::running::game::battlescape::inspect_panel::{
    components::{
        InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
        InspectObjectProtection, InspectObjectText, InspectPanelRoot, InspectStatBlockHost,
    },
    shadow::{ShownCoverLedger, ShownOccupancyGrid},
};

/// The read-only marker→entity lookups the inspect panel needs to find its own nodes.
///
/// A [`SystemParam`] bundle so the update system declares them as one param. All read-only
/// `Query<Entity, With<…>>` (the markers are disjoint), so no conflict with the
/// [`StatBlockWidgets`](crate::states::running::game::battlescape::stat_block::StatBlockWidgets) write bundle. `pub(in …battlescape)` for `private_interfaces`.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::states::running::game::battlescape) struct InspectNodes<'w, 's> {
    /// The panel root (whole-panel visibility toggle).
    pub root:         Query<'w, 's, Entity, With<InspectPanelRoot>>,
    /// The shared stat-block host (the ganger sub-block show/hide target).
    pub host:         Query<'w, 's, Entity, With<InspectStatBlockHost>>,
    /// The object block container (the object sub-block show/hide target).
    pub object_block: Query<'w, 's, Entity, With<InspectObjectBlock>>,
    /// The object block's **title** `Text` (object kind heading).
    pub object_title: Query<'w, 's, Entity, With<InspectObjectText>>,
    /// The object block's **Hardness** `Text` line.
    pub hardness:     Query<'w, 's, Entity, With<InspectObjectHardness>>,
    /// The object block's **Protection** `Text` line.
    pub protection:   Query<'w, 's, Entity, With<InspectObjectProtection>>,
    /// The object block's **Height band** `Text` line.
    pub height:       Query<'w, 's, Entity, With<InspectObjectHeight>>,
    /// The object block's integrity `ProgressBar` track.
    pub object_bar:   Query<'w, 's, Entity, With<InspectObjectBar>>,
    /// The sub-blocks' layout `Node` writer — drives `Display::Flex` / `Display::None` so a
    /// hidden sub-block is REMOVED from layout (GTW-295). Filtered `Without<ProgressBarFill>`
    /// so it stays disjoint from the [`StatBlockWidgets`](crate::states::running::game::battlescape::stat_block::StatBlockWidgets) `fills` `&mut Node` writer.
    pub display:      Query<'w, 's, &'static mut Node, Without<ProgressBarFill>>,
}

/// The sim-resource reads the inspect panel resolves its EFFECTIVE target against (GTW-300).
///
/// A [`SystemParam`] bundle so the update system declares the inspect target + grid + ledger
/// reads as ONE param (the [`too_many_arguments`](clippy::too_many_arguments) idiom). The grid
/// / ledger / fog reads are the CURSOR-TIME SHADOWS (GTW-762), NOT the live sim resources:
/// during closed-gate playback each shadow is frozen at what the cursor has shown, so the
/// panel describes the cursor's playback position rather than a move / hit / reveal the sim
/// has already applied but the view has not yet played. Each is [`Option`] (present only when
/// the battle / presenter registered it, `bevy-traps.md` #1); read them through the
/// [`occupancy`](InspectReads::occupancy) / [`cover`](InspectReads::cover) /
/// [`fog`](InspectReads::fog) accessors. `pub(in …battlescape)` for `private_interfaces`.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::states::running::game::battlescape) struct InspectReads<'w> {
    /// The inspect target (`gdtf_battle_input`) — the panel reads its EFFECTIVE mode
    /// (pinned-else-hovered, GTW-300), NOT the raw cursor cell.
    pub target: Res<'w, InspectTarget>,
    /// The cursor-time occupancy / terrain grid snapshot (occupant lookup + terrain kind) —
    /// read through [`occupancy`](InspectReads::occupancy).
    pub grid:   Option<Res<'w, ShownOccupancyGrid>>,
    /// The cursor-time cover-ledger snapshot (a hovered object's seeded structural stats) —
    /// read through [`cover`](InspectReads::cover).
    pub ledger: Option<Res<'w, ShownCoverLedger>>,
    /// The cursor-time squad-fog snapshot (GTW-378 / GTW-762) — the targeting-fog gate read as
    /// `Option` so the verdict FAILS CLOSED when absent (the SAME fail-closed
    /// [`cell_squad_visible`](gdtf_battle_presenter::cell_squad_visible) the reticle + the
    /// fire-refusal use). A fog-hidden enemy occupant must NOT populate the panel (the
    /// info-leak); a BLOCKING wall / cover still inspects (map geometry / mission memory).
    /// Read through [`fog`](InspectReads::fog).
    pub squad:  Option<Res<'w, ShownSquadVisibility>>,
    /// The player's own faction (GTW-378) — routes a hovered occupant's
    /// [`FactionRelation`](gdtf_battle_sim::visibility::FactionRelation) so [`cell_squad_visible`](gdtf_battle_presenter::cell_squad_visible) decides via `is_ganger_visible`
    /// (own-squad always visible, an enemy iff its cell is currently VISIBLE).
    pub player: Option<Res<'w, PlayerFaction>>,
}

impl InspectReads<'_> {
    /// The cursor-time occupancy grid the panel resolves occupants / terrain against, or
    /// [`None`] when the shadow is not registered (a harness without the battle) — the
    /// flattened read over the [`ShownOccupancyGrid`] shadow (GTW-762).
    #[must_use]
    pub(in crate::states::running::game::battlescape) fn occupancy(
        &self,
    ) -> Option<&OccupancyGrid> {
        self.grid.as_deref().map(ShownOccupancyGrid::grid)
    }

    /// The cursor-time cover ledger a hovered object's stats seed from, or [`None`] when the
    /// shadow is not registered — the flattened read over the [`ShownCoverLedger`] shadow
    /// (GTW-762).
    #[must_use]
    pub(in crate::states::running::game::battlescape) fn cover(&self) -> Option<&CoverLedger> {
        self.ledger.as_deref().map(ShownCoverLedger::ledger)
    }

    /// The cursor-time squad fog the panel's occupant gate reads, or [`None`] when the shadow
    /// is not registered (fail-closed) — the flattened read over the [`ShownSquadVisibility`]
    /// shadow (GTW-762).
    #[must_use]
    pub(in crate::states::running::game::battlescape) fn fog(&self) -> Option<&SquadVisibility> {
        self.squad.as_deref().map(ShownSquadVisibility::visibility)
    }
}

/// The faction-tint reads the inspect panel needs to recolor the name line by the hovered
/// ganger's allegiance (AC2).
///
/// A [`SystemParam`] bundle so the update system declares the tint inputs as ONE param
/// (the [`too_many_arguments`](clippy::too_many_arguments) idiom — system analogue of the
/// ctor-struct fix). Both resources are [`Option`] (state-scoped — present only during a
/// live battle, `bevy-traps.md` #1); the [`UiTextColor`](bevy::text::TextColor) writer is
/// disjoint from every [`StatBlockWidgets`](crate::states::running::game::battlescape::stat_block::StatBlockWidgets) query (a distinct component), so no conflict.
/// `pub(in …battlescape)` for `private_interfaces`.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::states::running::game::battlescape) struct FactionTint<'w, 's> {
    /// The player's faction — the hovered ganger is an ENEMY when its [`Faction`](gdtf_battle_sim::ganger::Faction) differs.
    pub player: Option<Res<'w, PlayerFaction>>,
    /// The runtime theme — its body-text color is the player (normal) name color.
    pub theme:  Option<Res<'w, GdtfTheme>>,
    /// The name-line [`TextColor`](bevy::text::TextColor) writer (the tint target).
    pub colors: Query<'w, 's, &'static mut UiTextColor>,
}
