/// The painted-map read this suite drives.
pub(crate) const EDITOR_MAP: &str = "editor.map";

/// The grid-extent write this suite drives.
pub(crate) const EDITOR_SET_GRID_SIZE: &str = "editor.set_grid_size";

/// The paint-tile write this suite drives.
pub(crate) const EDITOR_SELECT_TILE: &str = "editor.select_tile";

/// The paint-facing write this suite drives.
pub(crate) const EDITOR_SELECT_FACING: &str = "editor.select_facing";

/// The edit-storey write this suite drives.
pub(crate) const EDITOR_SET_LEVEL: &str = "editor.set_level";

/// The canvas write this suite drives.
pub(crate) const EDITOR_PAINT: &str = "editor.paint";

/// The authored-prefab load this suite drives.
pub(crate) const EDITOR_LOAD_PREFAB: &str = "editor.load_prefab";

/// The mode-tab write this suite sends to leave the Prefab tab.
pub(crate) const EDITOR_SET_MODE: &str = "editor.set_mode";

/// Every command this ticket scopes to the Prefab tab.
pub(crate) const PREFAB_TAB_COMMANDS: [&str; 7] = [
    EDITOR_MAP,
    EDITOR_SET_GRID_SIZE,
    EDITOR_SELECT_TILE,
    EDITOR_SELECT_FACING,
    EDITOR_SET_LEVEL,
    EDITOR_PAINT,
    EDITOR_LOAD_PREFAB,
];

/// Arguments each of them takes, in the same order, so one loop can call them all.
pub(crate) const PREFAB_TAB_ARGUMENTS: [&str; 7] = [
    "(level: 0)",
    "(width: 4, height: 4, levels: 2)",
    "(key: \"00000000-0000-0000-0000-000000000000\")",
    "(facing: East)",
    "(level: 0)",
    "(x: 0, y: 0)",
    "(name: \"no_such_room\", theme: \"00000000-0000-0000-0000-000000000000\", size: (width: 4, \
     height: 4, levels: 1), role: Fill)",
];
