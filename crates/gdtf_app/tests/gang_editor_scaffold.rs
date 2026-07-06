//! GTW-420: headless behavioral tests for the in-app gang-editor SCAFFOLD.
//!
//! These run on the `MinimalPlugins` [`GdtfTestAppBuilder`] (the real state stack, `UiPlugin`,
//! and the real `GangEditorScenePlugin` wired through `ScenesPlugin`), seeded with a theme so the
//! editor screen spawns. They assert on the WORLD and the model resource — the system-effects —
//! never on rendering or real device input.
//!
//! Coverage (C5):
//!
//! - [`enter_editor_inserts_model_and_screen`] — driving into
//!   [`RunningState::DebugGangEditor`](gdtf_app::test_support::RunningState) inserts the
//!   [`EditableGang`] model AND spawns the editor screen root (C1 / C2).
//! - [`add_member_grows_model_and_adds_row`] — pressing the "Add member" button appends a
//!   member to the model AND a member-row entity appears (AC4).
//! - [`exit_editor_despawns_screen_and_removes_model`] — transitioning AWAY from `DebugGangEditor`
//!   despawns the screen root AND removes the model resource (C1).
//! - [`commit_gang_name_updates_model`] — a `TextFieldCommitted` on the gang-name field updates
//!   the model name (AC3).
//!
//! Each test is pin-discriminating: a missing transition, a non-inserted / non-removed model, a
//! no-op add, or a dropped commit turns it red. Values are not pinned (the scaffold's default
//! member / name are implementation detail) — only the EFFECTS are asserted.

use bevy::{
    ecs::{component::Component, entity::Entity},
    prelude::With,
    state::state::NextState,
    ui::Interaction,
};
use gdtf_app::test_support::{
    AddMemberButton, AppState, EditableGang, EditorScreenRoot, GangNameField, MemberRow,
    RunningState,
};
use gdtf_test_utils::GdtfTestAppBuilder;
use gdtf_ui::{CommittedTextValue, TextFieldCommitted, theme::default_theme};

/// Builds a headless app driven into [`RunningState::DebugGangEditor`] with the editor screen
/// spawned: seed the theme before the first update so the `OnEnter` spawn sees it, start in
/// `AppState::Running` (whose default sub-state is `Menu`), then set the `DebugGangEditor`
/// transition and pump updates until the screen is up.
fn editor_app() -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    // Enter Running (rests on Menu, spawns the menu).
    app.update();
    // Drive Menu -> DebugGangEditor (the cfg-gated button does this in the GUI; here we set the
    // transition directly, the headless idiom — no window / mouse).
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::DebugGangEditor);
    // Apply the transition + run OnEnter (insert model + spawn screen), then one more update so
    // any deferred spawn-scene commands (themed title) flush.
    app.update();
    app.update();
    app
}

/// Counts the entities carrying marker `M` in the world.
fn count_with<M: Component>(app: &mut bevy::app::App) -> usize {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).count()
}

/// The single entity carrying marker `M`, if exactly one exists.
fn single_with<M: Component>(app: &mut bevy::app::App) -> Option<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// C1 / C2: entering `DebugGangEditor` inserts the editable model AND spawns the screen root.
///
/// Pin: if the `OnEnter` wiring is missing the model is never inserted (`EditableGang` absent) or
/// the root never spawns — either fails the assert.
#[test]
fn enter_editor_inserts_model_and_screen() {
    let mut app = editor_app();

    assert_eq!(
        app.world()
            .resource::<bevy::state::state::State<RunningState>>()
            .get(),
        &RunningState::DebugGangEditor,
        "the app must rest in RunningState::DebugGangEditor after the transition",
    );
    assert!(
        app.world().get_resource::<EditableGang>().is_some(),
        "OnEnter(DebugGangEditor) must insert the EditableGang model (AC2)",
    );
    assert!(
        single_with::<EditorScreenRoot>(&mut app).is_some(),
        "OnEnter(DebugGangEditor) must spawn exactly one editor screen root (C1)",
    );
}

/// AC4: pressing "Add member" appends a member to the model AND a member-row entity appears.
///
/// Pin: a no-op add (model count unchanged) or a missing row spawn fails the assert.
#[test]
fn add_member_grows_model_and_adds_row() {
    let mut app = editor_app();

    let before_count = app
        .world()
        .get_resource::<EditableGang>()
        .map(|model| model.members().len())
        .unwrap_or_default();
    let before_rows = count_with::<MemberRow>(&mut app);

    // Inject the press a real pointer would otherwise drive on the Add-member button.
    let button = single_with::<AddMemberButton>(&mut app).unwrap_or(Entity::PLACEHOLDER);
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = Interaction::Pressed;
    }
    app.update();

    let after_count = app
        .world()
        .get_resource::<EditableGang>()
        .map(|model| model.members().len())
        .unwrap_or_default();
    let after_rows = count_with::<MemberRow>(&mut app);

    assert_eq!(
        after_count,
        before_count + 1,
        "Add member must append exactly one member to the model (AC4)",
    );
    assert_eq!(
        after_rows,
        before_rows + 1,
        "Add member must spawn exactly one new member-list row (AC4)",
    );
}

/// C1: transitioning AWAY from `DebugGangEditor` despawns the screen root AND removes the model.
///
/// Pin: a missing `OnExit` remove leaves `EditableGang` present; a missing `DespawnOnExit` leaves
/// the root entity alive — either fails the assert.
#[test]
fn exit_editor_despawns_screen_and_removes_model() {
    let mut app = editor_app();
    // Precondition: both present in the editor.
    assert!(app.world().get_resource::<EditableGang>().is_some());
    assert!(single_with::<EditorScreenRoot>(&mut app).is_some());

    // Leave the editor (back to the menu).
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Menu);
    app.update();
    app.update();

    assert!(
        app.world().get_resource::<EditableGang>().is_none(),
        "OnExit(DebugGangEditor) must remove the EditableGang model resource (C1)",
    );
    assert_eq!(
        count_with::<EditorScreenRoot>(&mut app),
        0,
        "OnExit(DebugGangEditor) must despawn the editor screen root (C1)",
    );
}

/// AC3: a `TextFieldCommitted` on the gang-name field updates the model name.
///
/// Pin: if the commit listener drops the field-identity filter or never sets the name, the model
/// name stays unchanged and the assert fails. Value-agnostic on the START name; asserts the
/// committed value wins.
#[test]
fn commit_gang_name_updates_model() {
    let mut app = editor_app();
    let field = single_with::<GangNameField>(&mut app).unwrap_or(Entity::PLACEHOLDER);

    // Emit the commit the text-field widget raises on Enter / blur (the upstream seam GTW-411
    // owns; here we drive the action layer's real code path with the synthesized message).
    let committed = "Iron Skulls";
    app.world_mut().write_message(TextFieldCommitted::new(
        field,
        CommittedTextValue::new(committed),
    ));
    app.update();

    let model_name = app
        .world()
        .get_resource::<EditableGang>()
        .map(|model| model.name().as_str().to_owned());
    assert_eq!(
        model_name.as_deref(),
        Some(committed),
        "a commit on the gang-name field must set the model GangName to the committed value (AC3)",
    );
}
