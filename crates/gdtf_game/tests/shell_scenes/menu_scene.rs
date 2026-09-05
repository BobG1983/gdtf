//! Main menu: buttons, theme roles, focus, nav chain, despawn on exit.
use bevy::{
    ecs::entity::Entity,
    input_focus::{InputFocus, directional_navigation::DirectionalNavigationMap},
    math::CompassOctant,
    prelude::ChildOf,
    state::state::State,
    ui::widget::Button,
};
use cobalt_test_utils::MinimalTestAppBuilder;
use gdtf_game::test_support::{
    AppState, BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton,
    RunningState,
};
use gdtf_ui::{
    DisabledButton,
    theme::default_theme,
    themed::{ThemeRole, Themed},
};

fn menu_app() -> bevy::app::App {
    let mut app =
        MinimalTestAppBuilder::new_with_scene_support(gdtf_game::test_support::register_headless)
            .starting_in(AppState::Running)
            .build();
    app.world_mut().insert_resource(default_theme());
    app.update();
    app
}

fn single_with<M: bevy::ecs::component::Component>(app: &mut bevy::app::App) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, bevy::ecs::prelude::With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

#[test]
fn entering_menu_spawns_buttons_and_rests_on_menu() {
    let mut app = menu_app();

    assert_eq!(
        app.world().resource::<State<RunningState>>().get(),
        &RunningState::Menu,
        "menu must rest on RunningState::Menu (no auto-advance)",
    );

    let mut enabled = app.world_mut().query_filtered::<Entity, (
        bevy::ecs::prelude::With<Button>,
        bevy::ecs::prelude::Without<DisabledButton>,
    )>();
    let enabled_count = enabled.iter(app.world()).count();
    assert!(
        enabled_count >= 3,
        "expected at least 3 enabled buttons, found {enabled_count}",
    );

    let mut disabled = app
        .world_mut()
        .query_filtered::<Entity, bevy::ecs::prelude::With<DisabledButton>>();
    assert_eq!(
        disabled.iter(app.world()).count(),
        1,
        "exactly one disabled button (HiveScape) must exist",
    );
}

#[test]
fn all_four_markers_present_with_correct_disabled_state() {
    let mut app = menu_app();

    let battlescape = single_with::<BattlescapeButton>(&mut app);
    let options = single_with::<OptionsButton>(&mut app);
    let hivescape = single_with::<HiveScapeButton>(&mut app);
    let quit = single_with::<QuitButton>(&mut app);

    assert!(battlescape.is_some(), "BattlescapeButton must be present");
    assert!(options.is_some(), "OptionsButton must be present");
    assert!(hivescape.is_some(), "HiveScapeButton must be present");
    assert!(quit.is_some(), "QuitButton must be present");

    let world = app.world();
    assert!(
        hivescape.is_some_and(|e| world.get::<DisabledButton>(e).is_some()),
        "HiveScape must be a DisabledButton",
    );
    for (label, entity) in [
        ("Battlescape", battlescape),
        ("Options", options),
        ("Quit", quit),
    ] {
        assert!(
            entity.is_some_and(|e| world.get::<DisabledButton>(e).is_none()),
            "{label} must NOT be disabled",
        );
    }
}

#[test]
fn every_menu_entity_is_themed_and_state_scoped() {
    let mut app = menu_app();

    let title = single_with::<MenuTitle>(&mut app);
    let battlescape = single_with::<BattlescapeButton>(&mut app);
    let options = single_with::<OptionsButton>(&mut app);
    let hivescape = single_with::<HiveScapeButton>(&mut app);
    let quit = single_with::<QuitButton>(&mut app);

    let world = app.world();
    for (label, entity, role) in [
        ("title", title, ThemeRole::Title),
        ("battlescape", battlescape, ThemeRole::Button),
        ("options", options, ThemeRole::Button),
        ("hivescape", hivescape, ThemeRole::Button),
        ("quit", quit, ThemeRole::Button),
    ] {
        let entity = entity.unwrap_or(Entity::PLACEHOLDER);
        assert_eq!(
            world.get::<Themed>(entity).map(|t| **t),
            Some(role),
            "{label} must carry Themed({role:?})",
        );
        assert!(
            world
                .get::<bevy::prelude::DespawnOnExit<RunningState>>(entity)
                .is_some_and(|marker| marker.0 == RunningState::Menu),
            "{label} must carry DespawnOnExit(RunningState::Menu)",
        );
    }
}

#[test]
fn menu_tree_is_background_root_with_title_and_panel_box() {
    let mut app = menu_app();

    let title = single_with::<MenuTitle>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let battlescape = single_with::<BattlescapeButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let options = single_with::<OptionsButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let hivescape = single_with::<HiveScapeButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let quit = single_with::<QuitButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);

    let world = app.world();

    let root = world
        .get::<ChildOf>(title)
        .map_or(Entity::PLACEHOLDER, bevy::prelude::ChildOf::parent);
    assert_eq!(
        world.get::<Themed>(root).map(|t| **t),
        Some(ThemeRole::Background),
        "the title's parent (the menu root) must be Themed(Background)",
    );

    let panel = world
        .get::<ChildOf>(battlescape)
        .map_or(Entity::PLACEHOLDER, bevy::prelude::ChildOf::parent);
    assert_eq!(
        world.get::<Themed>(panel).map(|t| **t),
        Some(ThemeRole::Panel),
        "the battlescape button's parent (the panel box) must be Themed(Panel)",
    );
    for (label, button) in [
        ("battlescape", battlescape),
        ("options", options),
        ("hivescape", hivescape),
        ("quit", quit),
    ] {
        assert_eq!(
            world
                .get::<ChildOf>(button)
                .map(bevy::prelude::ChildOf::parent),
            Some(panel),
            "{label} must be a child of the panel box",
        );
    }

    assert_eq!(
        world
            .get::<ChildOf>(panel)
            .map(bevy::prelude::ChildOf::parent),
        Some(root),
        "the panel box must be a child of the menu root",
    );
    assert_ne!(
        world
            .get::<ChildOf>(title)
            .map(bevy::prelude::ChildOf::parent),
        Some(panel),
        "the title must NOT be a child of the panel box (it floats on the backdrop)",
    );
}

#[test]
fn battlescape_grabs_initial_focus() {
    let mut app = menu_app();
    let battlescape = single_with::<BattlescapeButton>(&mut app);

    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        battlescape,
        "InputFocus must point at the Battlescape button",
    );
}

#[test]
fn nav_chain_links_enabled_buttons_only() {
    let mut app = menu_app();

    let battlescape = single_with::<BattlescapeButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let options = single_with::<OptionsButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let quit = single_with::<QuitButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let hivescape = single_with::<HiveScapeButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);

    let map = app.world().resource::<DirectionalNavigationMap>();

    assert_eq!(
        map.get_neighbor(battlescape, CompassOctant::South).get(),
        Some(options),
        "Battlescape South neighbor must be Options",
    );
    assert_eq!(
        map.get_neighbor(options, CompassOctant::North).get(),
        Some(battlescape),
        "Options North neighbor must be Battlescape",
    );
    assert_eq!(
        map.get_neighbor(options, CompassOctant::South).get(),
        Some(quit),
        "Options South neighbor must be Quit",
    );
    assert_eq!(
        map.get_neighbor(quit, CompassOctant::North).get(),
        Some(options),
        "Quit North neighbor must be Options",
    );
    assert_eq!(
        map.get_neighbor(quit, CompassOctant::South).get(),
        None,
        "Quit must have no South neighbor (no wrap)",
    );
    assert_eq!(
        map.get_neighbor(battlescape, CompassOctant::North).get(),
        None,
        "Battlescape must have no North neighbor (no wrap)",
    );
    assert_eq!(
        map.get_neighbor(hivescape, CompassOctant::North).get(),
        None,
        "disabled HiveScape must not be in the nav chain",
    );
    assert_eq!(
        map.get_neighbor(hivescape, CompassOctant::South).get(),
        None,
        "disabled HiveScape must not be in the nav chain",
    );
    assert_eq!(
        map.get_neighbor(options, CompassOctant::South).get(),
        Some(quit),
        "Options must skip the disabled HiveScape and link straight to Quit",
    );
}

#[test]
fn leaving_menu_despawns_entities_and_clears_nav_map() {
    let mut app = menu_app();

    let battlescape = single_with::<BattlescapeButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let options = single_with::<OptionsButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let quit = single_with::<QuitButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);

    app.world_mut()
        .resource_mut::<bevy::state::state::NextState<RunningState>>()
        .set(RunningState::Options);
    app.update();

    assert_ne!(
        app.world().resource::<State<RunningState>>().get(),
        &RunningState::Menu,
        "precondition: must have left RunningState::Menu",
    );

    for (label, present) in [
        ("battlescape", single_with::<BattlescapeButton>(&mut app)),
        ("options", single_with::<OptionsButton>(&mut app)),
        ("hivescape", single_with::<HiveScapeButton>(&mut app)),
        ("quit", single_with::<QuitButton>(&mut app)),
        ("title", single_with::<MenuTitle>(&mut app)),
    ] {
        assert!(present.is_none(), "{label} must be despawned on menu exit");
    }

    let map = app.world().resource::<DirectionalNavigationMap>();
    for (label, button) in [
        ("battlescape", battlescape),
        ("options", options),
        ("quit", quit),
    ] {
        assert!(
            map.get_neighbors(button).is_none(),
            "the old menu {label} button must be cleared from the nav map",
        );
    }
}
