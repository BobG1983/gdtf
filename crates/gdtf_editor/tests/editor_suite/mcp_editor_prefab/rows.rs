use serde::Deserialize;

/// A slot as the editor answers it: cell plus storey.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct CellRow {
    pub(crate) x:     i32,
    pub(crate) y:     i32,
    pub(crate) level: u8,
}

/// The grid extent as the editor answers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct GridSizeRow {
    pub(crate) width:  u8,
    pub(crate) height: u8,
    pub(crate) levels: u8,
}

/// A client's own reading of a cardinal side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum FacingRow {
    North,
    East,
    South,
    West,
}

/// One painted slot as `editor.map` answers it.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PaintedRow {
    pub(crate) cell:   CellRow,
    pub(crate) tile:   String,
    pub(crate) facing: FacingRow,
}

/// `editor.map`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct MapReplyRow {
    pub(crate) grid_size: GridSizeRow,
    pub(crate) painted:   Vec<PaintedRow>,
}

/// `editor.set_grid_size`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SetGridSizeReplyRow {
    pub(crate) grid_size: GridSizeRow,
    pub(crate) level:     u8,
}

/// `editor.set_level`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SetLevelReplyRow {
    pub(crate) level: u8,
}

/// Which of the palette's two conditions a tile key failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum SelectTileRefusalRow {
    NoTerrainDef,
    NotInTheThemePalette,
}

/// `editor.select_tile`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SelectTileReplyRow {
    pub(crate) refusal:       Option<SelectTileRefusalRow>,
    pub(crate) selected_tile: Option<String>,
}

/// Why a paint wrote nothing before any placement rule was reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum PaintRefusalRow {
    NoSelectedTile,
}

/// Why the placement rules turned a slot down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum IllegalReasonRow {
    OutOfBounds,
    SlabSealsLadder,
}

/// The verdict the rules returned for the slot the caller asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum VerdictRow {
    Legal {
        /// The slot the placement clears on its way in.
        auto_clear: Option<CellRow>,
    },
    Illegal(IllegalReasonRow),
}

/// What the connector pairing pass did after the tile landed.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum PairingRow {
    Rejected,
    PlacedNoPair,
    PlacedPairSkipped,
    PairPlaced { paired: String, at: CellRow },
}

/// `editor.select_facing`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SelectFacingReplyRow {
    pub(crate) facing: FacingRow,
}

/// `editor.paint`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct PaintReplyRow {
    pub(crate) refusal: Option<PaintRefusalRow>,
    pub(crate) verdict: Option<VerdictRow>,
    pub(crate) pairing: Option<PairingRow>,
}

/// A client's own reading of a prefab's spawn role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum SpawnRoleRow {
    Player,
    Enemy,
    Fill,
}

/// What `editor.load_prefab` did to the canvas.
#[derive(Debug, Deserialize)]
pub(crate) enum LoadPrefabOutcomeRow {
    Opened {
        /// The extent the loaded prefab put on the session.
        grid_size:  GridSizeRow,
        /// How many cells it painted.
        placements: usize,
    },
    NoSuchPrefab {
        /// The prefab name that was asked for.
        name:  String,
        /// The theme half of the key that was asked for.
        theme: String,
        /// The extent half of the key that was asked for.
        size:  GridSizeRow,
        /// The role half of the key that was asked for.
        role:  SpawnRoleRow,
    },
}

/// `editor.load_prefab`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct LoadPrefabReplyRow {
    pub(crate) outcome: LoadPrefabOutcomeRow,
}

/// The two `editor.session` fields this suite reads back after a load.
#[derive(Debug, Deserialize)]
pub(crate) struct SessionGridRow {
    pub(crate) theme:     String,
    pub(crate) grid_size: GridSizeRow,
}
