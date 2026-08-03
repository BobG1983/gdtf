use std::path::Path;

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
use gdtf_assets::{ContentFinding, ContentIntegrityReport, ContentValidationDone};
use gdtf_content_editor::MapEditorPlugin;
use gdtf_test_utils::advance_until;

pub(crate) const MAX_UPDATES: u32 = 10_000;

pub(crate) const REARM_UPDATES: u32 = 200;

pub(crate) const DANGLING_DEFAULT_FLOOR: &str = "00000000-0000-0000-0000-063000000001";

pub(crate) const DANGLING_PALETTE: &str = "00000000-0000-0000-0000-063000000002";

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

pub(crate) fn editor_app_on_fixture_root() -> App {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/validate_root");
    editor_app_with_asset_root(&root)
}

pub(crate) fn advance_to_published(app: &mut App) {
    let published = advance_until(
        app,
        |app| {
            app.world()
                .get_resource::<ContentValidationDone>()
                .is_some()
        },
        MAX_UPDATES,
    );
    assert!(
        published,
        "the editor never published the content-integrity report — the GTW-630 authoring-time \
         validation pass is not registered (ContentValidationDone was never stamped)",
    );
    for _ in 0..4 {
        app.update();
    }
}

pub(crate) fn has_dangling_ref(
    report: &ContentIntegrityReport,
    family: &str,
    target: &str,
) -> bool {
    dangling_ref_referrer(report, family, target).is_some()
}

pub(crate) fn has_malformed(report: &ContentIntegrityReport, stem: &str) -> bool {
    report.findings().iter().any(|finding| {
        matches!(
            finding,
            ContentFinding::MalformedFile { path, .. } if path.contains(stem)
        )
    })
}

pub(crate) fn dangling_ref_referrer(
    report: &ContentIntegrityReport,
    family: &str,
    target: &str,
) -> Option<String> {
    report.findings().iter().find_map(|finding| match finding {
        ContentFinding::DanglingRef {
            referrer,
            target: found_target,
            family: found_family,
            ..
        } if **found_target == *target && **found_family == *family => Some((**referrer).clone()),
        _ => None,
    })
}
