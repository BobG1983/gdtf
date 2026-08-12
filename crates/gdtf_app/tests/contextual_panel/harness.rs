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
use gdtf_battle_sim::{
    ganger::Tu, injuries::InjuryRegistry, tuning::CombatTuning, weapon::WeaponRegistry,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::{
    DisabledButton,
    theme::{GdtfTheme, default_theme},
};

pub(crate) const BUDGET: u32 = 96;

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

    let at_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    assert!(at_menu, "the walk should reach RunningState::Menu");
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let at_battle = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        at_battle,
        "the walk should reach BattleScapeState::BattleRunning; last was {:?}",
        battlescape_state(&app),
    );
    app
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
