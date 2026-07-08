//! GTW-479 C3: the ARMOR mode's own SAVE path feeds the authoring-validation
//! loop — a saved armor file satisfies the gang-equipment armor edge through the
//! REAL loader, and an armor RE-SAVE re-arms the pass and re-publishes findings.
//!
//! THE SEAM, noted per the GTW-651 contract convention: the file-watcher is NOT
//! active in this headless harness (`file_watcher` is binary-propagated, never
//! in test builds), so the ONE watcher-owned step — "a changed file on disk
//! triggers its reload" — is driven directly via [`AssetServer::reload`];
//! everything else is the REAL production loop: the real root-parameterized
//! armor save write ([`write_armor_in`]), the real folder-family loader (here
//! its per-file SALVAGE path — the root plants one malformed sibling) reading
//! the saved bytes back off disk, the real redrive registry rebuild, and the
//! real re-arm → re-check → re-publish.

use bevy::asset::AssetServer;
use gdtf_assets::{ContentFamily, ContentIntegrityReport};
use gdtf_battle_sim::{
    armor::{ArmorName, ArmorProtection, ArmorRegistry, BodyPart},
    weapon::WeaponName,
};
use gdtf_content_editor::{
    ArmorDraft, GangDraft, armor_file_name, draft_to_roster, draft_to_spec, write_armor_in,
    write_gang_in,
};
use gdtf_content_families::ArmorFamily;
use gdtf_test_utils::advance_until;

use crate::harness::{
    MAX_UPDATES, advance_to_published, editor_app_with_asset_root, has_dangling_ref, has_malformed,
};

/// The saved armor's name (sanitizes to itself, so it is also the file stem) —
/// the key the fixture gang's member references.
const REARM_ARMOR: &str = "rearm_plate";

/// The fixture gang member's DANGLING weapon key (the `TempDir` root
/// materializes no weapons folder) — the finding the re-armed pass must
/// RE-publish onto the fresh report.
const DANGLING_WEAPON: &str = "armor_suite_missing_weapon";

/// The deliberately malformed armor sibling's file stem. Its load-time
/// `MalformedFile` finding is the RESET observable: the re-arm replaces the
/// report, and only the REFERENCE checks re-run — so the finding's
/// disappearance proves the report was reset, not stale (the documented
/// reset-don't-accumulate behavior of the re-arm).
const MALFORMED_STEM: &str = "broken_plate";

/// C3: author + SAVE an armor suit through the REAL form model + write into a
/// `TempDir` assets root referenced by a saved gang — at editor launch the REAL
/// (salvage-path) armor loader resolves the gang's armor key, so NO dangling
/// `ArmorRegistry` finding is published (the armor edge, satisfied by the saved
/// file). Then EDIT + RE-SAVE the armor through the same write and reload the
/// saved path (the watcher stand-in — module doc): the redrive rebuilds the
/// [`ArmorRegistry`] from the re-read file (the edited spec lands) and the
/// re-armed pass re-publishes onto a FRESH report — the gang's still-dangling
/// weapon finding is re-reported while the load-time `MalformedFile` finding is
/// dropped by the reset.
#[test]
fn armor_save_reload_rearms_validation_and_republishes_findings() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    // Author + SAVE (the real Save-button write) the armor suit the gang wears.
    let mut armor_draft = ArmorDraft::new_armor();
    armor_draft.set_name(REARM_ARMOR.to_owned());
    armor_draft.piece_mut(BodyPart::Torso).protection = ArmorProtection::new(4);
    let (armor_name, armor_spec) = draft_to_spec(&armor_draft);
    let written = write_armor_in(dir.path(), &armor_name, &armor_spec);
    assert!(
        written.is_ok(),
        "the real armor write must succeed: {:?}",
        written.as_ref().err(),
    );

    // Plant one malformed armor SIBLING (fixture setup, raw bytes): it forces
    // the family's real per-file salvage and its `MalformedFile` finding is the
    // re-arm's reset observable.
    let malformed_path = dir
        .path()
        .join(ArmorFamily::FOLDER)
        .join(format!("{MALFORMED_STEM}.{}", ArmorFamily::EXTENSION));
    let planted = std::fs::write(&malformed_path, "(this is not an ArmorSpec");
    assert!(
        planted.is_ok(),
        "planting the malformed sibling must succeed"
    );

    // A gang wearing the saved armor, wielding a weapon key that dangles.
    let mut gang_draft = GangDraft::new_gang();
    gang_draft.set_name("armor_rearm_gang".to_owned());
    gang_draft.add_member();
    if let Some(member) = gang_draft.members_mut().first_mut() {
        member.armor = ArmorName::new(REARM_ARMOR.to_owned());
        member.weapon = WeaponName::new(DANGLING_WEAPON.to_owned());
    }
    let (gang_name, roster) = draft_to_roster(&gang_draft);
    let gang_written = write_gang_in(dir.path(), &gang_name, &roster);
    assert!(
        gang_written.is_ok(),
        "the real gang write must succeed: {:?}",
        gang_written.as_ref().err(),
    );

    // BOOT the editor on the TempDir root: the saved armor loads through the
    // real (salvaged) folder walk and SATISFIES the gang's armor edge.
    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_published(&mut app);
    {
        let report = app.world().resource::<ContentIntegrityReport>();
        assert!(
            !has_dangling_ref(report, "ArmorRegistry", REARM_ARMOR),
            "the SAVED armor must resolve the gang's armor key at editor launch (no dangling \
             ArmorRegistry finding); report: {:?}",
            report.findings(),
        );
        assert!(
            has_dangling_ref(report, "WeaponRegistry", DANGLING_WEAPON),
            "the gang's dangling weapon key must surface at editor launch; report: {:?}",
            report.findings(),
        );
        assert!(
            has_malformed(report, MALFORMED_STEM),
            "the malformed armor sibling must surface as a MalformedFile finding at launch; \
             report: {:?}",
            report.findings(),
        );
    }

    // EDIT + RE-SAVE through the SAME real write (same name → same stem →
    // overwrites the file the loader loaded).
    armor_draft.piece_mut(BodyPart::Torso).protection = ArmorProtection::new(9);
    let (armor_name, edited_spec) = draft_to_spec(&armor_draft);
    let rewritten = write_armor_in(dir.path(), &armor_name, &edited_spec);
    assert!(
        rewritten.is_ok(),
        "the armor re-save must succeed: {:?}",
        rewritten.as_ref().err(),
    );

    // The watcher stand-in (module doc): reload the saved path from disk.
    let saved_path = format!("{}/{}", ArmorFamily::FOLDER, armor_file_name(&armor_name));
    app.world().resource::<AssetServer>().reload(saved_path);

    // The reload re-reads the saved bytes → the redrive rebuilds the registry
    // (the EDITED spec lands) → the re-armed pass re-publishes onto a FRESH
    // report: the weapon finding is re-reported, the MalformedFile finding is
    // dropped by the reset.
    let republished = advance_until(
        &mut app,
        |app| {
            let registry_rebuilt =
                app.world()
                    .get_resource::<ArmorRegistry>()
                    .is_some_and(|registry| {
                        registry.spec(&ArmorName::new(REARM_ARMOR.to_owned())) == Some(&edited_spec)
                    });
            let report_fresh = app
                .world()
                .get_resource::<ContentIntegrityReport>()
                .is_some_and(|report| {
                    has_dangling_ref(report, "WeaponRegistry", DANGLING_WEAPON)
                        && !has_malformed(report, MALFORMED_STEM)
                });
            registry_rebuilt && report_fresh
        },
        MAX_UPDATES,
    );
    assert!(
        republished,
        "an armor SAVE + reload must rebuild the ArmorRegistry with the re-saved spec, re-arm \
         the validation pass, and re-publish the current findings onto a fresh report",
    );
    let report = app.world().resource::<ContentIntegrityReport>();
    assert!(
        !has_dangling_ref(report, "ArmorRegistry", REARM_ARMOR),
        "the re-saved armor key must still resolve after the re-check; report: {:?}",
        report.findings(),
    );
}
