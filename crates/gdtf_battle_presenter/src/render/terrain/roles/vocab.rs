//! The presenter-owned terrain graphic-role VOCABULARY (GTW-566) — one [`TileRole`]
//! variant per renderer-known graphic key, so a role key is spelled in exactly ONE place.

/// A terrain graphic ROLE — the CLOSED Rust vocabulary of renderer-known graphic
/// keys (GTW-566 C1).
///
/// Before this enum the role vocabulary was spelled in three independently-maintained
/// places (a role-table field set, a ~20-key string match, and a hand-mirrored
/// editor pick enum), so they drifted. Now [`as_key`](TileRole::as_key) is the exact
/// authored key and [`from_key`](TileRole::from_key) its inverse — the draw's
/// sim-fact fallback mapping, the editor's picker
/// (via [`def_authorable`](TileRole::def_authorable)), and the prefab connector pairing
/// (via [`counterpart`](TileRole::counterpart)) all derive from it rather than
/// re-listing key strings. GTW-665: the role→atlas-index TABLE this enum used to
/// index into is RETIRED — each key now resolves through the sprite-def registry
/// ([`resolve_sprite`](crate::render::terrain::resolve::resolve_sprite)); the enum
/// stays the closed vocabulary/concern boundary. The sim's
/// [`TerrainGraphicKey`](gdtf_battle_sim::piece::TerrainGraphicKey) stays an opaque string
/// newtype — this type never crosses into the sim (the render-free boundary); the
/// presenter classifies the sim's string at its own edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileRole {
    /// `floor` — the default walkable-ground tile.
    Floor,
    /// `floor_alt_panel` — the bolted-panel floor alternate.
    FloorAltPanel,
    /// `wall` — the default (NS-orientation) wall tile.
    Wall,
    /// `wall_ew` — the east-west-running wall tile (GTW-469).
    WallEw,
    /// `cover` — the chest-high cover tile.
    Cover,
    /// `emplacement` — the VACANT weapon-emplacement tile (GTW-543).
    Emplacement,
    /// `emplacement_occupied` — the OCCUPIED (manned) emplacement tile,
    /// swapped in at runtime (GTW-543).
    EmplacementOccupied,
    /// `slab` — the elevated-deck slab tile.
    Slab,
    /// `rubble` — the broken-debris destroyed-cover tile.
    Rubble,
    /// `slab_destroyed` — the destroyed-SLAB tile, swapped in at runtime
    /// (GTW-367).
    SlabDestroyed,
    /// `door` — the plain doorway / hatch tile (authored for future
    /// variety).
    Door,
    /// `stair_up` — the stair-endpoint tile you ASCEND from, chosen by
    /// link direction (GTW-373).
    StairUp,
    /// `stair_down` — the stair-endpoint tile you DESCEND from, chosen by
    /// link direction (GTW-373).
    StairDown,
    /// `ladder` — the ladder link-cell tile.
    Ladder,
    /// `door_ns` — the north-south door orientation (GTW-470).
    DoorNs,
    /// `door_ew` — the east-west door orientation (GTW-470).
    DoorEw,
    /// `stair_ns_up` — the north-south ascend stair (GTW-470).
    StairNsUp,
    /// `stair_ns_down` — the north-south descend stair (GTW-470).
    StairNsDown,
    /// `stair_ew_up` — the east-west ascend stair (GTW-470).
    StairEwUp,
    /// `stair_ew_down` — the east-west descend stair (GTW-470).
    StairEwDown,
}

impl TileRole {
    /// Every role, in the authored-catalog order — the completeness anchor the
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

    /// The exact authored graphic key for this role — the ONE place a
    /// graphic-role key string is spelled in production Rust (GTW-566 C2).
    /// Since GTW-665 the key doubles as the role's SPRITE NAME: the seeded
    /// `assets/content/sprites/<key>.spritedef.ron` catalog carries one def
    /// per key (the roles/test lockstep pin), so a role resolves through
    /// [`resolve_sprite`](crate::render::terrain::resolve::resolve_sprite).
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

    /// Whether a terrain DEF may name this role as its `graphic_name` — the flag the
    /// editor's graphic picker filters [`ALL`](Self::ALL) by (GTW-566 C5):
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
