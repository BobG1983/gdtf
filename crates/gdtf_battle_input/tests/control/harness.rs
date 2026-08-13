//! Shared control fixture: the headless control app, fixture authoring, and
use bevy::{input::ButtonInput, prelude::*};
use gdtf_battle_input::{GdtfBattleInputPlugin, InspectTarget, SelectedShooter};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    acts::{FireRequested, MoveRequested, SetFacingRequested},
    battle::PlayerFaction,
    ganger::{Aiming, TuMax},
    magazine::{LoadedRounds, Magazine, ReloadTu},
    prelude::{
        BattleInProgress, CellLevel, Faction, Level, LifeState, OccupancyGrid, Position, Tu,
    },
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::SquadVisibility,
    weapon::{
        FireMode, FireModeSpec, Handedness, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, WieldedBy,
    },
};
use gdtf_test_utils::{MessageProbePlugin, probed};

pub(crate) const PLAYER_FACTION: Faction = Faction::new(0);
pub(crate) const ENEMY_FACTION: Faction = Faction::new(1);
pub(crate) const LEVEL: Level = Level::new(0);

pub(crate) fn control_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ))
    .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(ActiveLevel::new(LEVEL));
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());
    app.world_mut()
        .insert_resource(ButtonInput::<KeyCode>::default());
    add_probes(&mut app);
    app
}

pub(crate) const fn spec(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

pub(crate) fn spawn_player_shooter(app: &mut App, cell: CellLevel) -> Entity {
    let single = spec(0.2, 1);
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
    ganger
}

pub(crate) fn place_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(enemy));
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

pub(crate) fn set_hovered(app: &mut App, cell: Option<CellLevel>) {
    app.world_mut().insert_resource(InspectTarget::new(cell));
}

pub(crate) fn set_selection(app: &mut App, entity: Entity) {
    app.world_mut()
        .insert_resource(SelectedShooter::new(entity));
}

pub(crate) fn move_target(app: &App) -> Option<CellLevel> {
    app.world()
        .get_resource::<gdtf_battle_input::PathPreviewTarget>()
        .and_then(|t| **t)
}

pub(crate) fn selected(app: &App) -> Option<Entity> {
    app.world()
        .get_resource::<SelectedShooter>()
        .and_then(|s| **s)
}

pub(crate) fn add_probes(app: &mut App) {
    app.add_plugins((
        MessageProbePlugin::<FireRequested>::default(),
        MessageProbePlugin::<MoveRequested>::default(),
        MessageProbePlugin::<SetFacingRequested>::default(),
    ));
}

pub(crate) fn fires(app: &App) -> Vec<FireRequested> {
    probed::<FireRequested>(app)
}

pub(crate) fn moves(app: &App) -> Vec<MoveRequested> {
    probed::<MoveRequested>(app)
}

pub(crate) fn facings(app: &App) -> Vec<SetFacingRequested> {
    probed::<SetFacingRequested>(app)
}
