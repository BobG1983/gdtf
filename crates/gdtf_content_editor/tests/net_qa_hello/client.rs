use gdtf_qa_protocol::{
    command::{CommandArgsRon, CommandName},
    message::{ProtocolVersion, QaRequest, QaResponse, RunCommand},
    ports::NetQaPort,
};

use crate::{
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

/// The registry-load write the editor host publishes.
pub(crate) const EDITOR_LOAD: &str = "editor.load";

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

/// The edit-storey write the editor host publishes.
pub(crate) const EDITOR_SET_LEVEL: &str = "editor.set_level";

/// The canvas write the editor host publishes.
pub(crate) const EDITOR_PAINT: &str = "editor.paint";

/// Every command name the editor host publishes today.
pub(crate) const EDITOR_COMMAND_NAMES: [&str; 20] = [
    EDITOR_PHASE,
    EDITOR_LAST_SAVE,
    EDITOR_VALIDATION,
    EDITOR_FAMILIES,
    EDITOR_SESSION,
    EDITOR_DRAFT,
    EDITOR_SET_MODE,
    EDITOR_NEW,
    EDITOR_LOAD,
    EDITOR_SAVE,
    EDITOR_SET_FIELD,
    EDITOR_LIST_OP,
    EDITOR_SELECT_THEME,
    EDITOR_TOGGLE_TERRAIN,
    EDITOR_SET_DEFAULT_FLOOR,
    EDITOR_MAP,
    EDITOR_SET_GRID_SIZE,
    EDITOR_SELECT_TILE,
    EDITOR_SET_LEVEL,
    EDITOR_PAINT,
];

/// Command names that need the authoring scene, so they refuse the editor's Load pass.
pub(crate) const EDITOR_EDITING_ONLY: [&str; 14] = [
    EDITOR_FAMILIES,
    EDITOR_SESSION,
    EDITOR_SET_MODE,
    EDITOR_NEW,
    EDITOR_LOAD,
    EDITOR_SAVE,
    EDITOR_SET_FIELD,
    EDITOR_LIST_OP,
    EDITOR_SELECT_THEME,
    EDITOR_MAP,
    EDITOR_SET_GRID_SIZE,
    EDITOR_SELECT_TILE,
    EDITOR_SET_LEVEL,
    EDITOR_PAINT,
];

/// Command names that also need the Terrain tab, so they refuse every other tab.
pub(crate) const EDITOR_TERRAIN_TAB_ONLY: [&str; 2] = [EDITOR_SET_FIELD, EDITOR_LIST_OP];

/// Command names that also need the Theme tab, so they refuse every other tab.
pub(crate) const EDITOR_THEME_TAB_ONLY: [&str; 2] =
    [EDITOR_TOGGLE_TERRAIN, EDITOR_SET_DEFAULT_FLOOR];

/// Command names that need any form tab, so they refuse the default Prefab tab.
pub(crate) const EDITOR_FORM_TAB_ONLY: [&str; 1] = [EDITOR_DRAFT];

pub(crate) fn run_editor_phase(arguments: &str) -> QaRequest {
    run_editor(EDITOR_PHASE, arguments)
}

pub(crate) fn run_a_misspelled_command_name() -> QaRequest {
    QaRequest::Run(RunCommand::new(
        CommandName::from_static("editor.phasee"),
        CommandArgsRon::new("()".to_owned()),
    ))
}

pub(crate) fn wrong_version() -> ProtocolVersion {
    ProtocolVersion::new(*ProtocolVersion::CURRENT + 1)
}

pub(crate) fn exchange_while_editing(
    app: &mut bevy::app::App,
    port: NetQaPort,
) -> Result<[QaResponse; EDITING_EXCHANGES], TestError> {
    let mut client = Client::connect(port)?;
    Ok([
        client.exchange(app, &QaRequest::Hello(ProtocolVersion::CURRENT))?,
        client.exchange(app, &QaRequest::Hello(wrong_version()))?,
        client.exchange(app, &run_a_misspelled_command_name())?,
    ])
}
