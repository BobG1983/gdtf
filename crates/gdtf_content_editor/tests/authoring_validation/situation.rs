//! The situation's outbound reference edges are checked on the editor host too.

use gdtf_assets::{ContentIntegrityReport, ReferenceKeyScheme};

use crate::harness::{advance_to_published, editor_app_on_fixture_root, has_dangling_ref};

// The gang the fixture situation names, which the fixture root holds no file for.
const GHOST_GANG: &str = "ghost_gang";

#[test]
fn a_dangling_situation_gang_ref_surfaces_in_the_editor_at_authoring_time() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    let report = app.world().get_resource::<ContentIntegrityReport>();
    assert!(
        report.is_some(),
        "the ContentIntegrityReport resource must exist in the editor app",
    );
    let Some(report) = report else { return };

    assert!(
        has_dangling_ref(
            report,
            "GangRegistry",
            GHOST_GANG,
            ReferenceKeyScheme::FileStem,
        ),
        "the situation's gang key is checked on the editor host, so `{GHOST_GANG}` must come \
         back as a GangRegistry file-stem miss; report: {:?}",
        report.findings(),
    );
}
