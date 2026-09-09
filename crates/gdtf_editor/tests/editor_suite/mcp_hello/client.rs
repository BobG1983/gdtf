use cobalt_mcp_protocol::{
    command::{CommandArgsRon, CommandName},
    message::{McpRequest, McpResponse, ProtocolVersion, RunCommand},
    ports::McpPort,
};

use crate::mcp_shared::{
    socket::{Client, run_editor},
    support::TestError,
};

/// How many exchanges the editing-phase handshake case makes on one connection.
pub(crate) const EDITING_EXCHANGES: usize = 3;

/// The lifecycle read the editor host publishes.
pub(crate) const EDITOR_PHASE: &str = "editor.phase";

/// The newest-save read the editor host publishes.
pub(crate) const EDITOR_LAST_SAVE: &str = "editor.last_save";

/// The content-integrity read the editor host publishes.
pub(crate) const EDITOR_VALIDATION: &str = "editor.validation";

/// The registry-keys read the editor host publishes.
pub(crate) const EDITOR_FAMILIES: &str = "editor.families";

/// The authoring-session read the editor host publishes.
pub(crate) const EDITOR_SESSION: &str = "editor.session";

/// The active-draft read the editor host publishes.
pub(crate) const EDITOR_DRAFT: &str = "editor.draft";

/// The mode-tab write the editor host publishes.
pub(crate) const EDITOR_SET_MODE: &str = "editor.set_mode";

/// The blank-draft write the editor host publishes.
pub(crate) const EDITOR_NEW: &str = "editor.new";

/// The Theme tab's registry-load write the editor host publishes.
pub(crate) const EDITOR_LOAD_THEME: &str = "editor.load_theme";

/// The Gang tab's registry-load write the editor host publishes.
pub(crate) const EDITOR_LOAD_GANG: &str = "editor.load_gang";

/// The Armor tab's registry-load write the editor host publishes.
pub(crate) const EDITOR_LOAD_ARMOR: &str = "editor.load_armor";

/// The Injury tab's registry-load write the editor host publishes.
pub(crate) const EDITOR_LOAD_INJURY: &str = "editor.load_injury";

/// The Sprite tab's registry-load write the editor host publishes.
pub(crate) const EDITOR_LOAD_SPRITE: &str = "editor.load_sprite";

/// The Attachment tab's registry-load write the editor host publishes.
pub(crate) const EDITOR_LOAD_ATTACHMENT: &str = "editor.load_attachment";

/// The Weapon tab's registry-load write the editor host publishes.
pub(crate) const EDITOR_LOAD_WEAPON: &str = "editor.load_weapon";

/// The `MeleeWeapon` tab's registry-load write the editor host publishes.
pub(crate) const EDITOR_LOAD_MELEE_WEAPON: &str = "editor.load_melee_weapon";

/// The Field tab's registry-load write the editor host publishes.
pub(crate) const EDITOR_LOAD_FIELD: &str = "editor.load_field";

/// The Terrain tab's registry-load write the editor host publishes.
pub(crate) const EDITOR_LOAD_TERRAIN: &str = "editor.load_terrain";

/// The Prefab tab's authored-prefab load the editor host publishes.
pub(crate) const EDITOR_LOAD_PREFAB: &str = "editor.load_prefab";

/// The draft-save write the editor host publishes.
pub(crate) const EDITOR_SAVE: &str = "editor.save";

/// The single-field write the editor host publishes.
pub(crate) const EDITOR_SET_FIELD: &str = "editor.set_field";

/// The list-field write the editor host publishes.
pub(crate) const EDITOR_LIST_OP: &str = "editor.list_op";

/// The session's theme write the editor host publishes.
pub(crate) const EDITOR_SELECT_THEME: &str = "editor.select_theme";

/// The theme draft's terrain-list write the editor host publishes.
pub(crate) const EDITOR_TOGGLE_TERRAIN: &str = "editor.toggle_terrain";

/// The theme draft's default-floor write the editor host publishes.
pub(crate) const EDITOR_SET_DEFAULT_FLOOR: &str = "editor.set_default_floor";

/// The painted-map read the editor host publishes.
pub(crate) const EDITOR_MAP: &str = "editor.map";

/// The prefab grid-extent write the editor host publishes.
pub(crate) const EDITOR_SET_GRID_SIZE: &str = "editor.set_grid_size";

/// The paint-tile write the editor host publishes.
pub(crate) const EDITOR_SELECT_TILE: &str = "editor.select_tile";

/// The paint-facing write the editor host publishes.
pub(crate) const EDITOR_SELECT_FACING: &str = "editor.select_facing";

/// The edit-storey write the editor host publishes.
pub(crate) const EDITOR_SET_LEVEL: &str = "editor.set_level";

/// The canvas write the editor host publishes.
pub(crate) const EDITOR_PAINT: &str = "editor.paint";

/// The Injury sub-tab write the editor host publishes.
pub(crate) const EDITOR_SELECT_INJURY_TAB: &str = "editor.select_injury_tab";

/// The weighting-table pick the editor host publishes.
pub(crate) const EDITOR_SELECT_WEIGHTING_TABLE: &str = "editor.select_weighting_table";

/// The weighting-table read the editor host publishes.
pub(crate) const EDITOR_WEIGHTING: &str = "editor.weighting";

/// The weighting-table save the editor host publishes.
pub(crate) const EDITOR_SAVE_WEIGHTING: &str = "editor.save_weighting";

/// The record delete the editor host publishes.
pub(crate) const EDITOR_DELETE_RECORD: &str = "editor.delete_record";

/// The screen capture the editor host publishes, under the game host's own spelling.
pub(crate) const CAPTURE_SCREENSHOT: &str = "capture.screenshot";

/// The condition hold the editor host publishes, under the game host's own spelling.
pub(crate) const WAIT: &str = "wait";

/// Every command name the editor host publishes today.
pub(crate) const EDITOR_COMMAND_NAMES: [&str; 38] = [
    EDITOR_PHASE,
    EDITOR_LAST_SAVE,
    EDITOR_VALIDATION,
    EDITOR_FAMILIES,
    EDITOR_SESSION,
    EDITOR_DRAFT,
    EDITOR_SET_MODE,
    EDITOR_NEW,
    EDITOR_LOAD_THEME,
    EDITOR_LOAD_GANG,
    EDITOR_LOAD_ARMOR,
    EDITOR_LOAD_INJURY,
    EDITOR_LOAD_SPRITE,
    EDITOR_LOAD_ATTACHMENT,
    EDITOR_LOAD_WEAPON,
    EDITOR_LOAD_MELEE_WEAPON,
    EDITOR_LOAD_FIELD,
    EDITOR_LOAD_TERRAIN,
    EDITOR_LOAD_PREFAB,
    EDITOR_SAVE,
    EDITOR_SET_FIELD,
    EDITOR_LIST_OP,
    EDITOR_SELECT_THEME,
    EDITOR_TOGGLE_TERRAIN,
    EDITOR_SET_DEFAULT_FLOOR,
    EDITOR_MAP,
    EDITOR_SET_GRID_SIZE,
    EDITOR_SELECT_TILE,
    EDITOR_SELECT_FACING,
    EDITOR_SET_LEVEL,
    EDITOR_PAINT,
    EDITOR_SELECT_INJURY_TAB,
    EDITOR_SELECT_WEIGHTING_TABLE,
    EDITOR_WEIGHTING,
    EDITOR_SAVE_WEIGHTING,
    CAPTURE_SCREENSHOT,
    WAIT,
    EDITOR_DELETE_RECORD,
];

/// Command names whose reply is parked and answered on a later frame.
pub(crate) const EDITOR_DEFERRED: [&str; 3] = [CAPTURE_SCREENSHOT, WAIT, EDITOR_DELETE_RECORD];

/// Command names that need the authoring scene, so they refuse the editor's Load pass.
pub(crate) const EDITOR_EDITING_ONLY: [&str; 16] = [
    EDITOR_FAMILIES,
    EDITOR_SESSION,
    EDITOR_SET_MODE,
    EDITOR_NEW,
    EDITOR_SAVE,
    EDITOR_SET_FIELD,
    EDITOR_LIST_OP,
    EDITOR_SELECT_THEME,
    EDITOR_MAP,
    EDITOR_SET_GRID_SIZE,
    EDITOR_SELECT_TILE,
    EDITOR_SELECT_FACING,
    EDITOR_SET_LEVEL,
    EDITOR_PAINT,
    EDITOR_LOAD_PREFAB,
    EDITOR_DELETE_RECORD,
];

/// Command names that also need the Theme tab, so they refuse every other tab.
pub(crate) const EDITOR_THEME_TAB_ONLY: [&str; 3] = [
    EDITOR_TOGGLE_TERRAIN,
    EDITOR_SET_DEFAULT_FLOOR,
    EDITOR_LOAD_THEME,
];

/// Command names that also need the Injury tab, so they refuse every other tab.
pub(crate) const EDITOR_INJURY_TAB_ONLY: [&str; 5] = [
    EDITOR_SELECT_INJURY_TAB,
    EDITOR_SELECT_WEIGHTING_TABLE,
    EDITOR_WEIGHTING,
    EDITOR_SAVE_WEIGHTING,
    EDITOR_LOAD_INJURY,
];

/// Command names that need any form tab, so they refuse the default Prefab tab.
pub(crate) const EDITOR_FORM_TAB_ONLY: [&str; 3] = [EDITOR_DRAFT, EDITOR_SET_FIELD, EDITOR_LIST_OP];

/// Command names scoped to one form tab that the catalogue is never read on, so both phases
/// refuse them.
pub(crate) const EDITOR_ONE_FORM_TAB_ONLY: [&str; 8] = [
    EDITOR_LOAD_GANG,
    EDITOR_LOAD_ARMOR,
    EDITOR_LOAD_SPRITE,
    EDITOR_LOAD_ATTACHMENT,
    EDITOR_LOAD_WEAPON,
    EDITOR_LOAD_MELEE_WEAPON,
    EDITOR_LOAD_FIELD,
    EDITOR_LOAD_TERRAIN,
];

pub(crate) fn run_editor_phase(arguments: &str) -> McpRequest {
    run_editor(EDITOR_PHASE, arguments)
}

pub(crate) fn run_a_misspelled_command_name() -> McpRequest {
    McpRequest::Run(RunCommand::new(
        CommandName::from_static("editor.phasee"),
        CommandArgsRon::new("()".to_owned()),
    ))
}

pub(crate) fn wrong_version() -> ProtocolVersion {
    ProtocolVersion::new(*ProtocolVersion::CURRENT + 1)
}

pub(crate) fn exchange_while_editing(
    app: &mut bevy::app::App,
    port: McpPort,
) -> Result<[McpResponse; EDITING_EXCHANGES], TestError> {
    let mut client = Client::connect(port)?;
    Ok([
        client.exchange(app, &McpRequest::Hello(ProtocolVersion::CURRENT))?,
        client.exchange(app, &McpRequest::Hello(wrong_version()))?,
        client.exchange(app, &run_a_misspelled_command_name())?,
    ])
}
