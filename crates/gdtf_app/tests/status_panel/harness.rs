use bevy::{ecs::entity::Entity, prelude::*, state::state::State, ui::Val};
use gdtf_app::test_support::{AppState, BattleScapeState, InspectPanelRoot, RunningState};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    ganger::{GangerName, Hp, HpMax, TuMax, Wounds, WoundsMax},
    inflicted_wound::InflictedWounds,
    injuries::{InflictedInjuries, InjuryRegistry},
    prelude::{Cell, CellLevel, Faction, Level, LifeState, Position, Stance, StanceKind, Tu},
    tuning::CombatTuning,
    weapon::WeaponRegistry,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::{ProgressBarFill, theme::default_theme};

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

pub(crate) fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

pub(crate) fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

pub(crate) fn walk_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(
        ONE_STEP_A_FRAME,
    ));
    app
}

pub(crate) fn drive_to_battle_running(app: &mut App) {
    advance_until(app, |app| running_state(app) == Some(RunningState::Menu));
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });
}

pub(crate) fn battle_running_app() -> App {
    let mut app = walk_app();
    drive_to_battle_running(&mut app);
    app
}

pub(crate) fn descends_from<R: Component>(app: &App, entity: Entity) -> bool {
    let mut current = entity;
    loop {
        if app.world().get::<R>(current).is_some() {
            return true;
        }
        match app.world().get::<bevy::prelude::ChildOf>(current) {
            Some(parent) => current = parent.parent(),
            None => return false,
        }
    }
}

pub(crate) fn all_with<M: Component>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).collect()
}

pub(crate) fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    let found: Vec<Entity> = all_with::<M>(app)
        .into_iter()
        .filter(|&e| !descends_from::<InspectPanelRoot>(app, e))
        .collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

pub(crate) fn single_global<M: Component>(app: &mut App) -> Option<Entity> {
    match all_with::<M>(app).as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

pub(crate) fn bar_fill_at(app: &App, track: Entity) -> Option<f32> {
    let kids: Vec<Entity> = app
        .world()
        .get::<Children>(track)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    for kid in kids {
        if app.world().get::<ProgressBarFill>(kid).is_some()
            && let Some(node) = app.world().get::<Node>(kid)
            && let Val::Percent(p) = node.width
        {
            return Some(p);
        }
    }
    None
}

pub(crate) struct GangerSetup {
    pub(crate) cell:       Cell,
    pub(crate) level:      Level,
    pub(crate) name:       GangerName,
    pub(crate) faction:    Faction,
    pub(crate) stance:     StanceKind,
    pub(crate) tu:         Tu,
    pub(crate) tu_max:     TuMax,
    pub(crate) hp:         Hp,
    pub(crate) hp_max:     HpMax,
    pub(crate) wounds:     Wounds,
    pub(crate) wounds_max: WoundsMax,
    pub(crate) life:       LifeState,
    pub(crate) inflicted:  InflictedWounds,
    pub(crate) injuries:   InflictedInjuries,
}

pub(crate) fn spawn_and_select(app: &mut App, setup: GangerSetup) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(CellLevel::new(setup.cell, setup.level)),
            setup.name,
            setup.faction,
            Stance::new(setup.stance),
            setup.tu,
            setup.tu_max,
            setup.hp,
            setup.hp_max,
            setup.wounds,
            setup.wounds_max,
            setup.life,
            setup.inflicted,
            setup.injuries,
        ))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    ganger
}

pub(crate) fn default_setup() -> GangerSetup {
    GangerSetup {
        cell:       Cell::new(5, 6),
        level:      Level::new(0),
        name:       GangerName::new("Vex Harker".to_owned()),
        faction:    Faction::new(1),
        stance:     StanceKind::Crouching,
        tu:         Tu::new(7),
        tu_max:     TuMax::new(10),
        hp:         Hp::new(8),
        hp_max:     HpMax::new(16),
        wounds:     Wounds::new(2),
        wounds_max: WoundsMax::new(3),
        life:       LifeState::Alive,
        inflicted:  InflictedWounds::default(),
        injuries:   InflictedInjuries::default(),
    }
}
