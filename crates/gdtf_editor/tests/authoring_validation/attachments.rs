//! the weapon→attachment edge's authoring-time pins — the editor
use bevy::asset::AssetServer;
use cobalt_test_utils::advance_until;
use gdtf_assets::{ContentFamily, ContentIntegrityReport, ReferenceKeyScheme};
use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect},
    equipment::attachments::{AttachmentName, AttachmentRegistry, AttachmentSlot},
    weapon::WeaponName,
};
use gdtf_content_families::{AttachmentsFamily, WeaponsFamily};
use gdtf_editor::{
    AttachmentDraft, attachment_file_name, draft_to_attachment_spec, write_attachment_in,
};

use crate::{
    advance::advance_to_published,
    app::editor_app_with_asset_root,
    findings::{dangling_ref_referrer, has_dangling_ref},
    harness::has_malformed,
};

const REARM_SCOPE: &str = "rearm_scope";

const DANGLING_SCOPE: &str = "attachment_suite_missing_scope";

const MALFORMED_STEM: &str = "broken_scope";

const WEAPON_STEM: &str = "ghost_rifle";

const WEAPON_RON: &str = r#"(
    base_spread:   0.10,
    accuracy:      1.0,
    kickback:      0.05,
    fatal_bias:    0.0,
    damage:        6,
    punch:         2,
    shred:         1,
    damage_type:   Las,
    magazine: (
        size:      6,
        reload_tu: 10,
    ),
    fire_mode: [
        (kind: Single, cone_mult: 1.0, tu_percent: 0.30, shots: 1),
    ],
    stable: false,
    handedness:    TwoHanded,
    slots: [
        (Sight, 1),
    ],
    attachments:   ["rearm_scope", "attachment_suite_missing_scope"],
)
"#;

fn scope_draft(aim: f32) -> AttachmentDraft {
    let mut draft = AttachmentDraft::new_attachment();
    draft.set_name(REARM_SCOPE.to_owned());
    let spec = draft.spec_mut();
    spec.display_name = WeaponName::new("Rearm Scope".to_owned());
    spec.slot = AttachmentSlot::Sight;
    spec.effects = vec![AttachmentEffect::Aim(AimDelta::new(aim))];
    draft
}

fn plant_fixture_root(root: &std::path::Path) {
    let (name, spec) = draft_to_attachment_spec(&scope_draft(0.25));
    let written = write_attachment_in(root, &name, &spec);
    assert!(
        written.is_ok(),
        "the real attachment write must succeed: {:?}",
        written.as_ref().err(),
    );

    let malformed_path = root
        .join(AttachmentsFamily::FOLDER)
        .join(format!("{MALFORMED_STEM}.{}", AttachmentsFamily::EXTENSION));
    let planted = std::fs::write(&malformed_path, "(this is not an AttachmentSpec");
    assert!(
        planted.is_ok(),
        "planting the malformed sibling must succeed"
    );

    let weapons_dir = root.join(WeaponsFamily::FOLDER);
    let created = std::fs::create_dir_all(&weapons_dir);
    assert!(created.is_ok(), "creating the weapons folder must succeed");
    let weapon_path = weapons_dir.join(format!("{WEAPON_STEM}.{}", WeaponsFamily::EXTENSION));
    let weapon_planted = std::fs::write(&weapon_path, WEAPON_RON);
    assert!(
        weapon_planted.is_ok(),
        "planting the fixture weapon must succeed"
    );
}

#[test]
fn attachment_save_reload_rearms_validation_and_republishes_weapon_findings() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    plant_fixture_root(dir.path());

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_published(&mut app);
    {
        let report = app.world().resource::<ContentIntegrityReport>();
        assert!(
            !has_dangling_ref(
                report,
                "AttachmentRegistry",
                REARM_SCOPE,
                ReferenceKeyScheme::FileStem,
            ),
            "the SAVED attachment must resolve the weapon's key at editor launch (no dangling \
             AttachmentRegistry finding); report: {:?}",
            report.findings(),
        );
        let referrer = dangling_ref_referrer(
            report,
            "AttachmentRegistry",
            DANGLING_SCOPE,
            ReferenceKeyScheme::FileStem,
        );
        assert!(
            referrer.is_some(),
            "the weapon's dangling attachment key must surface at authoring time (the \
             weapon→attachment edge — is check_weapon_attachment_refs registered in the \
             editor?); report: {:?}",
            report.findings(),
        );
        assert!(
            referrer
                .as_ref()
                .is_some_and(|who| who.contains(WEAPON_STEM)),
            "the DanglingRef finding must name the referring weapon `{WEAPON_STEM}`; \
             named: {referrer:?}",
        );
        assert!(
            has_malformed(report, MALFORMED_STEM),
            "the malformed attachment sibling must surface as a MalformedFile finding at \
             launch; report: {:?}",
            report.findings(),
        );
    }

    let edited_draft = scope_draft(0.5);
    let (name, edited_spec) = draft_to_attachment_spec(&edited_draft);
    let rewritten = write_attachment_in(dir.path(), &name, &edited_spec);
    assert!(
        rewritten.is_ok(),
        "the attachment re-save must succeed: {:?}",
        rewritten.as_ref().err(),
    );

    let saved_path = format!(
        "{}/{}",
        AttachmentsFamily::FOLDER,
        attachment_file_name(&name)
    );
    app.world().resource::<AssetServer>().reload(saved_path);

    advance_until(&mut app, |app| {
        let registry_rebuilt = app
            .world()
            .get_resource::<AttachmentRegistry>()
            .is_some_and(|registry| {
                registry.spec(&AttachmentName::new(REARM_SCOPE.to_owned())) == Some(&edited_spec)
            });
        let report_fresh = app
            .world()
            .get_resource::<ContentIntegrityReport>()
            .is_some_and(|report| {
                has_dangling_ref(
                    report,
                    "AttachmentRegistry",
                    DANGLING_SCOPE,
                    ReferenceKeyScheme::FileStem,
                ) && !has_malformed(report, MALFORMED_STEM)
            });
        registry_rebuilt && report_fresh
    });
    let report = app.world().resource::<ContentIntegrityReport>();
    assert!(
        !has_dangling_ref(
            report,
            "AttachmentRegistry",
            REARM_SCOPE,
            ReferenceKeyScheme::FileStem,
        ),
        "the re-saved attachment key must still resolve after the re-check; report: {:?}",
        report.findings(),
    );
}
