//! the terrain→sprite-def edge's authoring-time pin — a terrain
//! authoring time).

use gdtf_assets::{ContentIntegrityReport, ReferenceKeyScheme};

use crate::{
    advance::advance_to_published, findings::has_dangling_ref, harness::editor_app_on_fixture_root,
};

const DANGLING_GRAPHIC: &str = "ghost_graphic";

#[test]
fn a_dangling_terrain_view_sprite_surfaces_in_the_editor_at_authoring_time() {
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
            "SpriteDefRegistry",
            DANGLING_GRAPHIC,
            ReferenceKeyScheme::FileStem,
        ),
        "the terrain def's dangling view sprite key must be reported at authoring time \
         (the terrain view→sprite-def edge); report: {:?}",
        report.findings(),
    );
}
