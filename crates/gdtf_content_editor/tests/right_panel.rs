//! Headless integration test for the map-editor right panel (theme dropdown + size selector),
//! swept onto the UUID-keyed terrain/theme model (GTW-495).
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness, so the
//! editor's actual `Load` pass resolves the shipped theme + terrain registries and its real
//! `Editing` scene spawns the right-panel controls + the shared [`MapEditorSession`] — not a copy.
//!
//! Asserts the contract end-to-end and pin-discriminatingly:
//!
//! - T1 (C1): the theme dropdown lists every theme the [`UuidThemeRegistry`] holds (one option
//!   per registered theme), and the session seeds to the dropdown's pre-selected default theme
//!   (a real, non-nil [`ThemeUuid`]) with that theme's default-floor resolved.
//! - T2 (C2): a real [`DropdownSelectionChanged<ThemeUuid>`] for a different theme updates the
//!   session theme AND resolves that theme's default-floor [`TerrainUuid`] (driving the real
//!   `apply_theme_selection` system).
//! - T3 (C3): real [`NumericFieldCommitted<GridSpanInput>`] commits store an in-bounds width
//!   and keep the stored [`GridSize`] clamped within `60×60×8` (driving the real
//!   `apply_size_commit`).
//! - T4 (layout regression): the controls hang inside the right panel's [`ScrollListArea`].
//!
//! Value-agnostic: it asserts COUNTS / the clamp CEILING / the registry-RESOLVED floor key, never
//! an authored magnitude. Panic/expect-free per the workspace lints.

use bevy::prelude::*;
use gdtf_battle_sim::level::{GridWidth, MAX_GRID_SPAN, ThemeUuid, UuidThemeRegistry};
use gdtf_content_editor::{
    EditorState, GridSpanInput, MapEditorPlugin, MapEditorSession, SizeFieldAxis, ThemeDropdown,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};
use gdtf_ui::{
    CommittedNumericValue, DropdownOptions, DropdownSelectionChanged, NumericFieldCommitted,
    ScrollListArea, UiPlugin,
};

/// A generous frame cap: the async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET — we poll the
/// `EditorState::Editing` SIGNAL, not a fixed count.
const MAX_UPDATES: u32 = 10_000;

/// Builds the real editor app on the no-renderer `DefaultPlugins` UI harness and advances it
/// to [`EditorState::Editing`] with the right-panel controls + the session present.
fn editor_in_editing() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(UiPlugin);
    app.add_plugins(MapEditorPlugin);

    let reached = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|s| *s.get() == EditorState::Editing)
        },
        MAX_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — its Load pass did not resolve the \
         theme + registries (a genuine load failure, not a frame-budget shortfall)",
    );
    // The OnEnter spawn + its deferred re-parent / control-spawn commands + the seed_default_theme
    // Update apply across a few frames; advance so the dropdown + fields + seeded session are
    // present before we query.
    for _ in 0..6 {
        app.update();
    }
    app
}

/// The theme dropdown control entity, if exactly one was spawned.
fn theme_dropdown(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query_filtered::<Entity, With<ThemeDropdown>>();
    let mut iter = query.iter(world);
    iter.next()
}

/// The width size-field entity (the one carrying [`SizeFieldAxis::Width`]), if spawned.
fn width_field(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &SizeFieldAxis)>();
    query
        .iter(world)
        .find_map(|(entity, axis)| matches!(axis, SizeFieldAxis::Width).then_some(entity))
}

/// The session's current [`GridSize`] width, in cells, if the session exists.
fn session_width(app: &App) -> Option<GridWidth> {
    app.world()
        .get_resource::<MapEditorSession>()
        .map(|session| session.grid_size().width())
}

/// The number of themes the registry holds — the count the dropdown must list.
fn registered_theme_count(app: &App) -> Option<usize> {
    app.world()
        .get_resource::<UuidThemeRegistry>()
        .map(UuidThemeRegistry::len)
}

/// T1 (C1): the theme dropdown lists one option per registered theme, and the session seeds to a
/// real (non-nil) theme with its default-floor resolved.
#[test]
fn theme_dropdown_lists_registered_themes_and_seeds_session() {
    let mut app = editor_in_editing();

    let expected = registered_theme_count(&app);
    assert!(
        expected.is_some_and(|n| n > 0),
        "the UuidThemeRegistry must hold at least one theme (a real folder resolve)",
    );

    let world = app.world_mut();
    let mut query = world.query_filtered::<&DropdownOptions<ThemeUuid>, With<ThemeDropdown>>();
    let dropdowns: Vec<_> = query.iter(world).collect();
    assert_eq!(
        dropdowns.len(),
        1,
        "exactly one theme dropdown must be spawned in the right panel",
    );
    if let Some(options) = dropdowns.into_iter().next() {
        assert_eq!(
            Some(options.options().len()),
            expected,
            "the theme dropdown must list one option per registered theme (C1)",
        );
    }

    // The session seeded to a real (non-nil) theme with a resolved default floor (C1).
    let session = app.world().get_resource::<MapEditorSession>();
    assert!(session.is_some(), "the MapEditorSession must exist");
    if let Some(session) = session {
        assert!(
            !session.theme().is_nil(),
            "the session must seed to a real (non-nil) theme once the registry resolves (C1)",
        );
        assert!(
            session.default_floor().is_some(),
            "the seeded theme's default floor must resolve (C1)",
        );
    }
}

/// T2 (C2): selecting a DIFFERENT theme updates the session theme AND resolves that theme's
/// default-floor key — driving the real `apply_theme_selection` system via a synthesized message.
#[test]
fn selecting_a_theme_updates_session_theme_and_default_floor() {
    let mut app = editor_in_editing();

    // Pick a theme DIFFERENT from the session's currently-seeded one (the registry holds >1).
    let start_theme = app
        .world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::theme);
    let target: Option<ThemeUuid> = app
        .world()
        .get_resource::<UuidThemeRegistry>()
        .and_then(|r| {
            r.defs()
                .map(|(key, _)| *key)
                .find(|key| Some(*key) != start_theme)
        });
    assert!(
        target.is_some(),
        "the registry must hold a second theme to switch to (the C2 target)",
    );
    let Some(target) = target else {
        return;
    };

    // The expected resolved default-floor key, read straight from the registry — value-agnostic.
    let expected_floor = app
        .world()
        .get_resource::<UuidThemeRegistry>()
        .and_then(|r| r.default_floor(&target));
    assert!(
        expected_floor.is_some(),
        "the target theme must declare a default-floor TerrainUuid — the C2 resolve target",
    );

    let control = theme_dropdown(&mut app);
    assert!(control.is_some(), "the theme dropdown control must exist");
    if let Some(control) = control {
        app.world_mut()
            .write_message(DropdownSelectionChanged::new(control, target));
        app.update();
    }

    let session = app.world().get_resource::<MapEditorSession>();
    assert!(session.is_some(), "the MapEditorSession must exist");
    if let Some(session) = session {
        assert_eq!(
            session.theme(),
            target,
            "selecting a theme must update the session theme (C2)",
        );
        assert_eq!(
            session.default_floor(),
            expected_floor,
            "selecting a theme must resolve + store that theme's default-floor TerrainUuid (C2)",
        );
    }
}

/// T3 (C3): the size-commit path stores an in-bounds width AND keeps the stored [`GridSize`]
/// clamped within `60×60×8` when an OVER-60 value is committed — driving the real
/// `apply_size_commit` system.
#[test]
fn committing_width_stores_in_bounds_and_clamps_over_max() {
    let mut app = editor_in_editing();

    let field = width_field(&mut app);
    assert!(
        field.is_some(),
        "a width size field (SizeFieldAxis::Width) must be spawned",
    );
    let Some(field) = field else {
        return;
    };

    let in_bounds: u8 = 40;
    assert!(
        in_bounds < MAX_GRID_SPAN,
        "the probe width must be in-bounds"
    );
    app.world_mut().write_message(NumericFieldCommitted::new(
        field,
        CommittedNumericValue::new(GridSpanInput::new(in_bounds)),
    ));
    app.update();
    assert_eq!(
        session_width(&app).map(|w| *w),
        Some(in_bounds),
        "committing an in-bounds width must store it on the session GridSize",
    );

    let over_max: u8 = MAX_GRID_SPAN.saturating_add(100);
    app.world_mut().write_message(NumericFieldCommitted::new(
        field,
        CommittedNumericValue::new(GridSpanInput::new(over_max)),
    ));
    app.update();
    let stored = session_width(&app).map(|w| *w);
    assert_eq!(
        stored,
        Some(in_bounds),
        "an over-max commit must leave the stored GridSize width clamped within the \
         {MAX_GRID_SPAN} ceiling (fail-closed reject keeps the last in-bounds width)",
    );
    if let Some(stored) = stored {
        assert!(
            stored <= MAX_GRID_SPAN,
            "the stored GridSize width must never exceed the {MAX_GRID_SPAN} ceiling (C3)",
        );
    }
}

/// T4 (layout regression — GTW-421 bottom-cramp): the size-field controls must hang inside the
/// right panel's [`ScrollListArea`] — the frame's clipping, scrolling viewport — NOT under the
/// scroll-list grid ROOT FRAME.
#[test]
fn controls_live_inside_the_scroll_area_not_the_grid_frame() {
    let mut app = editor_in_editing();

    let field = width_field(&mut app);
    assert!(
        field.is_some(),
        "a width size field (SizeFieldAxis::Width) must be spawned",
    );
    let Some(field) = field else {
        return;
    };

    let world = app.world_mut();
    let mut areas = world.query_filtered::<Entity, With<ScrollListArea>>();
    let area_set: Vec<Entity> = areas.iter(world).collect();
    let mut parents = world.query::<&ChildOf>();

    let mut cursor = Some(field);
    let mut found_in_area = false;
    for _ in 0..64 {
        let Some(current) = cursor else {
            break;
        };
        if area_set.contains(&current) {
            found_in_area = true;
            break;
        }
        cursor = parents.get(world, current).ok().map(ChildOf::parent);
    }
    assert!(
        found_in_area,
        "the right-panel controls must be parented inside the ScrollListArea (the clipping, \
         scrolling viewport) so they top-anchor and scroll — not under the grid root frame \
         (the GTW-421 bottom-cramp regression)",
    );
}
