//! GTW-651 C3(c): the GANG mode's own SAVE path feeds the authoring-validation
//! loop — a save writes the file where the loader reads, and the resulting
//! registry rebuild re-arms the pass.
//!
//! THE RELOAD TRIGGER, noted per the contract: the file-watcher is NOT active in this
//! headless harness (`file_watcher` is a binary-propagated feature, never in
//! test builds — the crate's documented convention covers the deterministic
//! injected-reload path instead of the OS watcher thread). So the ONE
//! watcher-owned step — "a changed file on disk triggers its reload" — is
//! driven directly via [`AssetServer::reload`]; everything else is the REAL
//! production loop: the real root-parameterized save write ([`write_gang_in`]),
//! the real folder-family loader reading the saved bytes back off disk, the
//! real redrive registry rebuild, and the real re-arm → re-check → re-publish.

use bevy::asset::AssetServer;
use gdtf_assets::{ContentFamily, ContentIntegrityReport};
use gdtf_battle_sim::weapon::WeaponName;
use gdtf_content_editor::{GangDraft, draft_to_roster, gang_file_name, write_gang_in};
use gdtf_content_families::GangsFamily;
use gdtf_test_utils::advance_until;

use crate::harness::{
    MAX_UPDATES, advance_to_published, editor_app_with_asset_root, has_dangling_ref,
};

/// The saved gang's name (sanitizes to itself, so it is also the file stem).
const REARM_GANG: &str = "rearm_gang";

/// The first save's DANGLING weapon key (the `TempDir` root materializes no
/// weapons folder, so every weapon key dangles).
const SAVED_DANGLING_WEAPON: &str = "saved_missing_weapon";

/// The re-save's DISTINCT dangling weapon key — the finding the re-armed pass
/// must re-publish in place of the first one.
const RESAVED_DANGLING_WEAPON: &str = "resaved_missing_weapon";

/// C3(c): author a gang through the REAL form model, SAVE it through the real
/// root-parameterized write into a `TempDir` assets root, and boot the editor on
/// that root — the saved dangling key surfaces at launch. Then EDIT + RE-SAVE
/// through the same write and reload the saved path (the watcher stand-in —
/// see the module doc): the redrive rebuilds the [`GangRegistry`] from the
/// re-read file and the re-armed pass re-publishes the RE-SAVED key's finding,
/// dropping the superseded one.
///
/// [`GangRegistry`]: gdtf_battle_sim::ganger::GangRegistry
#[test]
fn gang_save_reload_rearms_validation_with_the_saved_keys() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    // Author + SAVE (the real Save-button write) a gang whose weapon key dangles.
    let mut draft = GangDraft::new_gang();
    draft.set_name(REARM_GANG.to_owned());
    draft.add_member();
    if let Some(member) = draft.members_mut().first_mut() {
        member.weapon = WeaponName::new(SAVED_DANGLING_WEAPON.to_owned());
    }
    let (name, roster) = draft_to_roster(&draft);
    let written = write_gang_in(dir.path(), &name, &roster);
    assert!(
        written.is_ok(),
        "the real gang write must succeed: {:?}",
        written.as_ref().err(),
    );

    // BOOT the editor on the TempDir root: the saved gang loads through the
    // real folder walk and its dangling weapon key surfaces at launch.
    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_published(&mut app);
    {
        let report = app.world().resource::<ContentIntegrityReport>();
        assert!(
            has_dangling_ref(report, "WeaponRegistry", SAVED_DANGLING_WEAPON),
            "the SAVED gang's dangling weapon key must surface at editor launch; report: {:?}",
            report.findings(),
        );
    }

    // EDIT + RE-SAVE through the SAME real write (same name → same stem →
    // overwrites the file the loader loaded).
    if let Some(member) = draft.members_mut().first_mut() {
        member.weapon = WeaponName::new(RESAVED_DANGLING_WEAPON.to_owned());
    }
    let (name, roster) = draft_to_roster(&draft);
    let rewritten = write_gang_in(dir.path(), &name, &roster);
    assert!(
        rewritten.is_ok(),
        "the re-save must succeed: {:?}",
        rewritten.as_ref().err(),
    );

    // The watcher stand-in (module doc): reload the saved path from disk.
    let saved_path = format!("{}/{}", GangsFamily::FOLDER, gang_file_name(&name));
    app.world().resource::<AssetServer>().reload(saved_path);

    // The reload re-reads the saved bytes → the redrive rebuilds the registry →
    // the re-armed pass re-publishes the CURRENT (re-saved) finding.
    let republished = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<ContentIntegrityReport>()
                .is_some_and(|report| {
                    has_dangling_ref(report, "WeaponRegistry", RESAVED_DANGLING_WEAPON)
                })
        },
        MAX_UPDATES,
    );
    assert!(
        republished,
        "a gang SAVE + reload must re-arm the validation pass — the re-saved dangling weapon \
         key was never re-reported",
    );
    let report = app.world().resource::<ContentIntegrityReport>();
    assert!(
        !has_dangling_ref(report, "WeaponRegistry", SAVED_DANGLING_WEAPON),
        "the report must be RESET and re-checked on re-arm — the first save's superseded weapon \
         finding must not persist; report: {:?}",
        report.findings(),
    );
}
