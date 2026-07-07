//! GTW-663 C3: the terrain→sprite-def edge's authoring-time pin — a terrain
//! def whose `graphic_name` resolves NO sprite def surfaces as a
//! `DanglingRef` finding against the `SpriteDefRegistry` in the EDITOR's
//! report (the `graphic_name` foreign key of the GTW-600 ruling, validated at
//! authoring time).

use gdtf_assets::ContentIntegrityReport;

use crate::harness::{advance_to_published, editor_app_on_fixture_root, has_dangling_ref};

/// The fixture terrain def's DANGLING `graphic_name` (the fixture root
/// materializes no `content/sprites/` folder at all, so the sprite-def
/// registry fails closed EMPTY and the authored key resolves nothing).
const DANGLING_GRAPHIC: &str = "ghost_graphic";

/// C3 red-first pin: loading a terrain def whose `graphic_name` names a
/// missing sprite def surfaces a `DanglingRef` finding against the
/// `SpriteDefRegistry` on the [`ContentIntegrityReport`] in the EDITOR app —
/// the same report shape the game publishes at the end of `Load`.
#[test]
fn dangling_terrain_graphic_name_surfaces_in_the_editor_at_authoring_time() {
    let mut app = editor_app_on_fixture_root();
    advance_to_published(&mut app);

    let report = app.world().get_resource::<ContentIntegrityReport>();
    assert!(
        report.is_some(),
        "the ContentIntegrityReport resource must exist in the editor app",
    );
    let Some(report) = report else { return };
    assert!(
        has_dangling_ref(report, "SpriteDefRegistry", DANGLING_GRAPHIC),
        "the terrain def's dangling graphic_name must be reported at authoring time \
         (the GTW-663 terrain→sprite-def edge); report: {:?}",
        report.findings(),
    );
}
