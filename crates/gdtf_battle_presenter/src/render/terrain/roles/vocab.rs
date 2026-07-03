//! The presenter-owned terrain graphic-role VOCABULARY (GTW-566) — one [`TileRole`]
//! variant per [`TileRoles`] field, so a role key is spelled in exactly ONE place.

use super::table::{TileIndex, TileRoles};

/// A terrain graphic ROLE — the CLOSED Rust vocabulary over the [`TileRoles`] table,
/// one variant per authored `tile_roles.spritedef.ron` key (GTW-566 C1).
///
/// Before this enum the role vocabulary was spelled in three independently-maintained
/// places (the [`TileRoles`] field set, a ~20-key string match, and a hand-mirrored
/// editor pick enum), so they drifted. Now [`as_key`](TileRole::as_key) is the exact
/// authored RON key, [`from_key`](TileRole::from_key) its inverse, and
/// [`index_in`](TileRole::index_in) the exhaustive field read —
/// [`TileRoles::index_for_key`], the draw's role-default fallback, the editor's picker
/// (via [`def_authorable`](TileRole::def_authorable)), and the prefab connector pairing
/// (via [`counterpart`](TileRole::counterpart)) all derive from it rather than
/// re-listing key strings. The sim's
/// [`TerrainGraphicKey`](gdtf_battle_sim::TerrainGraphicKey) stays an opaque string
/// newtype — this type never crosses into the sim (the render-free boundary); the
/// presenter classifies the sim's string at its own edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileRole {
    /// [`TileRoles::floor`] — the default walkable-ground tile.
    Floor,
    /// [`TileRoles::floor_alt_panel`] — the bolted-panel floor alternate.
    FloorAltPanel,
    /// [`TileRoles::wall`] — the default (NS-orientation) wall tile.
    Wall,
    /// [`TileRoles::wall_ew`] — the east-west-running wall tile (GTW-469).
    WallEw,
    /// [`TileRoles::cover`] — the chest-high cover tile.
    Cover,
    /// [`TileRoles::emplacement`] — the VACANT weapon-emplacement tile (GTW-543).
    Emplacement,
    /// [`TileRoles::emplacement_occupied`] — the OCCUPIED (manned) emplacement tile,
    /// swapped in at runtime (GTW-543).
    EmplacementOccupied,
    /// [`TileRoles::slab`] — the elevated-deck slab tile.
    Slab,
    /// [`TileRoles::rubble`] — the broken-debris destroyed-cover tile.
    Rubble,
    /// [`TileRoles::slab_destroyed`] — the destroyed-SLAB tile, swapped in at runtime
    /// (GTW-367).
    SlabDestroyed,
    /// [`TileRoles::door`] — the plain doorway / hatch tile (authored for future
    /// variety).
    Door,
    /// [`TileRoles::stair_up`] — the stair-endpoint tile you ASCEND from, chosen by
    /// link direction (GTW-373).
    StairUp,
    /// [`TileRoles::stair_down`] — the stair-endpoint tile you DESCEND from, chosen by
    /// link direction (GTW-373).
    StairDown,
    /// [`TileRoles::ladder`] — the ladder link-cell tile.
    Ladder,
    /// [`TileRoles::door_ns`] — the north-south door orientation (GTW-470).
    DoorNs,
    /// [`TileRoles::door_ew`] — the east-west door orientation (GTW-470).
    DoorEw,
    /// [`TileRoles::stair_ns_up`] — the north-south ascend stair (GTW-470).
    StairNsUp,
    /// [`TileRoles::stair_ns_down`] — the north-south descend stair (GTW-470).
    StairNsDown,
    /// [`TileRoles::stair_ew_up`] — the east-west ascend stair (GTW-470).
    StairEwUp,
    /// [`TileRoles::stair_ew_down`] — the east-west descend stair (GTW-470).
    StairEwDown,
}

impl TileRole {
    /// Every role, in [`TileRoles`] field order — the completeness anchor the
    /// vocabulary tests and the editor's authorable filter iterate.
    pub const ALL: [Self; 20] = [
        Self::Floor,
        Self::FloorAltPanel,
        Self::Wall,
        Self::WallEw,
        Self::Cover,
        Self::Emplacement,
        Self::EmplacementOccupied,
        Self::Slab,
        Self::Rubble,
        Self::SlabDestroyed,
        Self::Door,
        Self::StairUp,
        Self::StairDown,
        Self::Ladder,
        Self::DoorNs,
        Self::DoorEw,
        Self::StairNsUp,
        Self::StairNsDown,
        Self::StairEwUp,
        Self::StairEwDown,
    ];

    /// The exact authored `tile_roles.spritedef.ron` key for this role — the ONE place
    /// a graphic-role key string is spelled in production Rust (GTW-566 C2).
    #[must_use]
    pub const fn as_key(self) -> &'static str {
        match self {
            Self::Floor => "floor",
            Self::FloorAltPanel => "floor_alt_panel",
            Self::Wall => "wall",
            Self::WallEw => "wall_ew",
            Self::Cover => "cover",
            Self::Emplacement => "emplacement",
            Self::EmplacementOccupied => "emplacement_occupied",
            Self::Slab => "slab",
            Self::Rubble => "rubble",
            Self::SlabDestroyed => "slab_destroyed",
            Self::Door => "door",
            Self::StairUp => "stair_up",
            Self::StairDown => "stair_down",
            Self::Ladder => "ladder",
            Self::DoorNs => "door_ns",
            Self::DoorEw => "door_ew",
            Self::StairNsUp => "stair_ns_up",
            Self::StairNsDown => "stair_ns_down",
            Self::StairEwUp => "stair_ew_up",
            Self::StairEwDown => "stair_ew_down",
        }
    }

    /// The role for an authored key, or [`None`] for an out-of-vocabulary string —
    /// the inverse of [`as_key`](Self::as_key), DERIVED from [`ALL`](Self::ALL) (not a
    /// second string list) so the two can never drift.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.as_key() == key)
    }

    /// The [`TileIndex`] this role resolves to in `roles` — the exhaustive per-variant
    /// field read. A variant without a matching [`TileRoles`] field fails HERE at
    /// compile time; a field without a variant fails the vocabulary-completeness test.
    #[must_use]
    pub const fn index_in(self, roles: &TileRoles) -> TileIndex {
        match self {
            Self::Floor => roles.floor,
            Self::FloorAltPanel => roles.floor_alt_panel,
            Self::Wall => roles.wall,
            Self::WallEw => roles.wall_ew,
            Self::Cover => roles.cover,
            Self::Emplacement => roles.emplacement,
            Self::EmplacementOccupied => roles.emplacement_occupied,
            Self::Slab => roles.slab,
            Self::Rubble => roles.rubble,
            Self::SlabDestroyed => roles.slab_destroyed,
            Self::Door => roles.door,
            Self::StairUp => roles.stair_up,
            Self::StairDown => roles.stair_down,
            Self::Ladder => roles.ladder,
            Self::DoorNs => roles.door_ns,
            Self::DoorEw => roles.door_ew,
            Self::StairNsUp => roles.stair_ns_up,
            Self::StairNsDown => roles.stair_ns_down,
            Self::StairEwUp => roles.stair_ew_up,
            Self::StairEwDown => roles.stair_ew_down,
        }
    }

    /// Whether a terrain DEF may name this role as its `graphic_name` — the flag the
    /// editor's graphic picker filters [`ALL`](Self::ALL) by (GTW-566 C5), derived from
    /// the [`TileRoles`] field docs:
    ///
    /// - [`EmplacementOccupied`](Self::EmplacementOccupied) and
    ///   [`SlabDestroyed`](Self::SlabDestroyed) are RUNTIME-SWAP roles — an emplacement
    ///   is always authored vacant (occupancy is runtime state) and a destroyed slab is
    ///   a destruction swap, so neither is reachable via a def `graphic_name`.
    /// - [`StairUp`](Self::StairUp) / [`StairDown`](Self::StairDown) are LINK-DIRECTION
    ///   roles — `draw_vertical_links` picks one per stair endpoint by the link's
    ///   direction relative to the active storey, never from a def key.
    /// - The plain [`Door`](Self::Door) is authored for future variety and stays
    ///   unoffered (the authorable door orientations are [`DoorNs`](Self::DoorNs) /
    ///   [`DoorEw`](Self::DoorEw)).
    ///
    /// Exhaustive on both sides (no wildcard) so adding a variant forces an explicit
    /// authorability decision.
    #[must_use]
    pub const fn def_authorable(self) -> bool {
        match self {
            Self::Floor
            | Self::FloorAltPanel
            | Self::Wall
            | Self::WallEw
            | Self::Cover
            | Self::Emplacement
            | Self::Slab
            | Self::Rubble
            | Self::Ladder
            | Self::DoorNs
            | Self::DoorEw
            | Self::StairNsUp
            | Self::StairNsDown
            | Self::StairEwUp
            | Self::StairEwDown => true,
            Self::EmplacementOccupied
            | Self::SlabDestroyed
            | Self::StairUp
            | Self::StairDown
            | Self::Door => false,
        }
    }

    /// The paired opposite END of a two-ended stair connector (GTW-566 C6) —
    /// `stair_up`↔`stair_down`, `stair_ns_up`↔`stair_ns_down`,
    /// `stair_ew_up`↔`stair_ew_down` — or [`None`] for every unpaired role.
    ///
    /// Bidirectional and typed: the prefab editor's connector auto-pairing (GTW-531)
    /// resolves a placed UP connector's DOWN counterpart through this instead of
    /// string-suffix surgery, so an out-of-vocabulary `*_up` name can never
    /// phantom-pair (fail-closed).
    #[must_use]
    pub const fn counterpart(self) -> Option<Self> {
        match self {
            Self::StairUp => Some(Self::StairDown),
            Self::StairDown => Some(Self::StairUp),
            Self::StairNsUp => Some(Self::StairNsDown),
            Self::StairNsDown => Some(Self::StairNsUp),
            Self::StairEwUp => Some(Self::StairEwDown),
            Self::StairEwDown => Some(Self::StairEwUp),
            // Every other role is single-ended — fail-closed, no pair.
            _ => None,
        }
    }

    /// Whether this role is the ASCEND end of a stair pair — the recognition half of
    /// the typed connector pairing (GTW-566 C6): the prefab editor auto-places the
    /// [`counterpart`](Self::counterpart) one storey up only for an up connector.
    #[must_use]
    pub const fn is_up_connector(self) -> bool {
        matches!(self, Self::StairUp | Self::StairNsUp | Self::StairEwUp)
    }
}
