//! GTW-119: headless behavioral tests for the focus-navigation layer.
//!
//! These run on the `MinimalPlugins` [`GdtfTestAppBuilder`] harness (the real
//! state stack + `UiPlugin`, which installs `FocusNavPlugin`). They assert the
//! [`InputFocus`] **resource value**, never `Node` geometry — directional
//! navigation here uses the manual [`DirectionalNavigationMap`] graph, so no UI
//! layout is required and the whole pipeline is deterministic headless.
//!
//! ## Non-headless gap
//!
//! What these tests *cannot* cover headless: real device input (a physical
//! `ArrowDown` keypress or gamepad D-pad producing a `NavigateRequest`), and
//! *automatic* directional navigation derived from on-screen `Node` positions
//! (which needs `bevy_ui` layout geometry). The bridge systems
//! (`bridge_keyboard_navigation`, `bridge_gamepad_navigation`) are exercised by
//! synthesizing their *output* message; turning a real keypress into that
//! message is verified by running the app — **TBD (Bevy harness)** for richer
//! input automation. The `apply_navigation` step and `set_initial_focus` helper,
//! which are the rule under test, are fully covered here.

use bevy::{
    input_focus::{InputFocus, directional_navigation::DirectionalNavigationMap},
    math::CompassOctant,
};
use gdtf_test_utils::GdtfTestAppBuilder;
use gdtf_ui::focus_nav::{NavDirection, NavigateRequest, set_initial_focus};

/// `set_initial_focus(commands, entity)` makes that entity the focused one.
///
/// Pin: spawns an entity, calls the helper, flushes the queued command, and
/// asserts [`InputFocus`] now points at it. If `set_initial_focus` stopped
/// setting focus (or set the wrong entity), `InputFocus.get()` would not equal
/// the spawned entity and this fails.
#[test]
fn set_initial_focus_sets_the_start() {
    // GTW-322 — starting in `Running` enters `Menu`, whose `spawn_menu` now authors its
    // tree via `bsn!` / `spawn_scene`; the deferred scene apply needs the scene resources,
    // so use the scene-support constructor (the widget-builder slice precedent).
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(gdtf_app::test_support::AppState::Running)
        .build();
    // One update so the app enters its initial state. (As of Bevy 0.19 the
    // `InputDispatchPlugin` that defaults focus ships in `DefaultPlugins`, not in
    // `UiPlugin`; this `MinimalPlugins` harness never adds it, so there is no
    // PostStartup focus-default system to run here.)
    app.update();

    let world = app.world_mut();
    let target = world.spawn_empty().id();

    // Drive the helper through a one-shot command queue, then flush so the
    // `insert_resource` it queues is applied.
    let mut commands = world.commands();
    set_initial_focus(&mut commands, target);
    world.flush();

    assert_eq!(
        world.resource::<InputFocus>().get(),
        Some(target),
        "set_initial_focus must point InputFocus at the given entity",
    );
}

/// A synthesized navigate-Down advances [`InputFocus`] along a manual edge.
///
/// Builds a two-node focus graph (top above bottom) with a symmetrical South
/// edge, focuses the top node, then writes a [`NavigateRequest`] for
/// [`NavDirection::DOWN`] and runs one update. `apply_navigation` drains the
/// request and calls `DirectionalNavigation::navigate(South)`, which must move
/// focus to the bottom node.
///
/// Pin: if `apply_navigation` were dropped from the schedule (or stopped reading
/// the request / calling `navigate`), focus would remain on the top node and the
/// final assertion would fail. This is the navigate clause's discriminating test.
#[test]
fn navigate_down_advances_focus() {
    // GTW-322 — see the sibling test: the `Menu` `spawn_menu` scene needs the scene
    // resources, so build with scene support.
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(gdtf_app::test_support::AppState::Running)
        .build();
    app.update();

    let world = app.world_mut();
    let top = world.spawn_empty().id();
    let bottom = world.spawn_empty().id();

    // Manual navigation graph: top --South--> bottom (and the reverse North).
    world
        .resource_mut::<DirectionalNavigationMap>()
        .add_symmetrical_edge(top, bottom, CompassOctant::South);

    // Focus starts on the top node.
    set_initial_focus(&mut world.commands(), top);
    world.flush();
    assert_eq!(
        world.resource::<InputFocus>().get(),
        Some(top),
        "focus should start on the top node before navigating",
    );

    // Synthesize a navigate-Down (the bridge's output) and let the apply system
    // consume it.
    world.write_message(NavigateRequest::new(NavDirection::DOWN));
    app.update();

    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(bottom),
        "navigate-Down must advance InputFocus to the bottom (South) neighbor",
    );
}
