//! The suite's shared plumbing: the real editor app rooted at an arbitrary
//! assets directory, the publish driver, and the report probes.

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

/// A generous frame cap for the async fixture load — a SAFETY NET, not a timing
/// budget (the tests poll the [`ContentValidationDone`] signal).
pub(crate) const MAX_UPDATES: u32 = 10_000;

/// A cap for the post-edit redrive → re-arm → re-publish settle (polled by
/// signal, never slept).
pub(crate) const REARM_UPDATES: u32 = 200;

/// The fixture theme's DANGLING `default_floor` terrain UUID (no terrain def in
/// the fixture root defines it) — shared by the theme pins and the gangs
/// re-arm pin (the whole-pass re-check re-reports it).
pub(crate) const DANGLING_DEFAULT_FLOOR: &str = "00000000-0000-0000-0000-063000000001";

/// The fixture theme's DANGLING palette-entry terrain UUID.
pub(crate) const DANGLING_PALETTE: &str = "00000000-0000-0000-0000-063000000002";

/// The real editor app rooted at an ARBITRARY assets directory (the
/// `load_seam.rs` recipe): only the folders the fixture root materializes load;
/// every other family fails closed to its empty registry (the no-strand
/// guarantee). Local to this suite because the shared harness deliberately pins
/// the workspace root.
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
            // Headless-test noise suppression (GTW-139): no global tracing
            // subscriber, so the DELIBERATE missing-folder asset errors these
            // fixture roots exercise do not print.
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
    // Bevy 0.19 routes a FAILED system-param validation to the global error
    // handler (default panics); with no render backend some render-provided
    // params cannot validate. `warn` restores the skip-with-a-log behavior
    // (the shared harness precedent).
    app.set_error_handler(warn);
    app.add_plugins(MapEditorPlugin);
    app
}

/// The editor app rooted at the committed `validate_root` fixture — one theme
/// whose terrain references all dangle plus one gang whose equipment keys all
/// dangle.
pub(crate) fn editor_app_on_fixture_root() -> App {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/validate_root");
    editor_app_with_asset_root(&root)
}

/// Drives the app until the validation pass PUBLISHED ([`ContentValidationDone`]
/// stamped), then a few settle frames so command flushes apply.
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

/// Whether `report` holds a `DanglingRef` finding for the given `target` key
/// against the given registry `family` — the per-edge finding shape every
/// check in `gdtf_content_families::validate` emits.
pub(crate) fn has_dangling_ref(
    report: &ContentIntegrityReport,
    family: &str,
    target: &str,
) -> bool {
    dangling_ref_referrer(report, family, target).is_some()
}

/// The REFERRER text of the `DanglingRef` finding for the given `target` key
/// against the given registry `family`, or [`None`] when no such finding is on
/// the report — so a test can pin WHO the finding names (A1: the gang file),
/// not just that it exists.
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
