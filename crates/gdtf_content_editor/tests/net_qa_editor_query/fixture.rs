//! The authoring-model fixture the `Editing` phase reads (GTW-805).

use bevy::prelude::*;
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, FindingFamily, FindingReferrer, FindingTarget,
    ReferenceKeyScheme,
};
use gdtf_battle_sim::{
    level::{GridHeight, GridLevels, GridSize, GridWidth, ThemeUuid},
    terrain::def::TerrainUuid,
};
use gdtf_content_editor::{EditorMode, EditorState, MapEditorSession, TerrainDraft};

use crate::support::{DANGLING_KEY, DRAFT_NAME};

/// Insert the state-scoped authoring model the topics read, and enter `Editing`.
///
/// The model resources are inserted directly rather than by running the editor's
/// `OnEnter(Editing)` lifecycle: this suite drives the QA channel, not the editor's asset
/// pass, and a headless `MinimalPlugins` app loads no assets. The resources are the same
/// types the editor's own lifecycle inserts, so the service reads exactly what it does in
/// the running editor.
pub(crate) fn open_the_editing_scene(app: &mut App) {
    let floor = TerrainUuid::generate();
    let Ok(grid) = GridSize::new(GridWidth::new(12), GridHeight::new(9), GridLevels::new(2)) else {
        unreachable!("the fixture grid size must be in bounds");
    };
    let mut draft = TerrainDraft::default();
    draft.set_display_name(DRAFT_NAME.to_owned());
    let mut report = ContentIntegrityReport::default();
    report.record(ContentFinding::DanglingRef {
        referrer: FindingReferrer::new("content/themes/fixture.terrain_theme.ron".to_owned()),
        target:   FindingTarget::new(DANGLING_KEY.to_owned()),
        family:   FindingFamily::new("TerrainDefRegistry".to_owned()),
        scheme:   ReferenceKeyScheme::Uuid,
    });

    let world = app.world_mut();
    world.insert_resource(EditorMode::Terrain);
    world.insert_resource(MapEditorSession::new(
        ThemeUuid::generate(),
        Some(floor),
        grid,
    ));
    world.insert_resource(draft);
    world.insert_resource(report);
    world.insert_resource(NextState::Pending(EditorState::Editing));
}
