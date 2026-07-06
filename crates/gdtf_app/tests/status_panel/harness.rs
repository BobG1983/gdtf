//! App-driving harness, generic queries, and the shared ganger fixture for the status-panel suite.

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

/// A budget large enough to drive the deep walk into the battlescape, bounded so a machine
/// that never reaches the predicate fails instead of hanging.
pub(crate) const BUDGET: u32 = 96;

// ---------------------------------------------------------------------------------
// Harness — drive the real stack to BattleRunning, where the panels are live.
// ---------------------------------------------------------------------------------

/// Reads the current [`BattleScapeState`] if it is active.
pub(crate) fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if it is active.
pub(crate) fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Builds the headless walk app, injecting the persistent `Load` resources (theme +
/// tuning + an empty weapon registry) the machine needs to traverse `Load`. No
/// `LoadedSituation` → the empty `Situation::default()` battle is set up, which still makes
/// `BattleInProgress` + `OccupancyGrid` present in `BattleRunning`.
pub(crate) fn walk_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-505: the Load->Intro gate also requires a MeleeWeaponRegistry (empty-default
    // seed stands in for the asset-less resolve, mirroring the WeaponRegistry seed above).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
    // GTW-269: the Load gate also requires an ArmorRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load→Intro gate also requires a GangRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    // GTW-489: the NEW gate-blocking PrefabRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    // GTW-487: the NEW gate-blocking TerrainDefRegistry + UuidThemeRegistry.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    app
}

/// Drives the app from `Running`/`Menu` down to `BattleScapeState::BattleRunning`.
pub(crate) fn drive_to_battle_running(app: &mut App) -> bool {
    let at_menu = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    if !at_menu {
        return false;
    }
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(
        app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    )
}

/// Drives the walk to `BattleRunning` and returns the app, asserting the descent succeeded.
pub(crate) fn battle_running_app() -> App {
    let mut app = walk_app();
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleScapeState::BattleRunning within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    app
}

/// Whether `entity` has an ancestor carrying marker `R` (walks the `ChildOf` chain up).
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

/// All entities carrying marker `M`.
pub(crate) fn all_with<M: Component>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).collect()
}

/// The single entity carrying marker `M`, scoped to the STATUS panel (NOT a descendant of
/// the inspect panel root). The two panels share the stat-block markers, so this discriminates
/// the status panel's widget from the inspect panel's.
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

/// The single entity carrying INSPECT-EXCLUSIVE marker `M` (only the inspect panel carries it,
/// so no scoping is needed — assert there is exactly one).
pub(crate) fn single_global<M: Component>(app: &mut App) -> Option<Entity> {
    match all_with::<M>(app).as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// The fill PERCENT of the `ProgressBar` rooted at `track` — reads the `ProgressBarFill`
/// child's `Node.width`. `None` if missing.
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

/// A ganger's vitals for a test spawn — grouped into one struct (the `too_many_arguments`
/// idiom).
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

/// Spawns a ganger with the components the stat block reads, SELECTS it, and returns its
/// entity. (The cursor-click selection is covered in `gdtf_battle_input`; the panel only
/// reads `*SelectedShooter` + the on-entity components.)
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

/// A reasonable default ganger setup the caller overrides per test.
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
