//! Headless integration test for the GTW-421 map-editor right panel (theme dropdown + size
//! selector).
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness (the
//! same harness the GTW-417 `editor_shell` test uses), so the editor's actual `Load` pass
//! resolves the shipped theme + catalog registries and its real `Editing` scene spawns the
//! right-panel controls + the shared [`MapEditorSession`] — not a copy.
//!
//! Asserts the contract end-to-end and pin-discriminatingly:
//!
//! - T1 (C1): the theme dropdown exists with all THREE [`LevelTheme`] options and its
//!   [`SelectedIndex`] points at the default theme.
//! - T2 (C2): a real [`DropdownSelectionChanged<LevelTheme>`] for a NON-default theme updates
//!   the session theme AND resolves that theme's catalog default-floor key (driving the real
//!   `apply_theme_selection` system).
//! - T3 (C3): real [`NumericFieldCommitted<GridSpanInput>`] commits store an in-bounds width
//!   and keep the stored [`GridSize`] clamped within `60×60×8` when an over-60 value is
//!   committed (driving the real `apply_size_commit` system).
//! - T4 (layout regression): the controls hang inside the right panel's [`ScrollListArea`]
//!   (the clipping, scrolling viewport) — not the scroll-list grid root frame — so they
//!   top-anchor and scroll instead of dropping into an off-screen implicit grid row (the
//!   GTW-421 bottom-cramp).
//!
//! Value-agnostic: it asserts the clamp CEILING and the catalog-RESOLVED floor key, never an
//! authored magnitude. Panic/expect-free per the workspace lints (the GTW-417 `editor_shell`
//! precedent: `assert!` + `if let Some` guards, never `unwrap`/`expect`/`panic`).

use bevy::prelude::*;
use gdtf_battle_sim::level::{GridWidth, LevelTheme, MAX_GRID_SPAN, ThemeCatalogRegistry};
use gdtf_editor::{
    EditorState, GridSpanInput, MapEditorPlugin, MapEditorSession, SizeFieldAxis, ThemeDropdown,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};
use gdtf_ui::{
    CommittedNumericValue, DropdownOptions, DropdownSelectionChanged, NumericFieldCommitted,
    ScrollListArea, SelectedIndex, UiPlugin,
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
    // The OnEnter spawn + its deferred re-parent / control-spawn commands apply across a few
    // frames; advance so the dropdown + fields + session are present before we query.
    for _ in 0..4 {
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

/// T1 (C1): the theme dropdown exists, lists all three [`LevelTheme`] options, and its
/// [`SelectedIndex`] points at the default theme's slot.
#[test]
fn theme_dropdown_lists_all_themes_with_default_selected() {
    let mut app = editor_in_editing();

    let world = app.world_mut();
    let mut query = world
        .query_filtered::<(&DropdownOptions<LevelTheme>, &SelectedIndex), With<ThemeDropdown>>();
    let dropdowns: Vec<_> = query.iter(world).collect();
    assert_eq!(
        dropdowns.len(),
        1,
        "exactly one theme dropdown must be spawned in the right panel",
    );

    if let Some((options, selected)) = dropdowns.into_iter().next() {
        // All three closed-set LevelTheme variants are offered (C1).
        let offered: Vec<LevelTheme> = options.options().iter().map(|opt| *opt.id()).collect();
        assert_eq!(
            offered.len(),
            3,
            "the theme dropdown must list all three LevelTheme variants, got {offered:?}",
        );
        for variant in [
            LevelTheme::IndustrialHive,
            LevelTheme::Underhive,
            LevelTheme::SumpWaste,
        ] {
            assert!(
                offered.contains(&variant),
                "the theme dropdown must offer {variant:?}",
            );
        }

        // The default theme is pre-selected on open (C1): the SelectedIndex points at the
        // option whose id is LevelTheme::default.
        let selected_id = options.options().get(**selected).map(|opt| *opt.id());
        assert_eq!(
            selected_id,
            Some(LevelTheme::default()),
            "the dropdown must pre-select the default LevelTheme on open",
        );
    }
}

/// T2 (C2): selecting a NON-default theme updates the session theme AND resolves that theme's
/// catalog default-floor key — driving the real `apply_theme_selection` system via a
/// synthesized selection message.
#[test]
fn selecting_a_theme_updates_session_theme_and_default_floor() {
    let mut app = editor_in_editing();

    // Pick a NON-default theme that has a shipped catalog (so the default-floor key resolves).
    let target = LevelTheme::Underhive;
    assert_ne!(
        target,
        LevelTheme::default(),
        "the test must select a non-default theme to prove the update path runs",
    );

    // The expected resolved default-floor key, read straight from the registry the editor
    // loaded — value-agnostic (we assert it MATCHES the catalog, not a hard-coded string).
    let expected_floor = app
        .world()
        .get_resource::<ThemeCatalogRegistry>()
        .and_then(|reg| reg.catalog(target))
        .map(|cat| cat.default_floor_key().clone());
    assert!(
        expected_floor.is_some(),
        "the shipped ThemeCatalogRegistry must hold an Underhive catalog with a default-floor \
         key — the C2 resolve target",
    );

    // Find the dropdown control entity, synthesize the real selection message the widget would
    // emit, and let the registered `apply_theme_selection` system consume it.
    let control = theme_dropdown(&mut app);
    assert!(control.is_some(), "the theme dropdown control must exist");
    if let Some(control) = control {
        app.world_mut()
            .write_message(DropdownSelectionChanged::new(control, target));
        app.update();
    }

    let session = app.world().get_resource::<MapEditorSession>();
    assert!(
        session.is_some(),
        "the MapEditorSession must exist in Editing"
    );
    if let Some(session) = session {
        assert_eq!(
            session.theme(),
            target,
            "selecting a theme must update the session theme",
        );
        assert_eq!(
            session.default_floor().cloned(),
            expected_floor,
            "selecting a theme must resolve + store that theme's catalog default-floor key (C2)",
        );
    }
}

/// T3 (C3): the size-commit path stores an in-bounds width AND keeps the stored
/// [`GridSize`] clamped within `60×60×8` when an OVER-60 value is committed — driving the
/// real `apply_size_commit` system via synthesized commit messages. Pin-discriminating: it
/// commits a known in-bounds width (stored) then an over-ceiling width (rejected fail-closed,
/// never stored, never panicking), so a missing clamp/validation would redden it.
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

    // First commit a KNOWN in-bounds width — proves the width axis updates the stored size.
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

    // Now commit an OVER-ceiling width — the size must stay CLAMPED within 60×60×8: the
    // fail-closed `GridSize::new` rejects it, so the previous in-bounds width is kept (never
    // the over-max value, never a panic).
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

/// T4 (layout regression — GTW-421 bottom-cramp): the size-field controls must hang inside
/// the right panel's [`ScrollListArea`] — the frame's clipping, scrolling viewport — NOT
/// under the scroll-list grid ROOT FRAME. The earlier bug parented the controls onto the
/// frame (the entity carrying [`RightPanelRegion`]), which dropped them into an off-screen
/// implicit grid row below the scroll area; this walks the width field's ancestry and asserts
/// a [`ScrollListArea`] is one of its ancestors. Pre-fix this FAILS (no `ScrollListArea`
/// ancestor — the controls sat under the bare grid frame).
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

    // Walk up the ChildOf chain from the width field; a ScrollListArea must be an ancestor.
    let world = app.world_mut();
    let mut areas = world.query_filtered::<Entity, With<ScrollListArea>>();
    let area_set: Vec<Entity> = areas.iter(world).collect();
    let mut parents = world.query::<&ChildOf>();

    let mut cursor = Some(field);
    let mut found_in_area = false;
    // Bounded walk (the panel subtree is shallow) so a cycle can never hang the test.
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
