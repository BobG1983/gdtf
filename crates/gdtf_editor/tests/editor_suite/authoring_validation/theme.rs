//! A2: the theme→terrain edge's authoring-time pins — a dangling
use bevy::asset::{AssetEvent, AssetServer, Assets, uuid::Uuid};
use cobalt_ron_assets::RonAsset;
use cobalt_test_utils::advance_until;
use gdtf_assets::{ContentFamily, ContentFolderHandle, ContentIntegrityReport, ReferenceKeyScheme};
use gdtf_battle_sim::{level::UuidThemeDef, terrain::def::TerrainUuid};
use gdtf_content_families::ThemeDefsFamily;

use crate::{
    authoring_validation::harness::{
        DANGLING_DEFAULT_FLOOR, DANGLING_PALETTE, editor_app_on_fixture_root,
    },
    content_shared::{advance::advance_to_published, findings::has_dangling_ref},
};

const EDITED_DEFAULT_FLOOR: u128 = 0x0000_0000_0000_0000_0000_0630_0000_0003;

fn has_dangling_terrain_ref(report: &ContentIntegrityReport, uuid: &str) -> bool {
    has_dangling_ref(report, "TerrainDefRegistry", uuid, ReferenceKeyScheme::Uuid)
}

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

#[test]
fn theme_hot_edit_rearms_validation_and_republishes_current_findings() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<UuidThemeDef>>(format!(
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
        "the loader's persistent theme-defs ContentFolderHandle must survive past Load ",
    );
    app.world_mut()
        .write_message(AssetEvent::Modified { id: handle.id() });

    let edited_uuid = TerrainUuid::new(Uuid::from_u128(EDITED_DEFAULT_FLOOR)).to_string();
    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<ContentIntegrityReport>()
            .is_some_and(|report| has_dangling_terrain_ref(report, &edited_uuid))
    });

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
