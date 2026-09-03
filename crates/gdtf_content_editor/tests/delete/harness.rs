//! The editor app this suite drives, and the weapon delete entry its cases register.

use std::path::{Path, PathBuf};

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    asset::AssetPlugin,
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_assets::{ContentMemberKey, ContentSourcePaths, ContentValidationDone, FindingFamily};
use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry, WeaponSpec};
use gdtf_content_editor::{DeleteEntry, DeleteOutcome, EditorQaAssetsRoot, MapEditorPlugin};
use gdtf_content_families::WeaponsFamily;
use gdtf_test_utils::advance_until;

/// The finding family label every ranged weapon check writes.
pub(crate) const WEAPON_FAMILY: &str = "WeaponRegistry";

/// Enough updates for a delete to settle, and few enough to fail rather than hang.
pub(crate) const OUTCOME_UPDATES: usize = 16;

/// A headless editor app whose asset server reads `root` instead of the workspace assets.
pub(crate) fn editor_app_with_asset_root(root: &Path) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            }),
    );
    app.set_error_handler(warn);
    app.add_plugins(MapEditorPlugin);
    app
}

/// Run updates until the integrity report has been published.
pub(crate) fn advance_to_published(app: &mut App) {
    advance_until(app, |app| {
        app.world()
            .get_resource::<ContentValidationDone>()
            .is_some()
    });
}

/// Run at most [`OUTCOME_UPDATES`] updates, answering the outcome if one landed.
pub(crate) fn advance_to_outcome(app: &mut App) -> Option<DeleteOutcome> {
    for _ in 0..OUTCOME_UPDATES {
        app.update();
        if let Some(outcome) = app.world().get_resource::<DeleteOutcome>() {
            return Some(outcome.clone());
        }
    }
    None
}

/// Whether validation had republished by the time the advance gave up.
pub(crate) fn is_published(app: &App) -> bool {
    app.world()
        .get_resource::<ContentValidationDone>()
        .is_some()
}

/// A delete entry for ranged weapons, driving [`WeaponRegistry`] and the weapon's file.
pub(crate) fn weapon_delete_entry() -> DeleteEntry {
    DeleteEntry::new(
        FindingFamily::new(WEAPON_FAMILY.to_owned()),
        Box::new(|world, key| {
            let mut registry = world.get_resource_mut::<WeaponRegistry>()?;
            let spec = registry.remove(&WeaponName::new((**key).clone()))?;
            Some(Box::new(spec))
        }),
        Box::new(|world, key, record| {
            let Ok(spec) = record.downcast::<WeaponSpec>() else {
                return;
            };
            if let Some(mut registry) = world.get_resource_mut::<WeaponRegistry>() {
                registry.insert(WeaponName::new((**key).clone()), *spec);
            }
        }),
        Box::new(|world, key| weapon_file_path(world, key).map_or(Ok(()), std::fs::remove_file)),
    )
}

// The weapon's file under the save root, as its source path names it.
fn weapon_file_path(world: &World, key: &ContentMemberKey) -> Option<PathBuf> {
    let root = world.get_resource::<EditorQaAssetsRoot>()?;
    let sources = world.get_resource::<ContentSourcePaths<WeaponsFamily>>()?;
    let relative = sources.path(key)?;
    Some(root.join(&**relative))
}
