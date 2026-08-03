//! authoring, and the `SystemState` `decide_left_click` driver.

use bevy::{
    ecs::system::SystemState, input::ButtonInput, math::Vec2, platform::collections::HashSet,
    prelude::*,
};
use gdtf_battle_input::{
    GdtfBattleInputPlugin, InspectTarget, LeftClickOutcome, PathPreviewTarget, SelectedFireMode,
    SelectedShooter, decide_left_click,
    fire_surface::{ShooterFireData, WeaponMagazine},
    selection::LeftClickReads,
};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::{Aiming, TuMax},
    magazine::{LoadedRounds, Magazine, ReloadTu},
    prelude::{CellLevel, Faction, Level, LifeState, OccupancyGrid, Position, Tu},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::SquadVisibility,
    weapon::{
        FireMode, FireModeSpec, Handedness, MagazineSize, MeleeWeapon, ModeConeMult, ModeKind,
        ModeShots, ModeTuPercent, WieldedBy, Wields,
    },
};

pub(crate) const PLAYER_FACTION: Faction = Faction::new(0);
pub(crate) const ENEMY_FACTION: Faction = Faction::new(1);
pub(crate) const LEVEL: Level = Level::new(0);
pub(crate) const TARGET_SIZE: Vec2 = Vec2::new(1280.0, 720.0);

pub(crate) const fn spec(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

pub(crate) fn decision_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut()
        .insert_resource(SelectedFireMode::new(spec(0.2, 1)));
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());
    app
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
    enemy
}

pub(crate) fn seed_fog(app: &mut App, visible: &[CellLevel], explored: &[CellLevel]) {
    let visible: HashSet<CellLevel> = visible.iter().copied().collect();
    let mut explored_set: HashSet<CellLevel> = explored.iter().copied().collect();
    explored_set.extend(visible.iter().copied());
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible, explored_set));
}

type DecideParams<'w, 's> = (
    LeftClickReads<'w>,
    Res<'w, InspectTarget>,
    Res<'w, PathPreviewTarget>,
    Query<'w, 's, &'static Faction>,
    Query<'w, 's, &'static LifeState>,
    Query<'w, 's, ShooterFireData<'static>>,
    Query<'w, 's, &'static Wields>,
    Query<'w, 's, WeaponMagazine<'static>, With<WieldedBy>>,
    Query<'w, 's, (), With<MeleeWeapon>>,
    Res<'w, SelectedShooter>,
);

pub(crate) fn decide(app: &mut App) -> LeftClickOutcome {
    let world = app.world_mut();
    let mut state: SystemState<DecideParams> = SystemState::new(world);
    let Ok((
        reads,
        inspect,
        move_target,
        factions,
        lifes,
        shooters,
        wields,
        weapons,
        melee,
        selected,
    )) = state.get(world)
    else {
        return LeftClickOutcome::NoOp;
    };
    decide_left_click(
        &reads,
        &inspect,
        &move_target,
        &factions,
        &lifes,
        &shooters,
        &wields,
        &weapons,
        &melee,
        &selected,
    )
}
