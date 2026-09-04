//! The terrain def's own view coverage, its `leaves_behind` and its per-view sprite keys,
//! all pinned at authoring time on the editor host.

use gdtf_assets::{ContentIntegrityReport, ReferenceKeyScheme};

use crate::{
    advance::advance_to_published,
    findings::{dangling_ref_referrer, has_dangling_ref},
    harness::{editor_app_on_fixture_root, missing_views},
};

const DANGLING_SUCCESSOR: &str = "00000000-0000-0000-0000-130700000bea";

const DANGLING_LEFTOVER: &str = "ghost_leftover_graphic";

const DANGLING_VIEW_GRAPHIC: &str = "ghost_view_graphic";

fn published_report(app: &bevy::app::App) -> Option<&ContentIntegrityReport> {
    app.world().get_resource::<ContentIntegrityReport>()
}

#[test]
fn a_def_short_of_the_views_it_owes_surfaces_in_the_editor_at_authoring_time() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    let Some(report) = published_report(&app) else {
        unreachable!("the ContentIntegrityReport resource must exist in the editor app")
    };
    let raised = missing_views(report, "Sparse Views Cover");
    assert_eq!(
        raised.len(),
        1,
        "the def is short two views and must be reported ONCE naming both, not once per view; \
         report: {:?}",
        report.findings(),
    );
    let Some(views) = raised.first() else { return };
    for view in ["Facing(South)", "Facing(West)"] {
        assert!(
            views.iter().any(|named| named == view),
            "the one finding must name {view}, a facing the def draws no art for; report: {:?}",
            report.findings(),
        );
    }
    assert_eq!(
        views.len(),
        2,
        "the finding names exactly the views the def is missing; report: {:?}",
        report.findings(),
    );
}

#[test]
fn a_leaves_behind_naming_no_terrain_def_surfaces_under_the_uuid_scheme() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    let Some(report) = published_report(&app) else {
        unreachable!("the ContentIntegrityReport resource must exist in the editor app")
    };
    assert!(
        has_dangling_ref(
            report,
            "TerrainDefRegistry",
            DANGLING_SUCCESSOR,
            ReferenceKeyScheme::Uuid,
        ),
        "a leaves_behind naming a successor def no registry holds must be reported against the \
         terrain registry under the UUID scheme; report: {:?}",
        report.findings(),
    );
}

#[test]
fn a_leaves_behind_naming_no_sprite_def_surfaces_under_the_file_stem_scheme() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    let Some(report) = published_report(&app) else {
        unreachable!("the ContentIntegrityReport resource must exist in the editor app")
    };
    assert!(
        has_dangling_ref(
            report,
            "SpriteDefRegistry",
            DANGLING_LEFTOVER,
            ReferenceKeyScheme::FileStem,
        ),
        "a leaves_behind naming a sprite no registry holds must be reported against the sprite \
         registry under the file-stem scheme; report: {:?}",
        report.findings(),
    );
}

#[test]
fn a_view_naming_no_sprite_def_surfaces_against_that_view() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    let Some(report) = published_report(&app) else {
        unreachable!("the ContentIntegrityReport resource must exist in the editor app")
    };
    let referrer = dangling_ref_referrer(
        report,
        "SpriteDefRegistry",
        DANGLING_VIEW_GRAPHIC,
        ReferenceKeyScheme::FileStem,
    );
    assert!(
        referrer.is_some(),
        "the one view naming a sprite no registry holds must be reported against the sprite \
         registry under the file-stem scheme; report: {:?}",
        report.findings(),
    );
    let Some(referrer) = referrer else { return };
    assert!(
        referrer.contains("Ghost View Cover") && referrer.contains("Facing(West)"),
        "the referrer names the def AND the view the bad key was authored on, or an author \
         cannot tell which row to fix; got `{referrer}`",
    );
    assert!(
        missing_views(report, "Ghost View Cover").is_empty(),
        "this def's view set is complete, so the finding above cannot be coming from it being \
         short of views; report: {:?}",
        report.findings(),
    );
}
