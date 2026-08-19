use bevy::{
    ecs::entity::Entity,
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    prelude::*,
    state::state::State,
    ui::BackgroundColor,
};
use gdtf_app::test_support::{AppState, BattleScapeState, RunningState};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    emplacement::{
        EmplacementEntrySides, EmplacementFacing, EmplacementOccupant, EmplacementState,
    },
    entity::TerrainCell,
    ganger::{Tu, TuMax},
    injuries::InjuryRegistry,
    prelude::{Cell, CellLevel, Faction, Level},
    terrain::facing::TerrainFacing,
    tuning::CombatTuning,
    weapon::WeaponRegistry,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::{
    DisabledButton,
    theme::{GdtfTheme, default_theme},
};

use super::actors::at;

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

pub(crate) fn battle_running_app() -> App {
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

    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Menu)
    });
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });
    app
}

/// The four cardinals, the entry sides a def-seeded open mount authors.
pub(crate) fn cardinal_sides() -> EmplacementEntrySides {
    EmplacementEntrySides::new(TerrainFacing::ALL.to_vec())
}

/// A placed emplacement turned the way the default cardinal points.
pub(crate) fn cardinal_facing() -> EmplacementFacing {
    EmplacementFacing::new(TerrainFacing::default())
}

/// A selected actor with an ample pool, standing on a cell.
pub(crate) fn spawn_emplacement_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app
        .world_mut()
        .spawn((at(x, y), Faction::new(gang), Tu::new(100), TuMax::new(100)))
        .id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

/// An emplacement on a cell, carrying only the parts the caller hands it.
pub(crate) fn spawn_emplacement(
    app: &mut App,
    x: i32,
    y: i32,
    state: EmplacementState,
    occupant: Option<Entity>,
    sides: Option<EmplacementEntrySides>,
    facing: Option<EmplacementFacing>,
) -> Entity {
    let mut entity = app.world_mut().spawn((
        TerrainCell::new(CellLevel::new(Cell::new(x, y), Level::new(0))),
        state,
    ));
    if let Some(occupant) = occupant {
        entity.insert(EmplacementOccupant::new(occupant));
    }
    if let Some(sides) = sides {
        entity.insert(sides);
    }
    if let Some(facing) = facing {
        entity.insert(facing);
    }
    entity.id()
}

pub(crate) fn all_with<M: Component>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).collect()
}

pub(crate) fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    match all_with::<M>(app).as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// The one entity carrying `M`, failing on `claim` when the marker resolves to any other count.
pub(crate) fn the_only<M: Component>(app: &mut App, claim: &str) -> Entity {
    let found = all_with::<M>(app);
    assert_eq!(found.len(), 1, "{claim}; found {} instead", found.len());
    let [one] = found.as_slice() else {
        unreachable!("the count assertion above leaves exactly one entity")
    };
    *one
}

pub(crate) fn visibility<M: Component>(app: &mut App) -> Option<Visibility> {
    let entity = single_with::<M>(app)?;
    app.world().get::<Visibility>(entity).copied()
}

pub(crate) fn parent_of(app: &App, child: Entity) -> Option<Entity> {
    app.world().get::<ChildOf>(child).map(ChildOf::parent)
}

pub(crate) fn press_digit(app: &mut App, key_code: KeyCode) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: Key::Character(" ".into()),
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}

pub(crate) fn set_pool(app: &mut App, actor: Entity, tu: Tu) {
    if let Ok(mut entity) = app.world_mut().get_entity_mut(actor) {
        entity.insert(tu);
    }
}

pub(crate) fn button_greyed<M: Component>(app: &mut App) -> bool {
    single_with::<M>(app).is_some_and(|button| app.world().get::<DisabledButton>(button).is_some())
}

pub(crate) fn button_fill<M: Component>(app: &mut App) -> Option<Color> {
    let button = single_with::<M>(app)?;
    app.world()
        .get::<BackgroundColor>(button)
        .map(|background| background.0)
}

/// The theme fill a button carries while greyed out, and the one it carries enabled and idle.
pub(crate) fn greyed_and_idle_fills(app: &App) -> (Color, Color) {
    let theme = app.world().get_resource::<GdtfTheme>();
    assert!(
        theme.is_some(),
        "the battle app must carry a GdtfTheme, or a button's fill has nothing to be checked \
         against",
    );
    let Some(theme) = theme else {
        unreachable!("the assertion above leaves the theme present")
    };
    (*theme.button.disabled, *theme.button.color)
}
