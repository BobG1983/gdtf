//! The test dropdown fixture + shared popup/label/message readers.

use bevy::prelude::*;
use gdtf_test_utils::GdtfUiTestAppBuilder;
use gdtf_ui::{
    DropdownColors, DropdownItem, DropdownLabel, DropdownOption, DropdownPopup,
    DropdownSelectionChanged, UiPlugin, register_dropdown, spawn_dropdown,
};

/// The test's option identity — a named newtype over the choice name (no-bare-types rule).
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct TestChoice(String);

impl TestChoice {
    /// Wraps a choice name.
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// The z-band the contextual panel renders on — the highest existing UI layer the floating
/// dropdown list must clear (mirrors `CONTEXTUAL_PANEL_Z` in `gdtf_app`).
pub(crate) const CONTEXTUAL_PANEL_Z: i32 = 20;

/// A distinct test color set so asserts discriminate.
pub(crate) const COLORS: DropdownColors = DropdownColors {
    control_bg:          Color::srgb(0.2, 0.2, 0.25),
    text:                Color::srgb(0.9, 0.9, 0.8),
    popup_bg:            Color::srgb(0.1, 0.1, 0.15),
    option_bg:           Color::srgb(0.15, 0.15, 0.2),
    option_highlight_bg: Color::srgb(0.45, 0.62, 0.30),
};

/// Builds the harness: the headless UI app + `UiPlugin` + the `TestChoice` dropdown
/// registration, so the real drivers run on a real layout.
pub(crate) fn harness() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(UiPlugin);
    register_dropdown::<TestChoice>(&mut app);
    app
}

/// The three test options, all distinct.
pub(crate) fn options() -> Vec<DropdownOption<TestChoice>> {
    vec![
        DropdownOption::new(TestChoice::new("alpha"), "Alpha"),
        DropdownOption::new(TestChoice::new("bravo"), "Bravo"),
        DropdownOption::new(TestChoice::new("charlie"), "Charlie"),
    ]
}

/// Spawns a dropdown with the three options (initially showing the first) and settles the
/// layout, returning the control entity.
pub(crate) fn spawn_test_dropdown(app: &mut App) -> Entity {
    let control = {
        let mut commands = app.world_mut().commands();
        spawn_dropdown(&mut commands, options(), 0, COLORS, ())
    };
    for _ in 0..3 {
        app.update();
    }
    control
}

/// Presses an entity (sets `Interaction::Pressed`) and runs ONLY the `Update` schedule so the
/// matching `Changed<Interaction>==Pressed` driver fires on the press edge.
///
/// It deliberately runs `Update` directly rather than `app.update()`: under the
/// `DefaultPlugins` headless harness the primary window is absent, so `bevy_ui`'s built-in
/// `ui_focus_system` (in `PreUpdate`) `set_if_neq`s every node's `Interaction` back to `None`
/// (no cursor over any node) — a full `app.update()` would clobber the manually-set `Pressed`
/// BEFORE the dropdown driver reads it. Running only `Update` (where every dropdown driver
/// lives) reads the edge we set; [`settle`] then runs full updates for layout.
pub(crate) fn press(app: &mut App, entity: Entity) {
    gdtf_test_utils::press_ui_button(app, entity);
    app.world_mut().run_schedule(Update);
}

/// Runs a few full `app.update()`s so freshly-spawned UI nodes get a computed layout. (These
/// re-clobber transient `Interaction`s to `None`, which is fine once a press has been handled.)
pub(crate) fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

/// The single open `DropdownPopup` entity, if any.
pub(crate) fn popup_of(app: &mut App) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<DropdownPopup>>();
    q.iter(app.world()).next()
}

/// The option-row entities of the open popup, in child order.
pub(crate) fn option_rows(app: &mut App) -> Vec<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<DropdownItem<TestChoice>>>();
    let mut rows: Vec<Entity> = q.iter(app.world()).collect();
    rows.sort_by_key(|&e| {
        app.world()
            .get::<DropdownItem<TestChoice>>(e)
            .map_or(usize::MAX, |item| *item.index())
    });
    rows
}

/// The shown label text of the closed control.
pub(crate) fn shown_label(app: &App, control: Entity) -> Option<String> {
    let children = app.world().get::<Children>(control)?;
    for &child in children {
        if app.world().get::<DropdownLabel>(child).is_some() {
            return app.world().get::<Text>(child).map(|t| t.0.clone());
        }
    }
    None
}

/// Drains the `DropdownSelectionChanged<TestChoice>` messages.
pub(crate) fn selection_messages(app: &mut App) -> Vec<DropdownSelectionChanged<TestChoice>> {
    let mut state: bevy::ecs::system::SystemState<
        MessageReader<DropdownSelectionChanged<TestChoice>>,
    > = bevy::ecs::system::SystemState::new(app.world_mut());
    let Ok(mut reader) = state.get_mut(app.world_mut()) else {
        return Vec::new();
    };
    reader.read().cloned().collect()
}
