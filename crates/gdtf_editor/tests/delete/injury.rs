//! The injury delete takes every weighting row naming it out of all three buckets.

use std::path::Path;

use gdtf_assets::{ContentFinding, ContentIntegrityReport, ContentMemberKey, FindingFamily};
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{
        DamageContext, InjuryName, InjuryRegistry, InjuryWeight, InjuryWeighting,
        WeightedInjuryEntry,
    },
    severity::Severity,
};
use gdtf_editor::{
    DeleteOutcome, DeleteRequest, EditorMcpAssetsRoot, InjuryDraft, draft_to_def,
    injury_save_path_in, weighting_save_path_in, write_injury_in, write_weighting_in,
};

use crate::{
    advance::advance_to_published,
    app::editor_app_with_asset_root,
    harness::{OUTCOME_UPDATES, advance_to_outcome, is_published},
};

/// The injury the weighting table names in two of its three buckets.
const DELETED_INJURY: &str = "fixture_gash";

/// The injury every bucket keeps, so the rewrite's ordering is readable.
const KEPT_INJURY: &str = "fixture_bruise";

/// The table the fixture weighting authors.
const CATEGORY: InjuryCategory = InjuryCategory::Head;

/// The damage context half of that table's key.
const CONTEXT: DamageContext = DamageContext::Ranged;

/// The finding family label every injury reference finding carries.
const INJURY_FAMILY: &str = "InjuryRegistry";

// An injury name from a file stem.
fn injury_name(stem: &str) -> InjuryName {
    InjuryName::new(stem.to_owned())
}

// A weighting row for one injury.
fn row(stem: &str, weight: u32) -> WeightedInjuryEntry {
    WeightedInjuryEntry::new(injury_name(stem), InjuryWeight::new(weight))
}

// Write an injury def under `root`, in the table's own category.
fn write_injury(root: &Path, stem: &str) -> bool {
    let mut draft = InjuryDraft::new_injury();
    draft.set_key(stem.to_owned());
    let (name, mut def) = draft_to_def(&draft);
    def.category = CATEGORY;
    def.severity = Severity::Minor;
    write_injury_in(root, &name, &def).is_ok()
}

// Write a table naming the deleted injury in the Minor and Critical buckets.
fn write_weighting(root: &Path) -> bool {
    write_weighting_in(root, &authored_weighting()).is_ok()
}

// The table as it is authored, before any delete rewrites it.
fn authored_weighting() -> InjuryWeighting {
    InjuryWeighting {
        category: CATEGORY,
        context:  CONTEXT,
        minor:    vec![row(KEPT_INJURY, 2), row(DELETED_INJURY, 3)],
        major:    vec![row(KEPT_INJURY, 4)],
        critical: vec![row(DELETED_INJURY, 5), row(KEPT_INJURY, 6)],
    }
}

// The table as the file under `root` holds it.
fn weighting_in_file(root: &Path) -> Option<InjuryWeighting> {
    let ron = std::fs::read_to_string(weighting_save_path_in(root, CATEGORY, CONTEXT)).ok()?;
    ron::from_str(&ron).ok()
}

#[test]
fn deleting_an_injury_takes_its_row_out_of_every_bucket_that_held_one() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };
    assert!(
        write_injury(dir.path(), DELETED_INJURY) && write_injury(dir.path(), KEPT_INJURY),
        "both fixture injury writes must succeed",
    );
    assert!(
        write_weighting(dir.path()),
        "the fixture weighting write must succeed",
    );

    let mut app = editor_app_with_asset_root(dir.path());
    app.insert_resource(EditorMcpAssetsRoot::new(dir.path().to_path_buf()));
    advance_to_published(&mut app);

    app.insert_resource(DeleteRequest::new(
        FindingFamily::new(INJURY_FAMILY.to_owned()),
        ContentMemberKey::new(DELETED_INJURY.to_owned()),
    ));
    let outcome = advance_to_outcome(&mut app);
    assert!(
        outcome.is_some(),
        "the delete must settle within {OUTCOME_UPDATES} updates; published at the end: {}",
        is_published(&app),
    );
    assert_eq!(
        outcome,
        Some(DeleteOutcome::Removed),
        "every bucket holding a row for the injury is rewritten, so the re-run finds nothing",
    );

    let written = weighting_in_file(dir.path());
    assert_eq!(
        written,
        Some(InjuryWeighting {
            category: CATEGORY,
            context:  CONTEXT,
            minor:    vec![row(KEPT_INJURY, 2)],
            major:    vec![row(KEPT_INJURY, 4)],
            critical: vec![row(KEPT_INJURY, 6)],
        }),
        "the written table holds neither row for the deleted injury and keeps its other rows \
         in their authored order; stopping at the first bucket that held one leaves Critical",
    );
    assert!(
        !injury_save_path_in(dir.path(), CATEGORY, &injury_name(DELETED_INJURY)).exists(),
        "the deleted injury's file must be gone",
    );
    assert!(
        app.world()
            .resource::<InjuryRegistry>()
            .def(&injury_name(DELETED_INJURY))
            .is_none(),
        "the removed record must stay out of InjuryRegistry",
    );
    let findings = app
        .world()
        .resource::<ContentIntegrityReport>()
        .findings()
        .iter()
        .filter(|finding| {
            matches!(finding, ContentFinding::DanglingRef { family, .. }
                if **family == *INJURY_FAMILY)
        })
        .count();
    assert_eq!(
        findings, 0,
        "the rewrite goes into the weighting asset store as well as the file, so the re-run \
         reads it with no reload and reports no injury finding",
    );
}
