//! GTW-630 headless pins for the editor's AUTHORING-TIME reference validation.
//!
//! A2: the editor app registers the GTW-582 validation pass over the edges it
//! loads (theme→terrain + emplacement→weapon, through the SAME
//! `gdtf_content_families::validate` checks the game registers), so a theme
//! referencing a missing terrain UUID surfaces on the
//! [`ContentIntegrityReport`] IN THE EDITOR — at authoring time, not on the
//! next game launch.
//!
//! The second pin covers the LIVE half of authoring time: a hot-edit of a
//! loaded theme (the seam redrive) RE-ARMS the pass — the report is reset,
//! re-checked against the CURRENT content, and re-published — so a dangling
//! key authored mid-session is reported at the edit, not on the next editor
//! launch.

use std::path::Path;

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    asset::{AssetEvent, AssetPlugin, AssetServer, Assets, uuid::Uuid},
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_assets::{
    ContentFamily, ContentFinding, ContentFolderHandle, ContentIntegrityReport,
    ContentValidationDone, RonAsset,
};
use gdtf_battle_sim::{level::UuidThemeDef, terrain::def::TerrainUuid};
use gdtf_content_editor::MapEditorPlugin;
use gdtf_content_families::ThemeDefsFamily;
use gdtf_test_utils::advance_until;

/// A generous frame cap for the async fixture load — a SAFETY NET, not a timing
/// budget (the tests poll the [`ContentValidationDone`] signal).
const MAX_UPDATES: u32 = 10_000;

/// A cap for the post-edit redrive → re-arm → re-publish settle (polled by
/// signal, never slept).
const REARM_UPDATES: u32 = 200;

/// The fixture theme's DANGLING `default_floor` terrain UUID (no terrain def in
/// the fixture root defines it).
const DANGLING_DEFAULT_FLOOR: &str = "00000000-0000-0000-0000-063000000001";

/// The fixture theme's DANGLING palette-entry terrain UUID.
const DANGLING_PALETTE: &str = "00000000-0000-0000-0000-063000000002";

/// The DISTINCT dangling terrain UUID the re-arm test hot-edits the theme's
/// `default_floor` to (in-memory only — the fixture file is never written).
const EDITED_DEFAULT_FLOOR: u128 = 0x0000_0000_0000_0000_0000_0630_0000_0003;

/// The real editor app rooted at the committed `validate_root` fixture — one
/// theme whose terrain references all dangle. Mirrors the `load_seam.rs`
/// arbitrary-root plugin recipe; local to this test because the shared harness
/// deliberately pins the workspace root.
fn editor_app_on_fixture_root() -> App {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/validate_root");
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
            // subscriber, so the DELIBERATE missing-folder asset errors this
            // fixture root exercises do not print.
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

/// Drives the app until the validation pass PUBLISHED ([`ContentValidationDone`]
/// stamped), then a few settle frames so command flushes apply.
fn advance_to_published(app: &mut App) {
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

/// Whether `report` holds a `DanglingRef` finding for the given terrain UUID
/// against the `TerrainDefRegistry` family — the theme→terrain edge's shape.
fn has_dangling_terrain_ref(report: &ContentIntegrityReport, uuid: &str) -> bool {
    report.findings().iter().any(|finding| {
        matches!(
            finding,
            ContentFinding::DanglingRef { target, family, .. }
                if **target == *uuid && **family == *"TerrainDefRegistry"
        )
    })
}

/// A2: loading a theme whose `default_floor` + palette entry reference missing
/// terrain UUIDs surfaces BOTH as `DanglingRef` findings on the
/// [`ContentIntegrityReport`] in the EDITOR app — the same report shape the
/// game publishes at the end of `Load`.
#[test]
fn dangling_theme_terrain_refs_surface_in_the_editor_at_authoring_time() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    let report = app.world().get_resource::<ContentIntegrityReport>();
    assert!(
        report.is_some(),
        "the ContentIntegrityReport resource must exist in the editor app",
    );
    let Some(report) = report else { return };
    assert!(
        has_dangling_terrain_ref(report, DANGLING_DEFAULT_FLOOR),
        "the theme's dangling default_floor UUID must be reported at authoring time; report: \
         {:?}",
        report.findings(),
    );
    assert!(
        has_dangling_terrain_ref(report, DANGLING_PALETTE),
        "the theme's dangling palette-entry UUID must be reported at authoring time; report: {:?}",
        report.findings(),
    );
}

/// The LIVE authoring half: hot-editing the loaded theme (the seam redrive
/// path) re-arms the pass — the report is RESET, re-checked against the edited
/// content, and re-published. The superseded `default_floor` finding is gone,
/// the edited (still-dangling) one is present, and the untouched palette
/// finding is re-reported.
#[test]
fn theme_hot_edit_rearms_validation_and_republishes_current_findings() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    // Hot-edit the loaded theme IN MEMORY (the load_redrive.rs recipe), then
    // fire the same `Modified` message the file watcher emits.
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<UuidThemeDef>>(format!(
            // GTW-634 A1: the folder segment is DERIVED from the family's owning const.
            "{}/fixture_theme/fixture_theme.terrain_theme.ron",
            ThemeDefsFamily::FOLDER
        ));
    {
        let mut themes = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<UuidThemeDef>>>();
        let asset = themes.get_mut(&handle);
        assert!(
            asset.is_some(),
            "the fixture theme member must be resident once the pass published",
        );
        let Some(mut asset) = asset else { return };
        asset.default_floor = TerrainUuid::new(Uuid::from_u128(EDITED_DEFAULT_FLOOR));
    }
    assert!(
        app.world()
            .get_resource::<ContentFolderHandle<ThemeDefsFamily>>()
            .is_some(),
        "the seam's persistent theme-defs ContentFolderHandle must survive past Load (GTW-533)",
    );
    app.world_mut()
        .write_message(AssetEvent::Modified { id: handle.id() });

    let edited_uuid = TerrainUuid::new(Uuid::from_u128(EDITED_DEFAULT_FLOOR)).to_string();
    let republished = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<ContentIntegrityReport>()
                .is_some_and(|report| has_dangling_terrain_ref(report, &edited_uuid))
        },
        REARM_UPDATES,
    );
    assert!(
        republished,
        "a theme hot-edit must re-arm the validation pass — the edited dangling default_floor \
         was never re-reported",
    );

    let world = app.world();
    let report = world.resource::<ContentIntegrityReport>();
    assert!(
        !has_dangling_terrain_ref(report, DANGLING_DEFAULT_FLOOR),
        "the report must be RESET and re-checked on re-arm — the superseded default_floor \
         finding must not persist; report: {:?}",
        report.findings(),
    );
    assert!(
        has_dangling_terrain_ref(report, DANGLING_PALETTE),
        "the still-dangling palette entry must be re-reported after the re-check; report: {:?}",
        report.findings(),
    );
}
