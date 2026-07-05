//! Shared control fixture: the headless control app, fixture authoring, and
//! the `*Requested` message probes.

use bevy::{input::ButtonInput, prelude::*};
use gdtf_battle_input::{GdtfBattleInputPlugin, InspectTarget, SelectedFireMode, SelectedShooter};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    Aiming, BattleInProgress, CellLevel, Faction, FireMode, FireModeSpec, Handedness, Level,
    LifeState, Magazine, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
    OccupancyGrid, PlayerFaction, Position, ReloadTu, SquadVisibility, Tu, TuMax,
    VerticalLinkGraph, WieldedBy,
    acts::{FireRequested, MoveRequested, SetFacingRequested},
    tuning::CombatTuning,
};
use gdtf_test_utils::{MessageProbePlugin, probed};

/// The faction the player controls (matches the inserted `PlayerFaction`).
pub(crate) const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from [`PLAYER_FACTION`]) — the FIRE branch's valid target.
pub(crate) const ENEMY_FACTION: Faction = Faction::new(1);
/// The level all control tests run on.
pub(crate) const LEVEL: Level = Level::new(0);

// ---------------------------------------------------------------------------------
// Fixtures + harness.
// ---------------------------------------------------------------------------------

/// Builds the base headless control app: `MinimalPlugins` plus the
/// `GdtfBattleInputPlugin`, the presenter-owned `ActiveLevel`, the `BattleInProgress`
/// gate, an empty `OccupancyGrid`, `CombatTuning`, the `PlayerFaction` the decision gates
/// on, and an empty `ButtonInput<MouseButton>`. No synthetic camera is needed because the
/// click systems run before `pick_hovered_cell`, so an injected `InspectTarget` is read
/// before the headless picker clobbers it.
pub(crate) fn control_app() -> App {
    let mut app = App::new();
    // GTW-322: `update_selection_highlight` (in `GdtfBattleInputPlugin`) spawns its reticle
    // via `Commands::spawn_scene`, which PANICS under `MinimalPlugins` without an
    // `AssetServer` + the scene schedule (the spike-documented requirement).
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ))
    .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(ActiveLevel::new(LEVEL));
    // GTW-521 — the `dispatch_act_intents` drain now also mutates the presenter-owned
    // `ViewMode` (the full-view toggle target). The real app gets it from
    // `TopDownRendererPlugin`; this input-only harness inserts the default (DownToActive).
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    // GTW-356: the shared left-click decision reads `Res<VerticalLinkGraph>` (the OQ-4
    // link-tile non-target gate, via `LeftClickReads`), and `battle_act_gate()` now gates the
    // click systems on it — seed an empty graph so the click decision runs (these control tests
    // place no vertical links, so every move target is a non-link tile).
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());
    add_probes(&mut app);
    app
}

/// A `Single`-kind fire-mode spec with a marker `tu_percent` (arbitrary, not pinned
/// tuning).
pub(crate) const fn spec(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// Spawns an armed, alive, loaded, affordable PLAYER-faction shooter at `cell` (carrying
/// the firing components `can_fire` reads + a `Position` for right-click turn-to-face),
/// places it in the occupancy grid, and returns its entity.
pub(crate) fn spawn_player_shooter(app: &mut App, cell: CellLevel) -> Entity {
    let single = spec(0.2, 1);
    // The ganger carries its OWN vitals only — the weapon's FireMode + Magazine ride on a
    // related WEAPON entity (`Wields`, GTW-323 slice 3); the `WieldedBy` insert hook
    // populates the ganger's `Wields` synchronously in a bare `World` spawn.
    let ganger = app
        .world_mut()
        .spawn((
            PLAYER_FACTION,
            Position::new(cell),
            Aiming::new(false),
            LifeState::Alive,
            Tu::new(255),
            TuMax::new(100),
        ))
        .id();
    app.world_mut().spawn((
        WieldedBy::new(ganger),
        FireMode::new(vec![single]),
        Magazine::new(10, MagazineSize::new(30), ReloadTu::new(12)),
        // GTW-443: the fire surface's WeaponMagazine query reads `(&Magazine, &Handedness)`.
        Handedness::OneHanded,
    ));
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    ganger
}

/// Spawns an ENEMY-faction occupant at `cell` (only a `Faction` — the fire path reads
/// the SHOOTER's firing components, never the target's) and returns its entity.
///
/// GTW-11 — it ALSO marks `cell` squad-VISIBLE in the fog (inserting a `SquadVisibility` if
/// absent), the realistic battle state (you fire on a SEEN enemy): without this the new
/// targeting-fog rung in `decide_left_click` would refuse the fire (fail-closed on a cell absent
/// from the fog). The `NoOp` / non-fire control tests are unaffected (they assert no fire for
/// other reasons, which a visible cell does not change).
pub(crate) fn place_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(enemy));
    // Mark the enemy cell squad-VISIBLE so the fire commit passes the GTW-11 fog gate.
    let mut visible: bevy::platform::collections::HashSet<CellLevel> = app
        .world()
        .get_resource::<SquadVisibility>()
        .map(|fog| fog.visible_cells().copied().collect())
        .unwrap_or_default();
    visible.insert(cell);
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible.clone(), visible));
    enemy
}

/// Injects the `InspectTarget`'s live hovered cell (read by the click systems before the
/// headless picker clobbers it).
pub(crate) fn set_hovered(app: &mut App, cell: Option<CellLevel>) {
    app.world_mut().insert_resource(InspectTarget::new(cell));
}

/// Forces the current `SelectedShooter` to `entity` (the test-injected selection).
pub(crate) fn set_selection(app: &mut App, entity: Entity) {
    app.world_mut()
        .insert_resource(SelectedShooter::new(entity));
}

/// Forces a non-default fire mode so the FIRE branch has a mode to fire (the plugin
/// already inits a single-shot default; this is explicit for the fire-mode tests).
pub(crate) fn set_fire_mode(app: &mut App, mode: FireModeSpec) {
    app.world_mut().insert_resource(SelectedFireMode::new(mode));
}

/// The current `PathPreviewTarget` (the GTW-356 two-click move target the preview previews TO).
pub(crate) fn move_target(app: &App) -> Option<CellLevel> {
    app.world()
        .get_resource::<gdtf_battle_input::PathPreviewTarget>()
        .and_then(|t| **t)
}

/// The current `SelectedShooter`.
pub(crate) fn selected(app: &App) -> Option<Entity> {
    app.world()
        .get_resource::<SelectedShooter>()
        .and_then(|s| **s)
}

// ---------------------------------------------------------------------------------
// Message probes — each runs AFTER the drain, so it sees the same update's emission.
// ---------------------------------------------------------------------------------

/// Adds the generic message probes (GTW-576 `MessageProbePlugin<M>`) for the three act
/// `*Requested` messages these tests assert on — the `Last`-schedule drain observes the
/// same update's emissions with its own `MessageReader` cursor.
pub(crate) fn add_probes(app: &mut App) {
    app.add_plugins((
        MessageProbePlugin::<FireRequested>::default(),
        MessageProbePlugin::<MoveRequested>::default(),
        MessageProbePlugin::<SetFacingRequested>::default(),
    ));
}

/// The collected `FireRequested` messages.
pub(crate) fn fires(app: &App) -> Vec<FireRequested> {
    probed::<FireRequested>(app)
}

/// The collected `MoveRequested` messages.
pub(crate) fn moves(app: &App) -> Vec<MoveRequested> {
    probed::<MoveRequested>(app)
}

/// The collected `SetFacingRequested` messages.
pub(crate) fn facings(app: &App) -> Vec<SetFacingRequested> {
    probed::<SetFacingRequested>(app)
}
