//! GTW-630 A2: the theme→terrain edge's authoring-time pins — a dangling
//! terrain UUID surfaces in the EDITOR's report at launch, and a hot-edit of
//! the loaded theme re-arms the pass (reset, re-check, re-publish).

use bevy::asset::{AssetEvent, AssetServer, Assets, uuid::Uuid};
use gdtf_assets::{ContentFamily, ContentFolderHandle, ContentIntegrityReport, RonAsset};
use gdtf_battle_sim::{level::UuidThemeDef, terrain::def::TerrainUuid};
use gdtf_content_families::ThemeDefsFamily;
use gdtf_test_utils::advance_until;

use crate::harness::{
    DANGLING_DEFAULT_FLOOR, DANGLING_PALETTE, REARM_UPDATES, advance_to_published,
    editor_app_on_fixture_root, has_dangling_ref,
};

/// The DISTINCT dangling terrain UUID the re-arm test hot-edits the theme's
/// `default_floor` to (in-memory only — the fixture file is never written).
const EDITED_DEFAULT_FLOOR: u128 = 0x0000_0000_0000_0000_0000_0630_0000_0003;

/// Whether `report` holds a `DanglingRef` finding for the given terrain UUID
/// against the `TerrainDefRegistry` family — the theme→terrain edge's shape.
fn has_dangling_terrain_ref(report: &ContentIntegrityReport, uuid: &str) -> bool {
    has_dangling_ref(report, "TerrainDefRegistry", uuid)
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
