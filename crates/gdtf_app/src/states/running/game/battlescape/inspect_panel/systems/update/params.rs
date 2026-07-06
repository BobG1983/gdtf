//! The three [`SystemParam`] read/write bundles the inspect-panel repaint declares: its
//! own node lookups, the sim-resource reads, and the faction-tint inputs. Split out of
//! the monolithic `update.rs` (GTW-583); the repaint rationale lives on the parent
//! `update` module.

use bevy::{prelude::*, text::TextColor as UiTextColor};
use gdtf_battle_input::InspectTarget;
use gdtf_battle_sim::{
    battle::PlayerFaction, cover::CoverLedger, prelude::OccupancyGrid, visibility::SquadVisibility,
};
use gdtf_ui::{ProgressBarFill, theme::GdtfTheme};

use crate::states::running::game::battlescape::inspect_panel::components::{
    InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
    InspectObjectProtection, InspectObjectText, InspectPanelRoot, InspectStatBlockHost,
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
/// and ledger are [`Option`] (state-scoped — present only during a live battle,
/// `bevy-traps.md` #1). `pub(in …battlescape)` for `private_interfaces`.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::states::running::game::battlescape) struct InspectReads<'w> {
    /// The inspect target (`gdtf_battle_input`) — the panel reads its EFFECTIVE mode
    /// (pinned-else-hovered, GTW-300), NOT the raw cursor cell.
    pub target: Res<'w, InspectTarget>,
    /// The occupancy / terrain grid (occupant lookup + terrain kind).
    pub grid:   Option<Res<'w, OccupancyGrid>>,
    /// The cover ledger (a hovered object's seeded structural stats).
    pub ledger: Option<Res<'w, CoverLedger>>,
    /// The squad fog (GTW-378) — the targeting-fog gate read as `Option` so the verdict FAILS
    /// CLOSED when absent (the SAME fail-closed [`cell_squad_visible`](gdtf_battle_presenter::cell_squad_visible) the reticle + the
    /// fire-refusal use). A fog-hidden enemy occupant must NOT populate the panel (the
    /// info-leak); a BLOCKING wall / cover still inspects (map geometry / mission memory).
    pub squad:  Option<Res<'w, SquadVisibility>>,
    /// The player's own faction (GTW-378) — routes a hovered occupant's
    /// [`FactionRelation`](gdtf_battle_sim::visibility::FactionRelation) so [`cell_squad_visible`](gdtf_battle_presenter::cell_squad_visible) decides via `is_ganger_visible`
    /// (own-squad always visible, an enemy iff its cell is currently VISIBLE).
    pub player: Option<Res<'w, PlayerFaction>>,
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
