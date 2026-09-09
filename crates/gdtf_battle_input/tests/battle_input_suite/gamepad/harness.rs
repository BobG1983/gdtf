//! authoring, and the `SystemState` `decide_left_click` driver.

use bevy::{
    ecs::system::SystemState, input::ButtonInput, math::Vec2, platform::collections::HashSet,
    prelude::*,
};
use gdtf_battle_input::{
    ChosenFireMode, GdtfBattleInputPlugin, LeftClickOutcome, SelectedShooter, ShooterArms,
    chosen_spec, decide_left_click,
    selection::{LeftClickReads, PointerSelection},
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

/// The spec the gun the selected shooter fires is set to.
pub(crate) fn selected_mode(app: &App) -> Option<FireModeSpec> {
    let shooter = app
        .world()
        .get_resource::<SelectedShooter>()
        .and_then(|selected| **selected)?;
    let wields = app.world().get::<Wields>(shooter)?;
    let weapon = wields.ranged_weapon(|weapon| {
        app.world()
            .get_entity(weapon)
            .is_ok_and(|row| row.contains::<MeleeWeapon>())
    })?;
    let modes = app.world().get::<FireMode>(weapon)?;
    chosen_spec(modes, app.world().get::<ChosenFireMode>(weapon))
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
    PointerSelection<'w>,
    Query<'w, 's, &'static Faction>,
    Query<'w, 's, &'static LifeState>,
    ShooterArms<'w, 's>,
);

pub(crate) fn decide(app: &mut App) -> LeftClickOutcome {
    let world = app.world_mut();
    let mut state: SystemState<DecideParams> = SystemState::new(world);
    let Ok((reads, selection, factions, lifes, arms)) = state.get_mut(world) else {
        return LeftClickOutcome::NoOp;
    };
    decide_left_click(&reads, &selection, &factions, &lifes, &arms)
}
