/// The painted-map read this suite drives.
pub(crate) const EDITOR_MAP: &str = "editor.map";

/// The grid-extent write this suite drives.
pub(crate) const EDITOR_SET_GRID_SIZE: &str = "editor.set_grid_size";

/// The paint-tile write this suite drives.
pub(crate) const EDITOR_SELECT_TILE: &str = "editor.select_tile";

/// The edit-storey write this suite drives.
pub(crate) const EDITOR_SET_LEVEL: &str = "editor.set_level";

/// The canvas write this suite drives.
pub(crate) const EDITOR_PAINT: &str = "editor.paint";

/// The mode-tab write this suite sends to leave the Prefab tab.
pub(crate) const EDITOR_SET_MODE: &str = "editor.set_mode";

/// Every command this ticket scopes to the Prefab tab.
pub(crate) const PREFAB_TAB_COMMANDS: [&str; 5] = [
    EDITOR_MAP,
    EDITOR_SET_GRID_SIZE,
    EDITOR_SELECT_TILE,
    EDITOR_SET_LEVEL,
    EDITOR_PAINT,
];

/// Arguments each of those five takes, in the same order, so one loop can call them all.
pub(crate) const PREFAB_TAB_ARGUMENTS: [&str; 5] = [
    "(level: 0)",
    "(width: 4, height: 4, levels: 2)",
    "(key: \"00000000-0000-0000-0000-000000000000\")",
    "(level: 0)",
    "(x: 0, y: 0)",
];
