//! Terrain tile role vocabulary and authored-key mapping.

/// Role of a terrain tile graphic.
/// [`as_key`](TileRole::as_key) is the authored key and [`from_key`](TileRole::from_key) its inverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileRole {
    /// Default floor tile.
    Floor,
    /// Alternate floor panel.
    FloorAltPanel,
    /// North-south wall.
    Wall,
    /// East-west wall.
    WallEw,
    /// Destructible cover.
    Cover,
    /// Empty emplacement.
    Emplacement,
    /// Emplacement with an occupant.
    EmplacementOccupied,
    /// Intact ceiling slab.
    Slab,
    /// Rubble after destruction.
    Rubble,
    /// Generic door (legacy).
    Door,
    /// Stair leading up (legacy).
    StairUp,
    /// Stair leading down (legacy).
    StairDown,
    /// Ladder connector.
    Ladder,
    /// North-south door.
    DoorNs,
    /// East-west door.
    DoorEw,
    /// North-south stair up.
    StairNsUp,
    /// North-south stair down.
    StairNsDown,
    /// East-west stair up.
    StairEwUp,
    /// East-west stair down.
    StairEwDown,
}

impl TileRole {
    /// Every role, in a stable order.
    pub const ALL: [Self; 19] = [
        Self::Floor,
        Self::FloorAltPanel,
        Self::Wall,
        Self::WallEw,
        Self::Cover,
        Self::Emplacement,
        Self::EmplacementOccupied,
        Self::Slab,
        Self::Rubble,
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

    /// Authored string key for this role.
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

    /// Parse a role from its authored key.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.as_key() == key)
    }

    /// Whether theme authors may assign a graphic for this role.
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
            Self::EmplacementOccupied | Self::StairUp | Self::StairDown | Self::Door => false,
        }
    }

    /// Opposite direction for stair pairs, if any.
    #[must_use]
    pub const fn counterpart(self) -> Option<Self> {
        match self {
            Self::StairUp => Some(Self::StairDown),
            Self::StairDown => Some(Self::StairUp),
            Self::StairNsUp => Some(Self::StairNsDown),
            Self::StairNsDown => Some(Self::StairNsUp),
            Self::StairEwUp => Some(Self::StairEwDown),
            Self::StairEwDown => Some(Self::StairEwUp),
            _ => None,
        }
    }

    /// Whether this role is an upward stair connector.
    #[must_use]
    pub const fn is_up_connector(self) -> bool {
        matches!(self, Self::StairUp | Self::StairNsUp | Self::StairEwUp)
    }
}
