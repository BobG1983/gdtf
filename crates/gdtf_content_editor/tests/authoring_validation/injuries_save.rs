//! GTW-654 C4: the INJURY mode's weighting SAVE path feeds the authoring-validation
//! loop — a save writes the file where the bespoke injuries loader reads, the
//! resulting redrive rebuilds the [`InjuryRegistry`] (the watched registry), and
//! the re-armed pass re-publishes the CURRENT weighting-row findings through the
//! editor-registered `check_injury_weighting_refs` (the GTW-630/651 edge set,
//! extended by GTW-654).
//!
//! THE SEAM, noted per the convention: the file-watcher is NOT active in this
//! headless harness (`file_watcher` is a binary-propagated feature, never in test
//! builds), so the ONE watcher-owned step — "a changed file on disk triggers its
//! reload" — is driven directly via [`AssetServer::reload`]; everything else is
//! the REAL production loop: the real root-parameterized save write
//! ([`write_weighting_in`]), the real bespoke folder loader reading the saved
//! bytes back off disk, the real redrive rebuild of BOTH injuries resources, and
//! the real re-arm → re-check → re-publish.
//!
//! [`InjuryRegistry`]: gdtf_battle_sim::injuries::InjuryRegistry

use bevy::asset::AssetServer;
use gdtf_assets::ContentIntegrityReport;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{InjuryName, InjuryTables, InjuryWeight, WeightedInjuryEntry},
};
use gdtf_content_editor::{
    WeightingDraft, draft_to_weighting, weighting_file_name, write_weighting_in,
};
use gdtf_content_families::injuries::{INJURIES_FOLDER, WEIGHTING_SUBFOLDER};
use gdtf_test_utils::advance_until;

use crate::harness::{
    MAX_UPDATES, advance_to_published, editor_app_with_asset_root, has_dangling_ref,
};

/// The first save's DANGLING injury key (the `TempDir` root materializes no
/// injury defs, so every weighting row's key dangles).
const SAVED_DANGLING_INJURY: &str = "saved_missing_injury";

/// The re-save's DISTINCT dangling injury key — the finding the re-armed pass
/// must re-publish in place of the first one.
const RESAVED_DANGLING_INJURY: &str = "resaved_missing_injury";

/// Push one Minor row naming `key` onto a fresh Head-context draft through the
/// REAL model mutators (the model seam the weighting panel edits through).
fn weighting_draft_with_row(key: &str) -> WeightingDraft {
    let mut draft = WeightingDraft::default();
    draft.load_category(InjuryCategory::Head, &InjuryTables::default());
    draft.weighting_mut().minor.push(WeightedInjuryEntry::new(
        InjuryName::new(key.to_owned()),
        InjuryWeight::new(2),
    ));
    draft
}

/// C4: author a weighting table through the REAL form model, SAVE it through the
/// real root-parameterized write into a `TempDir` assets root, and boot the editor
/// on that root — the saved dangling injury key surfaces at launch (the editor now
/// registers the injuries edge). Then EDIT + RE-SAVE through the same write (same
/// category → same file) and reload the saved path (the watcher stand-in — see the
/// module doc): the redrive rebuilds the injuries pair from the re-read file and
/// the re-armed pass re-publishes the RE-SAVED key's finding, dropping the
/// superseded one — the watch set covers the injury registry end to end.
#[test]
fn weighting_save_reload_rearms_validation_with_the_saved_keys() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    // Author + SAVE (the real Save-button write) a weighting whose injury key
    // dangles (no defs are materialized in this root).
    let weighting = draft_to_weighting(&weighting_draft_with_row(SAVED_DANGLING_INJURY));
    let written = write_weighting_in(dir.path(), &weighting);
    assert!(
        written.is_ok(),
        "the real weighting write must succeed: {:?}",
        written.as_ref().err(),
    );

    // BOOT the editor on the TempDir root: the saved weighting loads through the
    // real bespoke folder walk and its dangling injury key surfaces at launch.
    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_published(&mut app);
    {
        let report = app.world().resource::<ContentIntegrityReport>();
        assert!(
            has_dangling_ref(report, "InjuryRegistry", SAVED_DANGLING_INJURY),
            "the SAVED weighting's dangling injury key must surface at editor launch; \
             report: {:?}",
            report.findings(),
        );
    }

    // EDIT + RE-SAVE through the SAME real write (same category → same file →
    // overwrites the table the loader loaded).
    let weighting = draft_to_weighting(&weighting_draft_with_row(RESAVED_DANGLING_INJURY));
    let rewritten = write_weighting_in(dir.path(), &weighting);
    assert!(
        rewritten.is_ok(),
        "the re-save must succeed: {:?}",
        rewritten.as_ref().err(),
    );

    // The watcher stand-in (module doc): reload the saved path from disk. Every
    // segment derives from the one-owner spellings (GTW-634).
    let saved_path = format!(
        "{INJURIES_FOLDER}/{WEIGHTING_SUBFOLDER}/{}",
        weighting_file_name(InjuryCategory::Head),
    );
    app.world().resource::<AssetServer>().reload(saved_path);

    // The reload re-reads the saved bytes → the redrive rebuilds the injuries pair
    // (marking the watched InjuryRegistry changed) → the re-armed pass re-publishes
    // the CURRENT (re-saved) finding.
    let republished = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<ContentIntegrityReport>()
                .is_some_and(|report| {
                    has_dangling_ref(report, "InjuryRegistry", RESAVED_DANGLING_INJURY)
                })
        },
        MAX_UPDATES,
    );
    assert!(
        republished,
        "a weighting SAVE + reload must re-arm the validation pass — the re-saved dangling \
         injury key was never re-reported (is the InjuryRegistry in the watch set?)",
    );
    let report = app.world().resource::<ContentIntegrityReport>();
    assert!(
        !has_dangling_ref(report, "InjuryRegistry", SAVED_DANGLING_INJURY),
        "the report must be RESET and re-checked on re-arm — the first save's superseded \
         injury finding must not persist; report: {:?}",
        report.findings(),
    );
}
