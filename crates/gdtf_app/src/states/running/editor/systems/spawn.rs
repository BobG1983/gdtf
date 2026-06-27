//! Spawns the gang-editor screen on `OnEnter(RunningState::DebugEditor)` (GTW-420).
//!
//! Builds a themed panel layout holding:
//!
//! - a **title** heading,
//! - the gang-NAME text field via [`spawn_text_field`](gdtf_ui::spawn_text_field) (carrying the
//!   [`GangNameField`] marker so its commit maps to the model name — AC3),
//! - an **"Add member"** button via [`spawn_button`](gdtf_ui::spawn_button) (the
//!   [`AddMemberButton`] marker — AC4),
//! - the member-list SHELL host ([`MemberListHost`]), seeded with one [`MemberRow`] per member
//!   already in the model (the load-from-registry path may carry members).
//!
//! Every entity carries [`DespawnOnExit(RunningState::DebugEditor)`](bevy::prelude::DespawnOnExit)
//! so the whole screen tears down on leave (C1). The model resource is inserted by a sibling
//! system ([`insert_editable_gang`](super::model_lifecycle::insert_editable_gang)) ordered before
//! this one, so the member count is known when the shell is seeded.

use bevy::{prelude::*, scene::CommandsSceneExt, ui::Val};
use gdtf_ui::{
    ButtonLabel, CommittedTextValue, FieldColors, spawn_button, spawn_panel, spawn_text_field,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::states::{
    RunningState,
    running::editor::{
        components::{AddMemberButton, EditorScreenRoot, GangNameField, MemberListHost, MemberRow},
        model::EditableGang,
    },
};

/// Inter-child vertical gap of the editor column, in `Vh` — relative units (the responsive-UI
/// rule), reusing the menu's calibrated `10px / 720` separation so the editor spacing matches.
const EDITOR_GAP_VH: f32 = 1.388_89;

/// Spawns the full editor screen when [`RunningState::DebugEditor`] is entered.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1) — in the running app the theme is present by the time the menu can
/// reach the editor. Reads the [`EditableGang`] model (inserted by the ordered-before sibling
/// system) as `Option<Res<…>>` so it is robust if the model is somehow absent. Each spawned
/// node carries [`DespawnOnExit(RunningState::DebugEditor)`](bevy::prelude::DespawnOnExit).
pub(in crate::states::running::editor) fn spawn_editor_screen(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    model: Option<Res<EditableGang>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed screen (the menu precedent).
        return;
    };

    // ROOT — a centered, full-viewport themed backdrop column.
    let root = commands
        .spawn((
            EditorScreenRoot,
            Themed::new(ThemeRole::Background),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Vh(EDITOR_GAP_VH),
                ..default()
            },
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();

    // TITLE — a themed heading, a direct child of the root.
    let title = commands
        .spawn_scene((
            bevy::scene::bsn! {
                Themed::new(ThemeRole::Title)
                Text::new("GANG EDITOR")
            },
            bevy::scene::template_value(DespawnOnExit(RunningState::DebugEditor)),
        ))
        .id();

    // PANEL — wraps the editing controls in a themed box, a flex column.
    let panel = spawn_panel(&mut commands, &theme);
    commands.entity(panel).insert((
        DespawnOnExit(RunningState::DebugEditor),
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: Val::Vh(EDITOR_GAP_VH),
            ..default()
        },
    ));

    // GANG-NAME field — seeded with the model's current name, carrying the GangNameField marker
    // so the commit listener maps it to the model name (AC3).
    let initial_name = model
        .as_ref()
        .map(|model| model.name().as_str().to_owned())
        .unwrap_or_default();
    let name_field = spawn_text_field(
        &mut commands,
        CommittedTextValue::new(initial_name),
        field_colors(&theme),
        (GangNameField, DespawnOnExit(RunningState::DebugEditor)),
    );

    // ADD-MEMBER button (AC4).
    let add_button = spawn_button(
        &mut commands,
        &theme,
        ButtonLabel::new("Add member"),
        (AddMemberButton, DespawnOnExit(RunningState::DebugEditor)),
    );

    // MEMBER-LIST shell host — the container the per-member rows live under.
    let list_host = commands
        .spawn((
            MemberListHost,
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: Val::Vh(EDITOR_GAP_VH),
                ..default()
            },
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id();

    // Seed one minimal row per member already in the model (the load-from-registry path).
    if let Some(model) = model.as_ref() {
        for member in model.members() {
            let row = spawn_member_row(&mut commands, &theme, member.name().as_str());
            commands.entity(list_host).add_child(row);
        }
    }

    commands
        .entity(panel)
        .add_children(&[name_field, add_button, list_host]);
    commands.entity(root).add_children(&[title, panel]);
}

/// Spawns ONE minimal-but-real member-list row (a themed text node carrying [`MemberRow`]),
/// returning its [`Entity`] (GTW-420 SCAFFOLD scope — the rich row is GTW-425).
pub(in crate::states::running::editor) fn spawn_member_row(
    commands: &mut Commands,
    _theme: &GdtfTheme,
    name: &str,
) -> Entity {
    commands
        .spawn((
            MemberRow,
            Themed::new(ThemeRole::Text),
            Text::new(name.to_owned()),
            DespawnOnExit(RunningState::DebugEditor),
        ))
        .id()
}

/// The [`FieldColors`] the gang-name text field paints with, read from the theme (the field
/// background / text from the panel + body-text sub-themes, the caret from the body text). Pure
/// UI plumbing colors, not domain values.
fn field_colors(theme: &GdtfTheme) -> FieldColors {
    FieldColors {
        background: *theme.panel.color,
        text:       *theme.text.text_color,
        caret:      *theme.text.text_color,
    }
}
