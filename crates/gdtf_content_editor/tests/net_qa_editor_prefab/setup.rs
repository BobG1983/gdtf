use bevy::app::App;
use gdtf_battle_sim::terrain::def::TerrainUuid;
use gdtf_content_editor::EditorMode;

use crate::{
    harness::editing_app_and_client,
    names::{EDITOR_MAP, EDITOR_PAINT, EDITOR_SELECT_TILE, EDITOR_SET_GRID_SIZE, EDITOR_SET_LEVEL},
    outcome::ran_body,
    rows::{MapReplyRow, PaintReplyRow, SelectTileReplyRow, SetGridSizeReplyRow, SetLevelReplyRow},
    socket::{Client, run_editor},
    support::TestError,
    world::editor_mode,
};

/// An editing app on the Prefab tab, which is the tab a fresh process already has open.
pub(crate) fn prefab_app_and_client() -> Result<(App, Client), TestError> {
    let (app, client) = editing_app_and_client()?;
    let mode = editor_mode(&app)?;
    if mode != EditorMode::Prefab {
        return Err(format!(
            "the Prefab tab carries the mode resource's own default, so a fresh process needs no \
             editor.set_mode first, and this one opened on {mode:?}"
        )
        .into());
    }
    Ok((app, client))
}

/// The arguments `editor.select_tile` takes, for any key text a case wants to send.
pub(crate) fn select_tile_args(key: &str) -> String {
    format!("(key: \"{key}\")")
}

/// Run `editor.select_tile` for a key the caller chose and hand back what it answered.
pub(crate) fn select_tile(
    app: &mut App,
    client: &mut Client,
    key: TerrainUuid,
) -> Result<SelectTileReplyRow, TestError> {
    let arguments = select_tile_args(&(*key).to_string());
    let reply = client.exchange(app, &run_editor(EDITOR_SELECT_TILE, &arguments))?;
    ran_body(&reply, EDITOR_SELECT_TILE)
}

/// Run `editor.paint` at one cell of the storey being edited.
pub(crate) fn paint(
    app: &mut App,
    client: &mut Client,
    x: i32,
    y: i32,
) -> Result<PaintReplyRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_PAINT, &format!("(x: {x}, y: {y})")))?;
    ran_body(&reply, EDITOR_PAINT)
}

/// Run `editor.map` for one storey.
pub(crate) fn read_map(
    app: &mut App,
    client: &mut Client,
    level: u8,
) -> Result<MapReplyRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_MAP, &format!("(level: {level})")))?;
    ran_body(&reply, EDITOR_MAP)
}

/// Run `editor.set_level` for one storey.
pub(crate) fn set_level(
    app: &mut App,
    client: &mut Client,
    level: u8,
) -> Result<SetLevelReplyRow, TestError> {
    let arguments = format!("(level: {level})");
    let reply = client.exchange(app, &run_editor(EDITOR_SET_LEVEL, &arguments))?;
    ran_body(&reply, EDITOR_SET_LEVEL)
}

/// Run `editor.set_grid_size` for three spans the caller chose.
pub(crate) fn set_grid_size(
    app: &mut App,
    client: &mut Client,
    width: u8,
    height: u8,
    levels: u8,
) -> Result<SetGridSizeReplyRow, TestError> {
    let arguments = format!("(width: {width}, height: {height}, levels: {levels})");
    let reply = client.exchange(app, &run_editor(EDITOR_SET_GRID_SIZE, &arguments))?;
    ran_body(&reply, EDITOR_SET_GRID_SIZE)
}
