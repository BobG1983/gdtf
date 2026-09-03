//! the GANG equipment-refs edge joins the editor's authoring-time
use std::path::Path;

use bevy::asset::{AssetEvent, AssetServer, Assets};
use gdtf_assets::{
    ContentFamily, ContentFinding, ContentFolderHandle, ContentIntegrityReport, ContentSourcePaths,
    ReferenceKeyScheme, RonAsset,
};
use gdtf_battle_sim::{
    ganger::{GangName, GangRoster},
    weapon::{FISTS_KEY, WeaponName},
};
use gdtf_content_editor::gang_file_name;
use gdtf_content_families::GangsFamily;
use gdtf_test_utils::advance_until;

use crate::harness::{
    DANGLING_DEFAULT_FLOOR, advance_to_published, dangling_ref_referrer,
    editor_app_on_fixture_root, has_dangling_ref,
};

const FIXTURE_GANG_STEM: &str = "fixture_gang";

const DANGLING_WEAPON: &str = "missing_weapon";

const DANGLING_ARMOR: &str = "missing_armor";

const EDITED_DANGLING_WEAPON: &str = "edited_missing_weapon";

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

    let referrer = dangling_ref_referrer(
        report,
        "WeaponRegistry",
        DANGLING_WEAPON,
        ReferenceKeyScheme::FileStem,
    );
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
        has_dangling_ref(
            report,
            "ArmorRegistry",
            DANGLING_ARMOR,
            ReferenceKeyScheme::FileStem,
        ),
        "the gang member's dangling armor key must be reported at authoring time; report: {:?}",
        report.findings(),
    );
    assert!(
        has_dangling_ref(
            report,
            "MeleeWeaponRegistry",
            FISTS_KEY,
            ReferenceKeyScheme::FileStem,
        ),
        "the melee-less member's implicit `fists` default must be reported dangling (no melee \
         folder in this root); report: {:?}",
        report.findings(),
    );
}

#[test]
fn gang_hot_edit_rearms_validation_and_republishes_current_findings() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<GangRoster>>(format!(
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
        "the loader's persistent gangs ContentFolderHandle must survive past Load ",
    );
    app.world_mut()
        .write_message(AssetEvent::Modified { id: handle.id() });

    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<ContentIntegrityReport>()
            .is_some_and(|report| {
                has_dangling_ref(
                    report,
                    "WeaponRegistry",
                    EDITED_DANGLING_WEAPON,
                    ReferenceKeyScheme::FileStem,
                )
            })
    });

    let world = app.world();
    let report = world.resource::<ContentIntegrityReport>();
    assert!(
        !has_dangling_ref(
            report,
            "WeaponRegistry",
            DANGLING_WEAPON,
            ReferenceKeyScheme::FileStem,
        ),
        "the report must be RESET and re-checked on re-arm — the superseded weapon finding must \
         not persist; report: {:?}",
        report.findings(),
    );
    assert!(
        has_dangling_ref(
            report,
            "TerrainDefRegistry",
            DANGLING_DEFAULT_FLOOR,
            ReferenceKeyScheme::Uuid,
        ),
        "the gang re-arm must re-run EVERY registered check onto the one report — the theme's \
         untouched dangling default_floor must be re-reported; report: {:?}",
        report.findings(),
    );
}

#[test]
fn a_dangling_gang_weapon_ref_names_the_gang_record_and_its_source_file() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    let world = app.world();
    let report = world.get_resource::<ContentIntegrityReport>();
    assert!(
        report.is_some(),
        "the ContentIntegrityReport resource must exist in the editor app",
    );
    let Some(report) = report else { return };

    let referring = report.findings().iter().find_map(|finding| match finding {
        ContentFinding::DanglingRef {
            referring_record,
            target,
            family,
            ..
        } if **target == *DANGLING_WEAPON && **family == *"WeaponRegistry" => {
            Some(referring_record.clone())
        }
        _ => None,
    });
    assert!(
        referring.is_some(),
        "the gang member's dangling weapon key must be reported; report: {:?}",
        report.findings(),
    );
    let Some(referring) = referring else { return };

    assert_eq!(
        &*referring.family, "GangRegistry",
        "the referring record must name the gang registry",
    );
    assert_eq!(
        &*referring.key, FIXTURE_GANG_STEM,
        "the referring record must carry the key GangsFamily::insert_member returned",
    );
    assert_eq!(
        &*referring.field, "members[].weapon",
        "the referring record must name the field holding the reference",
    );

    let sources = world.get_resource::<ContentSourcePaths<GangsFamily>>();
    assert!(
        sources.is_some(),
        "the gangs family must publish its source paths beside its registry",
    );
    let Some(sources) = sources else { return };

    let expected = Path::new(GangsFamily::FOLDER)
        .join(gang_file_name(&GangName::new(FIXTURE_GANG_STEM.to_owned())));
    let found = sources.path(&referring.key);
    assert_eq!(
        found.map(|path| (**path).clone()),
        Some(expected.clone()),
        "the referring record's key must resolve to the gang's source file `{}`",
        expected.display(),
    );
}
