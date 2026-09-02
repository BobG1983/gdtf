//! the INJURY mode's weighting SAVE path feeds the authoring-validation
use bevy::asset::AssetServer;
use gdtf_assets::{ContentIntegrityReport, ReferenceKeyScheme};
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryName, InjuryTables, InjuryWeight, WeightedInjuryEntry},
};
use gdtf_content_editor::{
    WeightingDraft, draft_to_weighting, weighting_file_name, write_weighting_in,
};
use gdtf_content_families::injuries::{INJURIES_FOLDER, WEIGHTING_SUBFOLDER};
use gdtf_test_utils::advance_until;

use crate::harness::{advance_to_published, editor_app_with_asset_root, has_dangling_ref};

const SAVED_DANGLING_INJURY: &str = "saved_missing_injury";

const RESAVED_DANGLING_INJURY: &str = "resaved_missing_injury";

fn weighting_draft_with_row(key: &str) -> WeightingDraft {
    let mut draft = WeightingDraft::default();
    draft.load_table(
        InjuryCategory::Head,
        DamageContext::Ranged,
        &InjuryTables::default(),
    );
    draft.weighting_mut().minor.push(WeightedInjuryEntry::new(
        InjuryName::new(key.to_owned()),
        InjuryWeight::new(2),
    ));
    draft
}

#[test]
fn weighting_save_reload_rearms_validation_with_the_saved_keys() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let weighting = draft_to_weighting(&weighting_draft_with_row(SAVED_DANGLING_INJURY));
    let written = write_weighting_in(dir.path(), &weighting);
    assert!(
        written.is_ok(),
        "the real weighting write must succeed: {:?}",
        written.as_ref().err(),
    );

    let mut app = editor_app_with_asset_root(dir.path());
    advance_to_published(&mut app);
    {
        let report = app.world().resource::<ContentIntegrityReport>();
        assert!(
            has_dangling_ref(
                report,
                "InjuryRegistry",
                SAVED_DANGLING_INJURY,
                ReferenceKeyScheme::FileStem,
            ),
            "the SAVED weighting's dangling injury key must surface at editor launch; \
             report: {:?}",
            report.findings(),
        );
    }

    let weighting = draft_to_weighting(&weighting_draft_with_row(RESAVED_DANGLING_INJURY));
    let rewritten = write_weighting_in(dir.path(), &weighting);
    assert!(
        rewritten.is_ok(),
        "the re-save must succeed: {:?}",
        rewritten.as_ref().err(),
    );

    let saved_path = format!(
        "{INJURIES_FOLDER}/{WEIGHTING_SUBFOLDER}/{}",
        weighting_file_name(InjuryCategory::Head, DamageContext::Ranged),
    );
    app.world().resource::<AssetServer>().reload(saved_path);

    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<ContentIntegrityReport>()
            .is_some_and(|report| {
                has_dangling_ref(
                    report,
                    "InjuryRegistry",
                    RESAVED_DANGLING_INJURY,
                    ReferenceKeyScheme::FileStem,
                )
            })
    });
    let report = app.world().resource::<ContentIntegrityReport>();
    assert!(
        !has_dangling_ref(
            report,
            "InjuryRegistry",
            SAVED_DANGLING_INJURY,
            ReferenceKeyScheme::FileStem,
        ),
        "the report must be RESET and re-checked on re-arm — the first save's superseded \
         injury finding must not persist; report: {:?}",
        report.findings(),
    );
}
