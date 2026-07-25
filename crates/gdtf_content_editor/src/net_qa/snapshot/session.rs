//! The SESSION topic — the authoring session's selections (GTW-805).

use gdtf_battle_sim::{level::GridSize, terrain::def::TerrainUuid};
use gdtf_qa_protocol::view::{
    EditorSessionView, EditorTerrainKeyNet, EditorThemeKeyNet, GridHeightNet, GridLevelsNet,
    GridSizeNet, GridWidthNet,
};

use crate::MapEditorSession;

/// The SESSION topic's answer: the selected theme, the default floor it resolved, the
/// drawable-area dimensions, and the active paint tile.
///
/// The UUID keys go on the wire in their text form — the protocol crate stays free of a UUID
/// dependency, and a client compares the same string the editor's own save writes.
pub(super) fn session_view(session: &MapEditorSession) -> EditorSessionView {
    EditorSessionView::new(
        EditorThemeKeyNet::new(session.theme().to_string()),
        session.default_floor().map(terrain_key),
        grid_size(session.grid_size()),
        session.selected_tile().map(terrain_key),
    )
}

/// One terrain key in its wire text form.
fn terrain_key(terrain: TerrainUuid) -> EditorTerrainKeyNet {
    EditorTerrainKeyNet::new(terrain.to_string())
}

/// The drawable area's dimensions on the wire. The sim spans are `u8`-backed and the wire
/// ones are deliberately widened, so every value fits.
fn grid_size(size: GridSize) -> GridSizeNet {
    GridSizeNet::new(
        GridWidthNet::new(u16::from(*size.width())),
        GridHeightNet::new(u16::from(*size.height())),
        GridLevelsNet::new(*size.levels()),
    )
}
