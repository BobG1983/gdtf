use super::super::{ContentFinding, FindingReferrer, FindingTarget};

#[test]
fn a_missing_views_finding_names_the_def_and_every_view_it_lacks() {
    let finding = ContentFinding::MissingViews {
        referrer: FindingReferrer::new(
            "terrain def `Rusted Barrels` (00000000-0000-0000-0000-000000000001)".to_owned(),
        ),
        views:    vec![
            FindingTarget::new("Facing(East)".to_owned()),
            FindingTarget::new("Facing(West)".to_owned()),
        ],
    };

    let text = finding.to_string();
    assert!(
        text.contains("Rusted Barrels"),
        "the finding names the def that is short of art: {text}",
    );
    assert!(
        text.contains("Facing(East)"),
        "the finding names the first view the def is missing: {text}",
    );
    assert!(
        text.contains("Facing(West)"),
        "one finding per def names EVERY view it is missing, not just the first: {text}",
    );
}
