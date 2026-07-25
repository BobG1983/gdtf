//! The on-demand snapshot service + the top-level [`BattleView`] assembly (GTW-738, the
//! T5 view path).
//!
//! [`build_snapshots`] is an ON-DEMAND query service, NOT a change-driven presenter and
//! NOT a continuously-updated cache: registered to run AFTER
//! [`SimSystems::Simulate`](gdtf_battle_sim::occupancy_sync::SimSystems::Simulate) (the
//! post-Simulate observation point), it drains the routed
//! [`GetBattleState`](gdtf_qa_protocol::envelope::QaRequest::GetBattleState) queue the T3
//! router fills and, only when a request is pending, reads the LIVE post-Simulate world
//! and assembles a fresh [`BattleView`]. A request routed at frame N is drained here the
//! SAME frame (the router pushes in the `InputSystems::Gather` band, which is ordered
//! before `SimSystems::Simulate`), so the reply reflects that frame's post-Simulate
//! state — never a stale one-frame-behind read.

use bevy::prelude::*;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    battle::PlayerFaction,
    metric::MAX_LEVELS,
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    turn::ActiveFaction,
    visibility::SquadVisibility,
};
use gdtf_qa_protocol::{
    envelope::QaResponse,
    ids::{DoorToken, EmplacementToken, GangerToken},
    view::{
        BattleView, DoorOpenNet, DoorView, EmplacementMannedNet, EmplacementView,
        ExploredCellCountNet, FogView, GridHeightNet, GridLevelsNet, GridSizeNet, GridWidthNet,
        SelectionView, TerrainSummaryView, TurnView, VisibleCellCountNet,
    },
};

use super::{
    ganger::ganger_views,
    map::{cell_level_net, faction_net},
    panel::panel_button_views,
    read::SnapshotWorld,
    ui_stack::{UiStackRead, ui_stack_view},
};
use crate::dev::net_qa::pending::{PendingQueue, SnapshotPayload};

/// Drain the routed [`SnapshotPayload`] queue and answer each pending
/// [`GetBattleState`](gdtf_qa_protocol::envelope::QaRequest::GetBattleState) with a fresh
/// [`BattleView`] read from the LIVE post-Simulate world (GTW-738).
///
/// Registered `.after(SimSystems::Simulate)` and gated on a live battle by the plugin. The
/// view is built ONCE per frame and only when a request is pending — an idle frame drains
/// nothing and touches no ganger/terrain query, so this is a request-driven service, not a
/// per-frame cache.
pub(in crate::dev::net_qa) fn build_snapshots(
    mut snapshots: ResMut<PendingQueue<SnapshotPayload>>,
    world: SnapshotWorld,
    ui_stack: UiStackRead,
) {
    let pending = snapshots.drain_ready();
    if pending.is_empty() {
        return;
    }
    let view = build_battle_view(&world, &ui_stack);
    for (_payload, responder) in pending {
        responder.reply(QaResponse::Battle(view.clone()));
    }
}

/// Assemble the whole [`BattleView`] from the live read surface: every living ganger card,
/// the terrain summary + token handout, the focus-navigable HUD button token handout, the
/// squad fog, the current selection, the turn state, and the DEV UI-stack swap harness's
/// state (GTW-816 — `None` in a build with no harness).
fn build_battle_view(world: &SnapshotWorld, ui_stack: &UiStackRead) -> BattleView {
    BattleView::new(
        ganger_views(world),
        terrain_view(world),
        panel_button_views(world),
        fog_view(&world.fog),
        selection_view(*world.selection),
        turn_view(*world.active, *world.player),
        ui_stack_view(ui_stack),
    )
}

/// Project the interactive terrain into its [`TerrainSummaryView`] — the coarse grid
/// dimensions plus the door / emplacement token handout.
///
/// The grid dimensions are the sim's STRUCTURAL coarse-grid extent
/// ([`GRID_WIDTH`] × [`GRID_HEIGHT`] × [`MAX_LEVELS`], the fixed battle space the presenter
/// renders over) — not a tunable balance value. Each door / emplacement entity is minted
/// into its wire token (`Entity::to_bits`), so a client can drive the open-door /
/// enter-/exit-emplacement intents against a token a view handed it.
fn terrain_view(world: &SnapshotWorld) -> TerrainSummaryView {
    let grid = GridSizeNet::new(
        GridWidthNet::new(grid_span(GRID_WIDTH)),
        GridHeightNet::new(grid_span(GRID_HEIGHT)),
        GridLevelsNet::new(MAX_LEVELS),
    );
    let doors = world
        .doors
        .iter()
        .map(|(entity, cell, state)| {
            DoorView::new(
                DoorToken::new(entity.to_bits()),
                cell_level_net(**cell),
                DoorOpenNet::new(*state.is_open()),
            )
        })
        .collect();
    let emplacements = world
        .emplacements
        .iter()
        .map(|(entity, cell, state)| {
            EmplacementView::new(
                EmplacementToken::new(entity.to_bits()),
                cell_level_net(**cell),
                EmplacementMannedNet::new(*state.is_occupied()),
            )
        })
        .collect();
    TerrainSummaryView::new(grid, doors, emplacements)
}

/// Project the squad fog into its [`FogView`] — the visible + explored `(cell, level)`
/// key COUNTS.
///
/// Counts, not the per-cell lists: an open map keeps thousands of cells visible and the
/// explored set grows every turn, so the lists overflowed a QA client's context (GTW-763).
/// The counts still answer the QA signal (fog limits sight when `visible_count` is below
/// the grid total) while keeping the snapshot a fixed size.
fn fog_view(fog: &SquadVisibility) -> FogView {
    FogView::new(
        VisibleCellCountNet::new(cell_count(fog.visible_cells().count())),
        ExploredCellCountNet::new(cell_count(fog.explored_cells().count())),
    )
}

/// Widen a `usize` fog cell count to the wire count newtypes' inner `u32`, saturating (the
/// `60×60×8` grid's `28_800`-cell ceiling always fits).
fn cell_count(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// Project the current selection into its [`SelectionView`] — the selected ganger's token,
/// or `None`.
fn selection_view(selected: SelectedShooter) -> SelectionView {
    SelectionView::new((*selected).map(|entity| GangerToken::new(entity.to_bits())))
}

/// Project the turn state into its [`TurnView`] — whose turn it is and which side the
/// player controls.
fn turn_view(active: ActiveFaction, player: PlayerFaction) -> TurnView {
    TurnView::new(faction_net(*active), faction_net(*player))
}

/// Widen a `usize` structural grid extent to the wire [`GridWidthNet`]/[`GridHeightNet`]
/// inner `u16`, saturating (the fixed `60`-cell spans always fit).
fn grid_span(extent: usize) -> u16 {
    u16::try_from(extent).unwrap_or(u16::MAX)
}
