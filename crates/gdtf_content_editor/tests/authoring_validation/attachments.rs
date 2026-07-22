//! GTW-669 C4: the weapon→attachment edge's authoring-time pins — the editor
//! REGISTERS `check_weapon_attachment_refs` (a weapon `attachments:` key that
//! resolves no attachment item surfaces as a `DanglingRef` finding against the
//! `AttachmentRegistry` in the EDITOR's report), and the validation WATCH SET
//! contains the [`AttachmentRegistry`] (an attachment save's registry rebuild
//! re-arms the pass and re-publishes the CURRENT weapon-edge findings — the
//! GTW-651 recipe over the GTW-619 family).
//!
//! THE RELOAD TRIGGER, noted per the GTW-651 contract convention: the file-watcher is NOT
//! active in this headless harness (`file_watcher` is binary-propagated, never
//! in test builds), so the ONE watcher-owned step — "a changed file on disk
//! triggers its reload" — is driven directly via [`AssetServer::reload`];
//! everything else is the REAL production loop: the real root-parameterized
//! attachment save write ([`write_attachment_in`]), the real folder-family
//! loader (here its per-file SALVAGE path — the root plants one malformed
//! sibling as the re-arm's reset observable) reading the saved bytes back off
//! disk, the real redrive registry rebuild, and the real re-arm → re-check →
//! re-publish. The referring WEAPON is planted as raw fixture bytes (there is
//! no weapon editor mode to save through until GTW-670).
//!
//! [`AttachmentRegistry`]: gdtf_battle_sim::equipment::attachments::AttachmentRegistry

use bevy::asset::AssetServer;
use gdtf_assets::{ContentFamily, ContentIntegrityReport};
use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect},
    equipment::attachments::{AttachmentName, AttachmentRegistry, AttachmentSlot},
    weapon::WeaponName,
};
use gdtf_content_editor::{
    AttachmentDraft, attachment_file_name, draft_to_attachment_spec, write_attachment_in,
};
use gdtf_content_families::{AttachmentsFamily, WeaponsFamily};
use gdtf_test_utils::advance_until;

use crate::harness::{
    MAX_UPDATES, advance_to_published, dangling_ref_referrer, editor_app_with_asset_root,
    has_dangling_ref, has_malformed,
};

/// The saved attachment's name (sanitizes to itself, so it is also the file
/// stem) — the key the fixture weapon's `attachments:` list RESOLVES.
const REARM_SCOPE: &str = "rearm_scope";

/// The fixture weapon's DANGLING attachment key (no such item is ever
/// materialized) — the finding the editor-registered weapon→attachment check
/// must publish at launch and the re-armed pass must RE-publish onto the fresh
/// report.
const DANGLING_SCOPE: &str = "attachment_suite_missing_scope";

/// The deliberately malformed attachment sibling's file stem. Its load-time
/// `MalformedFile` finding is the RESET observable (see
/// [`has_malformed`](crate::harness::has_malformed)).
const MALFORMED_STEM: &str = "broken_scope";

/// The fixture weapon's file stem — the referrer the `DanglingRef` finding
/// must name.
const WEAPON_STEM: &str = "ghost_rifle";

/// The referring weapon, planted as raw `.weapon.ron` bytes (module doc: no
/// weapon editor mode exists to save through until GTW-670): one Sight slot,
/// one resolving attachment key ([`REARM_SCOPE`]) and one dangling one
/// ([`DANGLING_SCOPE`]). The magnitudes are throwaway fixture data.
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

/// An attachment draft authored through the REAL form-model mutators: a Sight
/// whose one effect carries the given aim magnitude (the re-save flips the
/// magnitude so the rebuilt registry is distinguishable from the boot one).
fn scope_draft(aim: f32) -> AttachmentDraft {
    let mut draft = AttachmentDraft::new_attachment();
    draft.set_name(REARM_SCOPE.to_owned());
    let spec = draft.spec_mut();
    spec.display_name = WeaponName::new("Rearm Scope".to_owned());
    spec.slot = AttachmentSlot::Sight;
    spec.effects = vec![AttachmentEffect::Aim(AimDelta::new(aim))];
    draft
}

/// Materialize the `TempDir` fixture root: SAVE the resolving attachment
/// through the REAL root-parameterized write ([`scope_draft`] at aim `0.25`),
/// plant the malformed attachment SIBLING (raw bytes — it forces the family's
/// real per-file salvage, and its `MalformedFile` finding is the re-arm's
/// reset observable), and plant the referring weapon (raw bytes — module doc).
fn plant_fixture_root(root: &std::path::Path) {
    // Author + SAVE (the real Save-button write) the attachment the weapon fits.
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

/// C4: at editor launch the REAL (salvage-path) attachments loader resolves the
/// fixture weapon's saved key — so only the DANGLING key surfaces, as a
/// `DanglingRef` against the `AttachmentRegistry` naming the weapon (the
/// editor registers `check_weapon_attachment_refs`). Then EDIT + RE-SAVE the
/// attachment through the same write and reload the saved path (the watcher
/// stand-in — module doc): the redrive rebuilds the [`AttachmentRegistry`]
/// from the re-read file (the edited spec lands, marking the WATCHED registry
/// changed — the only registry that changes in this window) and the re-armed
/// pass re-publishes onto a FRESH report — the weapon's still-dangling key is
/// re-reported while the load-time `MalformedFile` finding is dropped by the
/// reset.
///
/// [`AttachmentRegistry`]: gdtf_battle_sim::equipment::attachments::AttachmentRegistry
#[test]
fn attachment_save_reload_rearms_validation_and_republishes_weapon_findings() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    plant_fixture_root(dir.path());

    // BOOT the editor on the TempDir root: the weapon loads through the real
    // folder walk, the attachment through the real salvage, and the editor's
    // REGISTERED weapon→attachment check publishes exactly the dangling key.
    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_published(&mut app);
    {
        let report = app.world().resource::<ContentIntegrityReport>();
        assert!(
            !has_dangling_ref(report, "AttachmentRegistry", REARM_SCOPE),
            "the SAVED attachment must resolve the weapon's key at editor launch (no dangling \
             AttachmentRegistry finding); report: {:?}",
            report.findings(),
        );
        let referrer = dangling_ref_referrer(report, "AttachmentRegistry", DANGLING_SCOPE);
        assert!(
            referrer.is_some(),
            "the weapon's dangling attachment key must surface at authoring time (the GTW-669 \
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

    // EDIT + RE-SAVE through the SAME real write (same name → same stem →
    // overwrites the file the loader loaded).
    let edited_draft = scope_draft(0.5);
    let (name, edited_spec) = draft_to_attachment_spec(&edited_draft);
    let rewritten = write_attachment_in(dir.path(), &name, &edited_spec);
    assert!(
        rewritten.is_ok(),
        "the attachment re-save must succeed: {:?}",
        rewritten.as_ref().err(),
    );

    // The watcher stand-in (module doc): reload the saved path from disk.
    let saved_path = format!(
        "{}/{}",
        AttachmentsFamily::FOLDER,
        attachment_file_name(&name)
    );
    app.world().resource::<AssetServer>().reload(saved_path);

    // The reload re-reads the saved bytes → the redrive rebuilds the
    // AttachmentRegistry (the EDITED spec lands) → the WATCHED registry's
    // change re-arms the pass → it re-publishes onto a FRESH report: the
    // weapon's dangling key is re-reported, the MalformedFile finding is
    // dropped by the reset.
    let republished = advance_until(
        &mut app,
        |app| {
            let registry_rebuilt = app
                .world()
                .get_resource::<AttachmentRegistry>()
                .is_some_and(|registry| {
                    registry.spec(&AttachmentName::new(REARM_SCOPE.to_owned()))
                        == Some(&edited_spec)
                });
            let report_fresh = app
                .world()
                .get_resource::<ContentIntegrityReport>()
                .is_some_and(|report| {
                    has_dangling_ref(report, "AttachmentRegistry", DANGLING_SCOPE)
                        && !has_malformed(report, MALFORMED_STEM)
                });
            registry_rebuilt && report_fresh
        },
        MAX_UPDATES,
    );
    assert!(
        republished,
        "an attachment SAVE + reload must rebuild the AttachmentRegistry with the re-saved \
         spec, re-arm the validation pass (is the AttachmentRegistry in the watch set?), and \
         re-publish the current weapon-edge findings onto a fresh report",
    );
    let report = app.world().resource::<ContentIntegrityReport>();
    assert!(
        !has_dangling_ref(report, "AttachmentRegistry", REARM_SCOPE),
        "the re-saved attachment key must still resolve after the re-check; report: {:?}",
        report.findings(),
    );
}
