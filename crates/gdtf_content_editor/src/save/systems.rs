//! The **Bevy systems** and **filesystem writer** for the save path (GTW-432; swept onto the v2
//! UUID schema in GTW-495).
//!
//! - [`write_prefab`] — the pure IO layer: sanitize name → project → serialize → `fs::write`.
//! - [`spawn_save_controls`] — `OnEnter(Editing)` system: spawns the prefab-name text field +
//!   "Save prefab" button under the right panel.
//! - [`save_prefab_on_press`] — `Update` system: drives the write on a button press.

use std::path::PathBuf;

use bevy::{prelude::*, ui::Interaction};
use gdtf_battle_sim::{level::UuidThemeRegistry, terrain::def::TerrainDefRegistry};
use gdtf_ui::{ActiveButton, CommittedTextValue, FieldColors, spawn_text_field, theme::GdtfTheme};

use super::{
    project::{editor_map_to_prefab, prefab_save_path, sanitize_name, serialize_prefab},
    types::{PrefabNameField, SavePrefabButton, SavePrefabError},
};
use crate::{
    EditorMap, mode::PrefabModeContent, mode_host::mode_host_under_region,
    regions::RightPanelRegion, session::MapEditorSession,
};

/// Build + serialize + WRITE the prefab to
/// `assets/maps/<theme>/<size>/<stem>.prefab_v2.ron` (C1), or return the typed
/// [`SavePrefabError`] (never a panic).
///
/// Sanitizes the entered name, projects the map to a [`PrefabSpecV2`](gdtf_battle_sim::level::PrefabSpecV2)
/// (C3 illegal-cell guard inside [`editor_map_to_prefab`]), serializes it, resolves the theme's
/// directory from its display name in the [`UuidThemeRegistry`], creates the themed / sized
/// directory if absent, and writes the file. Returns the resolved [`PathBuf`] on success so the
/// caller can log it; a test reuses this whole path.
///
/// # Errors
///
/// Any [`SavePrefabError`] from name validation, the map projection, serialization, or the file
/// write.
pub(crate) fn write_prefab(
    raw_name: &str,
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    themes: &UuidThemeRegistry,
    session: &MapEditorSession,
) -> Result<PathBuf, SavePrefabError> {
    let stem = sanitize_name(raw_name);
    if stem.is_empty() {
        return Err(SavePrefabError::EmptyName);
    }
    let spec = editor_map_to_prefab(map, registry, session)?;
    let serialized = serialize_prefab(&spec)?;
    let theme_display = themes
        .def(&session.theme())
        .map_or_else(String::new, |def| (*def.display_name).clone());
    let path = prefab_save_path(&theme_display, session.grid_size(), &stem);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|err| SavePrefabError::Write(err.to_string()))?;
    }
    std::fs::write(&path, serialized).map_err(|err| SavePrefabError::Write(err.to_string()))?;
    Ok(path)
}

/// `OnEnter(Editing)`: spawn the prefab-NAME text field + the "Save prefab" button under the
/// right panel (the GTW-411 widget + the GTW-432 save trigger).
pub(crate) fn spawn_save_controls(mut commands: Commands, theme: Res<GdtfTheme>) {
    let field_colors = FieldColors {
        background: *theme.panel.color,
        text:       *theme.text.text_color,
        caret:      *theme.text.text_color,
    };
    let label_color = *theme.text.text_color;

    let label = commands
        .spawn((
            Text::new("Prefab name"),
            TextColor(label_color),
            Node {
                margin: UiRect::bottom(Val::Vh(0.2)),
                ..default()
            },
        ))
        .id();
    let name_field = spawn_text_field(
        &mut commands,
        CommittedTextValue::new(""),
        field_colors,
        PrefabNameField,
    );
    let save_button = commands
        .spawn((
            SavePrefabButton,
            Button,
            BackgroundColor(*theme.panel.border_color),
            Node {
                margin: UiRect::top(Val::Vh(0.6)),
                padding: UiRect::all(Val::Vh(0.6)),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_child((Text::new("Save prefab"), TextColor(label_color)))
        .id();

    let group = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            margin: UiRect::top(Val::Vh(1.5)),
            ..default()
        })
        .add_children(&[label, name_field, save_button])
        .id();

    commands.queue(move |world: &mut World| {
        // The save controls are PREFAB-mode content; hang them on the right panel's PREFAB-mode
        // container so they hide in TERRAIN mode (GTW-474, the GTW-432 save trigger).
        let Some(host) = mode_host_under_region::<RightPanelRegion, PrefabModeContent>(world)
        else {
            return;
        };
        if let Ok(mut host_entity) = world.get_entity_mut(host) {
            host_entity.add_child(group);
        }
    });
}

/// The [`Query`] of "Save prefab" buttons that changed [`Interaction`] this frame — named so
/// [`save_prefab_on_press`]'s signature stays under the clippy `type_complexity` threshold.
type ChangedSaveButtons<'w, 's> =
    Query<'w, 's, (Entity, &'static Interaction), (Changed<Interaction>, With<SavePrefabButton>)>;

/// `Update` (in `Editing`): WRITE the prefab on a "Save prefab" press (C1 — the live trigger).
///
/// Reads the [`SavePrefabButton`]'s [`Interaction`] edge, reads the entered name from the
/// [`PrefabNameField`]'s last-committed [`CommittedTextValue`], and calls [`write_prefab`] with
/// the current map / registry / theme registry / session. On success it adds the [`ActiveButton`]
/// highlight and logs the written path; on a typed [`SavePrefabError`] it logs an `error!` and
/// writes nothing (C3). All borrows are `Option` (the map / session are state-scoped resources —
/// `bevy-traps.md` #1).
pub(crate) fn save_prefab_on_press(
    mut commands: Commands,
    buttons: ChangedSaveButtons,
    name_fields: Query<&CommittedTextValue, With<PrefabNameField>>,
    map: Option<Res<EditorMap>>,
    registry: Option<Res<TerrainDefRegistry>>,
    themes: Option<Res<UuidThemeRegistry>>,
    session: Option<Res<MapEditorSession>>,
) {
    let Some((button, _)) = buttons
        .iter()
        .find(|(_, interaction)| **interaction == Interaction::Pressed)
    else {
        return;
    };
    let (Some(map), Some(registry), Some(themes), Some(session)) = (map, registry, themes, session)
    else {
        return;
    };
    let name = name_fields
        .iter()
        .next()
        .map_or_else(String::new, |value| value.value().to_owned());

    match write_prefab(&name, &map, &registry, &themes, &session) {
        Ok(path) => {
            info!("prefab save: wrote prefab to `{}`", path.display());
            commands.entity(button).insert(ActiveButton);
        }
        Err(err) => error!("prefab save: {err}"),
    }
}
