//! Shared fire-target fixture: the fire-target app, shooter / terrain / fog
//! authoring, and the highlight probe.

use bevy::{input::ButtonInput, platform::collections::HashSet, prelude::*};
use gdtf_battle_input::{GdtfBattleInputPlugin, InspectTarget, SelectedFireMode, SelectedShooter};
use gdtf_battle_presenter::{ActiveLevel, FireTargetHighlight, ViewMode};
use gdtf_battle_sim::{
    Aiming, BattleInProgress, CellLevel, Faction, FireMode, FireModeSpec, Handedness, Level,
    LifeState, Magazine, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
    OccupancyGrid, PlayerFaction, Position, ReloadTu, SquadVisibility, TerrainKind, Tu, TuMax,
    VerticalLinkGraph, WieldedBy, tuning::CombatTuning,
};

/// The faction the player controls (matches the inserted `PlayerFaction`).
pub(crate) const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from [`PLAYER_FACTION`]) — the fireable-target faction.
pub(crate) const ENEMY_FACTION: Faction = Faction::new(1);
/// The level the tests run on (the default `ActiveLevel`).
pub(crate) const LEVEL: Level = Level::new(0);

/// A `Single`-kind fire-mode spec with a marker `tu_percent` (arbitrary, not pinned tuning) — so
/// the expected cost is computed from `mode_tu_cost`, never asserted as a shipped magnitude.
pub(crate) const fn spec(tu_percent: f32) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(1),
    )
}

/// Builds the focused headless app: `MinimalPlugins` + the real `GdtfBattleInputPlugin`, plus the
/// resources `populate_fire_target` reads (all-Open occupancy, default tuning, the player
/// faction), the `BattleInProgress` gate witness, an empty fog (so the test marks enemy cells
/// VISIBLE explicitly), and the presenter-owned `FireTargetHighlight` + `ActiveLevel` (which the
/// presenter plugin would normally init — inserted here since this focused harness adds no
/// renderer plugin).
pub(crate) fn fire_target_app() -> App {
    let mut app = App::new();
    // `update_selection_highlight` spawns its reticle via `spawn_scene`, which needs an
    // `AssetServer` + the scene schedule (the GTW-322 spike requirement, the control.rs
    // precedent).
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ))
    .add_plugins(GdtfBattleInputPlugin);
    let w = app.world_mut();
    w.insert_resource(ActiveLevel::new(LEVEL));
    // GTW-521 — `dispatch_act_intents` also mutates the presenter-owned `ViewMode`.
    w.insert_resource(ViewMode::default());
    w.insert_resource(BattleInProgress);
    w.insert_resource(OccupancyGrid::default());
    w.insert_resource(VerticalLinkGraph::default());
    w.insert_resource(CombatTuning::default());
    w.insert_resource(PlayerFaction::new(PLAYER_FACTION));
    w.insert_resource(ButtonInput::<MouseButton>::default());
    w.insert_resource(FireTargetHighlight::cleared());
    // An empty fog — `place_enemy` marks the enemy cell VISIBLE so the fog gate passes.
    w.insert_resource(SquadVisibility::new(HashSet::default(), HashSet::default()));
    app
}

/// Spawns an armed, alive PLAYER-faction shooter at `cell` (carrying the `TuMax` / `Aiming` the
/// cost reads + a weapon entity with a magazine), selects it, and returns its entity + its TU max
/// / aiming so the test can compute the expected cost independently.
pub(crate) fn spawn_and_select_shooter(app: &mut App, cell: CellLevel) -> (Entity, TuMax, Aiming) {
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let ganger = app
        .world_mut()
        .spawn((
            PLAYER_FACTION,
            Position::new(cell),
            aiming,
            LifeState::Alive,
            Tu::new(255),
            tu_max,
        ))
        .id();
    app.world_mut().spawn((
        WieldedBy::new(ganger),
        Magazine::new(10, MagazineSize::new(30), ReloadTu::new(12)),
        // GTW-443: the fire surface's WeaponMagazine query reads `(&Magazine, &Handedness)`.
        Handedness::OneHanded,
    ));
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    (ganger, tu_max, aiming)
}

/// Spawns an armed, alive PLAYER-faction shooter at `cell` carrying a real
/// [`FireMode`]-bearing weapon entity (so the in-app `sync_fire_mode_on_select` resolves
/// `SelectedFireMode` off the weapon, the REAL runtime path — not an injected
/// `SelectedFireMode`), and selects it.
///
/// Reproduces the GTW-376 runtime ordering: the ganger is spawned and SELECTED FIRST, then
/// the wielded weapon entity (its `FireMode` + the `WieldedBy` back-reference) is spawned a
/// frame LATER, exactly as `setup_battle`'s deferred `queue_spawn_related_scenes::<Wields>`
/// applies after the GTW-255 auto-select has already flipped the selection. The fix's
/// `Added<FireMode>` re-trigger must then recover the mode (without this the mode stays at the
/// `tu_percent: 0` default and the cost reads "0 TU"). Returns the shooter entity, its TU max /
/// aiming, and the authored single-mode `tu_percent` so the test computes the expected cost
/// from the SAME inputs the resolved mode carries.
pub(crate) fn spawn_select_then_arm_late(
    app: &mut App,
    cell: CellLevel,
    single_tu_percent: f32,
) -> (Entity, TuMax, Aiming, f32) {
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let ganger = app
        .world_mut()
        .spawn((
            PLAYER_FACTION,
            Position::new(cell),
            aiming,
            LifeState::Alive,
            Tu::new(255),
            tu_max,
        ))
        .id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    // SELECT first — the selection-change frame, while the weapon's FireMode does not yet exist.
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    app.update();
    // The wielded weapon arrives LATE (the deferred-scene-spawn race): a real FireMode selector
    // (Single first, with the authored tu_percent) + the WieldedBy back-reference. The
    // WieldedBy hook populates the ganger's Wields collection on the next flush.
    app.world_mut().spawn((
        WieldedBy::new(ganger),
        FireMode::new(vec![spec(single_tu_percent)]),
        Magazine::new(10, MagazineSize::new(30), ReloadTu::new(12)),
        // GTW-443: the fire surface's WeaponMagazine query reads `(&Magazine, &Handedness)`.
        Handedness::OneHanded,
    ));
    // One update for the WieldedBy hook + the GTW-376 Added<FireMode> re-trigger to resolve the
    // mode off the now-present weapon.
    app.update();
    app.update();
    (ganger, tu_max, aiming, single_tu_percent)
}

/// Marks `cell` as shootable COVER (a BLOCKING `TerrainKind::Cover` marker in the occupancy
/// grid) and squad-VISIBLE — the GTW-377 fire-at-cover target state. No occupant is placed (a
/// cover cell is structure, not a ganger).
pub(crate) fn place_cover(app: &mut App, cell: CellLevel) {
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_terrain(cell, TerrainKind::Cover);
    mark_visible(app, cell);
}

/// Marks `cell` as bare FLOOR (an `Open` terrain marker, the default) and squad-VISIBLE — NOT a
/// fire target (an unoccupied, non-blocking cell is a MOVE destination, never a shot).
pub(crate) fn place_floor(app: &mut App, cell: CellLevel) {
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_terrain(cell, TerrainKind::Open);
    mark_visible(app, cell);
}

/// Places an ENEMY occupant at `cell` and marks the cell squad-VISIBLE (the realistic
/// fire-on-a-seen-enemy state), and returns its entity.
pub(crate) fn place_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(enemy));
    mark_visible(app, cell);
    enemy
}

/// Marks `cell` squad-VISIBLE (accruing onto the current fog).
pub(crate) fn mark_visible(app: &mut App, cell: CellLevel) {
    let mut visible: HashSet<CellLevel> = app
        .world()
        .get_resource::<SquadVisibility>()
        .map(|fog| fog.visible_cells().copied().collect())
        .unwrap_or_default();
    visible.insert(cell);
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible.clone(), visible));
}

/// Injects the live hovered cell (read by the populate system before the headless picker clobbers
/// it).
pub(crate) fn set_hovered(app: &mut App, cell: Option<CellLevel>) {
    app.world_mut().insert_resource(InspectTarget::new(cell));
}

/// Forces the selected fire mode.
pub(crate) fn set_fire_mode(app: &mut App, mode: FireModeSpec) {
    app.world_mut().insert_resource(SelectedFireMode::new(mode));
}

/// The current `FireTargetHighlight` after an update.
pub(crate) fn highlight(app: &App) -> FireTargetHighlight {
    *app.world().resource::<FireTargetHighlight>()
}
