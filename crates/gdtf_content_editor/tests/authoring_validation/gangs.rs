//! GTW-651: the GANG equipment-refs edge joins the editor's authoring-time
//! validation — a fixture gang with dangling equipment keys surfaces at editor
//! launch (C3(a) / A1), and a hot-edit of the loaded gang re-arms the pass and
//! re-publishes the CURRENT findings (C3(b)).

use bevy::asset::{AssetEvent, AssetServer, Assets};
use gdtf_assets::{ContentFamily, ContentFolderHandle, ContentIntegrityReport, RonAsset};
use gdtf_battle_sim::{
    ganger::GangRoster,
    weapon::{FISTS_KEY, WeaponName},
};
use gdtf_content_families::GangsFamily;
use gdtf_test_utils::advance_until;

use crate::harness::{
    DANGLING_DEFAULT_FLOOR, REARM_UPDATES, advance_to_published, dangling_ref_referrer,
    editor_app_on_fixture_root, has_dangling_ref,
};

/// The committed fixture gang's file STEM — its registry key, which the
/// finding's referrer must name (A1: "findings name the gang file").
const FIXTURE_GANG_STEM: &str = "fixture_gang";

/// The fixture gang member's DANGLING weapon key (no weapons folder in the
/// fixture root).
const DANGLING_WEAPON: &str = "missing_weapon";

/// The fixture gang member's DANGLING armor key.
const DANGLING_ARMOR: &str = "missing_armor";

/// The DISTINCT dangling weapon key the re-arm test hot-edits the member to
/// (in-memory only — the fixture file is never written).
const EDITED_DANGLING_WEAPON: &str = "edited_missing_weapon";

/// C3(a) / A1: loading a gang whose member's equipment keys resolve nothing
/// surfaces each as a `DanglingRef` finding on the [`ContentIntegrityReport`]
/// in the EDITOR app — naming the gang file (the referrer) and the missing key
/// (the target). The implicit `fists` melee default the member falls back to
/// is reported too (the check's documented default-edge behavior).
#[test]
fn dangling_gang_equipment_refs_surface_in_the_editor_at_authoring_time() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    let report = app.world().get_resource::<ContentIntegrityReport>();
    assert!(
        report.is_some(),
        "the ContentIntegrityReport resource must exist in the editor app",
    );
    let Some(report) = report else { return };

    // The weapon-key finding names the gang file + the missing key (A1).
    let referrer = dangling_ref_referrer(report, "WeaponRegistry", DANGLING_WEAPON);
    assert!(
        referrer.is_some(),
        "the gang member's dangling weapon key must be reported at authoring time; report: {:?}",
        report.findings(),
    );
    let Some(referrer) = referrer else { return };
    assert!(
        referrer.contains(FIXTURE_GANG_STEM),
        "the weapon finding's referrer must name the gang file (`{FIXTURE_GANG_STEM}`); \
         referrer: {referrer}",
    );

    assert!(
        has_dangling_ref(report, "ArmorRegistry", DANGLING_ARMOR),
        "the gang member's dangling armor key must be reported at authoring time; report: {:?}",
        report.findings(),
    );
    assert!(
        has_dangling_ref(report, "MeleeWeaponRegistry", FISTS_KEY),
        "the melee-less member's implicit `fists` default must be reported dangling (no melee \
         folder in this root); report: {:?}",
        report.findings(),
    );
}

/// C3(b): hot-editing the loaded gang (the hot-reload redrive path — the same
/// `Modified` message the file watcher emits) re-arms the pass: the report is
/// RESET, re-checked against the edited content, and re-published. The
/// superseded weapon finding is gone, the edited (still-dangling) key is
/// present — and the untouched THEME's finding is re-reported, because the
/// re-arm re-runs EVERY registered check onto the ONE consolidated report
/// (the A2 same-report behavior).
#[test]
fn gang_hot_edit_rearms_validation_and_republishes_current_findings() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    // Hot-edit the loaded gang IN MEMORY (the load_redrive.rs recipe), then
    // fire the same `Modified` message the file watcher emits.
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<GangRoster>>(format!(
            // GTW-634 A1: folder + extension are DERIVED from the family's owning consts.
            "{}/{FIXTURE_GANG_STEM}.{}",
            GangsFamily::FOLDER,
            GangsFamily::EXTENSION,
        ));
    {
        let mut gangs = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<GangRoster>>>();
        let asset = gangs.get_mut(&handle);
        assert!(
            asset.is_some(),
            "the fixture gang member must be resident once the pass published",
        );
        let Some(mut asset) = asset else { return };
        let member = asset.members.first_mut();
        assert!(member.is_some(), "the fixture gang must have one member");
        let Some(member) = member else { return };
        member.weapon = WeaponName::new(EDITED_DANGLING_WEAPON.to_owned());
    }
    assert!(
        app.world()
            .get_resource::<ContentFolderHandle<GangsFamily>>()
            .is_some(),
        "the loader's persistent gangs ContentFolderHandle must survive past Load (GTW-533)",
    );
    app.world_mut()
        .write_message(AssetEvent::Modified { id: handle.id() });

    let republished = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<ContentIntegrityReport>()
                .is_some_and(|report| {
                    has_dangling_ref(report, "WeaponRegistry", EDITED_DANGLING_WEAPON)
                })
        },
        REARM_UPDATES,
    );
    assert!(
        republished,
        "a gang hot-edit must re-arm the validation pass — the edited dangling weapon key was \
         never re-reported",
    );

    let world = app.world();
    let report = world.resource::<ContentIntegrityReport>();
    assert!(
        !has_dangling_ref(report, "WeaponRegistry", DANGLING_WEAPON),
        "the report must be RESET and re-checked on re-arm — the superseded weapon finding must \
         not persist; report: {:?}",
        report.findings(),
    );
    assert!(
        has_dangling_ref(report, "TerrainDefRegistry", DANGLING_DEFAULT_FLOOR),
        "the gang re-arm must re-run EVERY registered check onto the one report — the theme's \
         untouched dangling default_floor must be re-reported; report: {:?}",
        report.findings(),
    );
}
