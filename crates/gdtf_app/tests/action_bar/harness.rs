use bevy::{
    ecs::entity::Entity,
    prelude::*,
    state::state::State,
    ui::{Display, Node},
};
use gdtf_app::test_support::{AppState, BattleScapeState, RunningState};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    injuries::InjuryRegistry,
    tuning::CombatTuning,
    weapon::{
        FireMode, FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, WeaponRegistry,
    },
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::{ActiveSegment, SegmentIndex, theme::default_theme};

pub(crate) const BUDGET: u32 = 96;

pub(crate) const STABLE_CONTROL_BUTTONS: usize = 6;


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
    app.world_mut().insert_resource(ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    app
}

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

pub(crate) fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

pub(crate) fn count_with<M: Component>(app: &mut App) -> usize {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).count()
}

pub(crate) fn segment_is_active<M: Component>(app: &mut App) -> bool {
    let Some(segment) = single_with::<M>(app) else {
        return false;
    };
    let Some(index) = app.world().get::<SegmentIndex>(segment).map(|i| **i) else {
        return false;
    };
    let Some(parent) = app.world().get::<ChildOf>(segment).map(ChildOf::parent) else {
        return false;
    };
    app.world()
        .get::<ActiveSegment>(parent)
        .is_some_and(|active| **active == index)
}

pub(crate) fn segment_display<M: Component>(app: &mut App) -> Option<Display> {
    single_with::<M>(app).and_then(|e| app.world().get::<Node>(e).map(|n| n.display))
}

pub(crate) fn require_button<M: Component>(app: &mut App) -> Option<Entity> {
    let found = single_with::<M>(app);
    assert!(
        found.is_some(),
        "exactly one button of the expected marker must be spawned in BattleRunning",
    );
    found
}

pub(crate) const fn spec(kind: ModeKind, tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        kind,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

pub(crate) fn sbf_selector() -> FireMode {
    FireMode::new(vec![
        spec(ModeKind::Single, 0.2, 1),
        spec(ModeKind::Burst, 0.4, 3),
        spec(ModeKind::Full, 0.7, 6),
    ])
}
