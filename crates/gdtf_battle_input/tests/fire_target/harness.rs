//! authoring, and the highlight probe.

use bevy::{input::ButtonInput, platform::collections::HashSet, prelude::*};
use gdtf_battle_input::{
    ChosenFireMode, GdtfBattleInputPlugin, InspectTarget, SelectedShooter, chosen_spec,
};
use gdtf_battle_presenter::{ActiveLevel, FireTargetHighlight, ViewMode};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::{Aiming, Facing, TuMax},
    magazine::{LoadedRounds, Magazine, ReloadTu},
    occupancy::TerrainKind,
    prelude::{
        BattleInProgress, CellLevel, Direction, Faction, Level, LifeState, OccupancyGrid, Position,
        Tu,
    },
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::SquadVisibility,
    weapon::{
        FireMode, FireModeSpec, Handedness, MagazineSize, MeleeWeapon, ModeConeMult, ModeKind,
        ModeShots, ModeTuPercent, MountedWeapon, WieldedBy, Wields,
    },
};

pub(crate) const PLAYER_FACTION: Faction = Faction::new(0);
pub(crate) const ENEMY_FACTION: Faction = Faction::new(1);
pub(crate) const LEVEL: Level = Level::new(0);
pub(crate) const SHOOTER_FACING: Facing = Facing::new(Direction::East);

/// What the gun a shooter is spawned holding charges, until a case rewrites it.
pub(crate) const ARMED_TU_PERCENT: f32 = 0.2;

pub(crate) const fn spec(tu_percent: f32) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(1),
    )
}

pub(crate) fn fire_target_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ))
    .add_plugins(GdtfBattleInputPlugin);
    let w = app.world_mut();
    w.insert_resource(ActiveLevel::new(LEVEL));
    w.insert_resource(ViewMode::default());
    w.insert_resource(BattleInProgress);
    w.insert_resource(OccupancyGrid::default());
    w.insert_resource(VerticalLinkGraph::default());
    w.insert_resource(CombatTuning::default());
    w.insert_resource(PlayerFaction::new(PLAYER_FACTION));
    w.insert_resource(ButtonInput::<MouseButton>::default());
    w.insert_resource(ButtonInput::<KeyCode>::default());
    w.insert_resource(FireTargetHighlight::cleared());
    w.insert_resource(SquadVisibility::new(HashSet::default(), HashSet::default()));
    app
}

pub(crate) fn spawn_and_select_shooter(app: &mut App, cell: CellLevel) -> (Entity, TuMax, Aiming) {
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let ganger = app
        .world_mut()
        .spawn((
            PLAYER_FACTION,
            Position::new(cell),
            SHOOTER_FACING,
            aiming,
            LifeState::Alive,
            Tu::new(255),
            tu_max,
        ))
        .id();
    app.world_mut().spawn((
        WieldedBy::new(ganger),
        FireMode::new(vec![spec(ARMED_TU_PERCENT)]),
        Magazine::new(
            LoadedRounds::new(10),
            MagazineSize::new(30),
            ReloadTu::new(12),
        ),
        Handedness::OneHanded,
    ));
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    (ganger, tu_max, aiming)
}

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
            SHOOTER_FACING,
            aiming,
            LifeState::Alive,
            Tu::new(255),
            tu_max,
        ))
        .id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    app.update();
    app.world_mut().spawn((
        WieldedBy::new(ganger),
        FireMode::new(vec![spec(single_tu_percent)]),
        Magazine::new(
            LoadedRounds::new(10),
            MagazineSize::new(30),
            ReloadTu::new(12),
        ),
        Handedness::OneHanded,
    ));
    app.update();
    app.update();
    (ganger, tu_max, aiming, single_tu_percent)
}

pub(crate) fn place_cover(app: &mut App, cell: CellLevel) {
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_terrain(cell, TerrainKind::Cover);
    mark_visible(app, cell);
}

pub(crate) fn place_floor(app: &mut App, cell: CellLevel) {
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_terrain(cell, TerrainKind::Open);
    mark_visible(app, cell);
}

pub(crate) fn place_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(enemy));
    mark_visible(app, cell);
    enemy
}

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

pub(crate) fn set_hovered(app: &mut App, cell: Option<CellLevel>) {
    app.world_mut().insert_resource(InspectTarget::new(cell));
}

/// The weapon the selected shooter fires, if anything is selected and holds one.
pub(crate) fn selected_gun(app: &App) -> Option<Entity> {
    let shooter = app
        .world()
        .get_resource::<SelectedShooter>()
        .and_then(|selected| **selected)?;
    let wields = app.world().get::<Wields>(shooter)?;
    wields.firing_weapon(
        |weapon| {
            app.world()
                .get_entity(weapon)
                .is_ok_and(|row| row.contains::<MountedWeapon>())
        },
        |weapon| {
            app.world()
                .get_entity(weapon)
                .is_ok_and(|row| row.contains::<MeleeWeapon>())
        },
    )
}

/// Put `mode` on the gun the selected shooter fires and pick it. A no-op when it holds none.
pub(crate) fn set_fire_mode(app: &mut App, mode: FireModeSpec) {
    let Some(weapon) = selected_gun(app) else {
        return;
    };
    app.world_mut()
        .entity_mut(weapon)
        .insert((FireMode::new(vec![mode]), ChosenFireMode::new(mode.kind)));
}

/// The spec the gun the selected shooter fires is set to.
pub(crate) fn resolved_mode(app: &App) -> Option<FireModeSpec> {
    let weapon = selected_gun(app)?;
    let modes = app.world().get::<FireMode>(weapon)?;
    chosen_spec(modes, app.world().get::<ChosenFireMode>(weapon))
}

pub(crate) fn highlight(app: &App) -> FireTargetHighlight {
    *app.world().resource::<FireTargetHighlight>()
}
