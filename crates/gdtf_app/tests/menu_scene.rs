//! GTW-121: headless behavioral tests for the main-menu spawn scene.
//!
//! These run on the `MinimalPlugins` [`GdtfTestAppBuilder`] harness (the real
//! state stack + `UiPlugin`, which installs `FocusNavPlugin` and so initializes
//! the [`DirectionalNavigationMap`]). They assert on component **presence** and
//! resource **values**, never `Node` geometry — `MinimalPlugins` runs no
//! `bevy_ui` layout, and the menu's correctness here is "the right entities with
//! the right markers, focus, and nav edges exist", which is fully headless.
//!
//! The menu reads [`GdtfTheme`] in `OnEnter(RunningState::Menu)`, so each test
//! seeds a theme fixture **before** the first `update()` (when `Running` is
//! entered and the menu spawns) — mirroring how `state_walk` seeds it for the
//! `Load` scene. Pixel-accurate rendering of the spawned menu (real font/layout)
//! is **TBD (Bevy harness)** — out of scope for these presence/value asserts.

use bevy::{
    ecs::entity::Entity,
    input_focus::{InputFocus, directional_navigation::DirectionalNavigationMap},
    math::CompassOctant,
    prelude::ChildOf,
    state::state::State,
    ui::widget::Button,
};
use gdtf_app::test_support::{
    AppState, BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton,
    RunningState,
};
use gdtf_test_utils::GdtfTestAppBuilder;
use gdtf_ui::{
    DisabledButton,
    theme::default_theme,
    themed::{ThemeRole, Themed},
};

/// Builds a headless app started toward [`AppState::Running`], seeds a
/// [`GdtfTheme`] (the menu reads it on entry), and runs one update so the menu
/// is spawned and resting in [`RunningState::Menu`].
fn menu_app() -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    // Seed the theme BEFORE the first update so `OnEnter(RunningState::Menu)`'s
    // `spawn_menu` sees it (the real app resolves it during `Load`).
    app.world_mut().insert_resource(default_theme());
    app.update();
    app
}

/// Looks up the single entity carrying marker `M`, if exactly one exists.
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

/// Entering the menu rests in [`RunningState::Menu`] and spawns the full button
/// set — at least three *enabled* buttons (Battlescape / Options / Quit lack
/// [`DisabledButton`]) plus the disabled `HiveScape` — AC#1, AC#3.
///
/// Pin: if the spawn system were dropped, or auto-advanced off `Menu`, the
/// state or the button counts would fail.
#[test]
fn entering_menu_spawns_buttons_and_rests_on_menu() {
    let mut app = menu_app();

    assert_eq!(
        app.world().resource::<State<RunningState>>().get(),
        &RunningState::Menu,
        "menu must rest on RunningState::Menu (no auto-advance)",
    );

    // At least 3 entities have Button and LACK DisabledButton (the enabled ones).
    let mut enabled = app.world_mut().query_filtered::<Entity, (
        bevy::ecs::prelude::With<Button>,
        bevy::ecs::prelude::Without<DisabledButton>,
    )>();
    let enabled_count = enabled.iter(app.world()).count();
    assert!(
        enabled_count >= 3,
        "expected at least 3 enabled buttons, found {enabled_count}",
    );

    // Exactly one disabled button exists (HiveScape).
    let mut disabled = app
        .world_mut()
        .query_filtered::<Entity, bevy::ecs::prelude::With<DisabledButton>>();
    assert_eq!(
        disabled.iter(app.world()).count(),
        1,
        "exactly one disabled button (HiveScape) must exist",
    );
}

/// All four role markers are present, and `HiveScape` additionally carries
/// [`DisabledButton`] while the others do not — AC#3, AC#7.
///
/// Pin: dropping any marker, or disabling the wrong button, fails an assert.
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

/// Every menu entity (title + four buttons) carries both [`Themed`] and
/// [`DespawnOnExit(RunningState::Menu)`], with the correct [`ThemeRole`]: the
/// title is [`ThemeRole::Title`] and each button root is [`ThemeRole::Button`]
/// (GTW-149).
///
/// Pin: if a menu entity were spawned without the Themed marker (so a re-theme
/// wouldn't restyle it), without the right role, or without the state-scoped
/// despawn marker (so it would leak across the transition), an assert fails.
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

/// The menu tree matches the GTW-149 layout: a `Themed(Background)` root holds the
/// title as a DIRECT child and a `Themed(Panel)` box as another child; the four
/// buttons are children of the panel box (NOT of the root).
///
/// Pin: if the title were re-parented under the panel, if the panel box marker
/// were dropped, or if the buttons were parented to the root instead of the panel,
/// the corresponding `ChildOf` / role assert fails.
#[test]
fn menu_tree_is_background_root_with_title_and_panel_box() {
    let mut app = menu_app();

    let title = single_with::<MenuTitle>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let battlescape = single_with::<BattlescapeButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let options = single_with::<OptionsButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let hivescape = single_with::<HiveScapeButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let quit = single_with::<QuitButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);

    let world = app.world();

    // The title's parent is the root, and the root is Themed(Background).
    let root = world
        .get::<ChildOf>(title)
        .map_or(Entity::PLACEHOLDER, bevy::prelude::ChildOf::parent);
    assert_eq!(
        world.get::<Themed>(root).map(|t| **t),
        Some(ThemeRole::Background),
        "the title's parent (the menu root) must be Themed(Background)",
    );

    // Each button's parent is the panel box, and that box is Themed(Panel).
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

    // The panel box itself is a child of the root (a sibling of the title).
    assert_eq!(
        world
            .get::<ChildOf>(panel)
            .map(bevy::prelude::ChildOf::parent),
        Some(root),
        "the panel box must be a child of the menu root",
    );
    // And the title is NOT inside the panel — it floats on the backdrop.
    assert_ne!(
        world
            .get::<ChildOf>(title)
            .map(bevy::prelude::ChildOf::parent),
        Some(panel),
        "the title must NOT be a child of the panel box (it floats on the backdrop)",
    );
}

/// Initial focus is the Battlescape button — AC#5.
///
/// Pin: if `set_initial_focus` were dropped or aimed at another button,
/// `InputFocus` would not point at the Battlescape entity.
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

/// The non-wrapping nav chain Battlescape ↔ Options ↔ Quit exists, and the
/// disabled `HiveScape` is OMITTED from it — AC#10.
///
/// Pin: missing/asymmetric edges, a wrapping edge (Quit→Battlescape), or
/// `HiveScape` appearing as a neighbor each fail an assert.
#[test]
fn nav_chain_links_enabled_buttons_only() {
    let mut app = menu_app();

    let battlescape = single_with::<BattlescapeButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let options = single_with::<OptionsButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let quit = single_with::<QuitButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    let hivescape = single_with::<HiveScapeButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);

    let map = app.world().resource::<DirectionalNavigationMap>();

    // Battlescape <-> Options.
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
    // Options <-> Quit.
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
    // No wrap: Quit has no South neighbor, Battlescape has no North neighbor.
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
    // HiveScape (disabled) is omitted entirely from the chain.
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

/// Leaving the menu despawns every menu entity AND removes them from the nav map
/// — AC#10 cleanup.
///
/// Pin: if the `DespawnOnExit` markers were missing the entities would survive;
/// if `clear_nav_map` were dropped the stale edges would linger.
#[test]
fn leaving_menu_despawns_entities_and_clears_nav_map() {
    let mut app = menu_app();

    let battlescape = single_with::<BattlescapeButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);

    // Drive Menu -> Options so OnExit(Menu) fires.
    app.world_mut()
        .resource_mut::<bevy::state::state::NextState<RunningState>>()
        .set(RunningState::Options);
    app.update();

    assert_ne!(
        app.world().resource::<State<RunningState>>().get(),
        &RunningState::Menu,
        "precondition: must have left RunningState::Menu",
    );

    // All marker entities are gone.
    for (label, present) in [
        ("battlescape", single_with::<BattlescapeButton>(&mut app)),
        ("options", single_with::<OptionsButton>(&mut app)),
        ("hivescape", single_with::<HiveScapeButton>(&mut app)),
        ("quit", single_with::<QuitButton>(&mut app)),
        ("title", single_with::<MenuTitle>(&mut app)),
    ] {
        assert!(present.is_none(), "{label} must be despawned on menu exit");
    }

    // The nav map no longer references the old menu entities.
    let map = app.world().resource::<DirectionalNavigationMap>();
    assert!(
        map.get_neighbors(battlescape).is_none(),
        "the old menu entities must be cleared from the nav map",
    );
    assert!(
        map.neighbors.is_empty(),
        "the nav map must be empty after the menu (its sole user) leaves",
    );
}
