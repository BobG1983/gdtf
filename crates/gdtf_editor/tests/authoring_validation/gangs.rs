//! the GANG equipment-refs edge joins the editor's authoring-time
use std::path::Path;

use bevy::asset::{AssetEvent, AssetServer, Assets};
use cobalt_ron_assets::RonAsset;
use cobalt_test_utils::advance_until;
use gdtf_assets::{
    ContentFamily, ContentFinding, ContentFolderHandle, ContentIntegrityReport, ContentSourcePaths,
    ReferenceKeyScheme,
};
use gdtf_battle_sim::{
    ganger::{GangName, GangRoster, GangerName},
    weapon::{FISTS_KEY, WeaponName},
};
use gdtf_content_families::GangsFamily;
use gdtf_editor::{GangDraft, draft_to_roster, gang_file_name, write_gang_in};

use crate::{
    advance::advance_to_published,
    app::editor_app_with_asset_root,
    findings::{dangling_ref_referrer, has_dangling_ref},
    harness::{DANGLING_DEFAULT_FLOOR, editor_app_on_fixture_root},
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
        member.weapon = Some(WeaponName::new(EDITED_DANGLING_WEAPON.to_owned()));
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

const CLEARED_ARMOR_GANG: &str = "cleared_armor_gang";

const CLEARED_ARMOR_MEMBER: &str = "Cleared Armor Member";

const CLEARED_GANG_DANGLING_WEAPON: &str = "cleared_gang_missing_weapon";

/// Whether any `DanglingRef` against `family` names this gang and member as its referrer.
fn referrer_reported(report: &ContentIntegrityReport, family: &str, referrer_hint: &str) -> bool {
    report.findings().iter().any(|finding| {
        matches!(
            finding,
            ContentFinding::DanglingRef { referrer, family: found_family, .. }
                if **found_family == *family && referrer.contains(referrer_hint)
        )
    })
}

#[test]
fn a_member_holding_no_armor_key_raises_no_armor_finding_against_it() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let mut draft = GangDraft::new_gang();
    draft.set_name(CLEARED_ARMOR_GANG.to_owned());
    draft.add_member();
    if let Some(member) = draft.members_mut().first_mut() {
        member.name = GangerName::new(CLEARED_ARMOR_MEMBER.to_owned());
        member.armor = None;
        member.weapon = Some(WeaponName::new(CLEARED_GANG_DANGLING_WEAPON.to_owned()));
    }
    let (gang_name, roster) = draft_to_roster(&draft);
    let written = write_gang_in(dir.path(), &gang_name, &roster);
    assert!(
        written.is_ok(),
        "the real gang write must succeed: {:?}",
        written.as_ref().err(),
    );

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_published(&mut app);
    let report = app.world().resource::<ContentIntegrityReport>();

    assert!(
        referrer_reported(report, "WeaponRegistry", CLEARED_ARMOR_MEMBER),
        "the same member's dangling weapon key must still be reported, so the gang was read; \
         report: {:?}",
        report.findings(),
    );
    assert!(
        !referrer_reported(report, "ArmorRegistry", CLEARED_ARMOR_MEMBER),
        "a member holding no armor key must raise no ArmorRegistry finding naming it, whatever \
         the finding's target; report: {:?}",
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
