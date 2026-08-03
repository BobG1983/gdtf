/// authored key and [`from_key`](TileRole::from_key) its inverse — the draw's
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileRole {
        Floor,
        FloorAltPanel,
        Wall,
        WallEw,
        Cover,
        Emplacement,
            EmplacementOccupied,
        Slab,
        Rubble,
            SlabDestroyed,
            Door,
            StairUp,
            StairDown,
        Ladder,
        DoorNs,
        DoorEw,
        StairNsUp,
        StairNsDown,
        StairEwUp,
        StairEwDown,
}

impl TileRole {
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

                #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.as_key() == key)
    }

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

                #[must_use]
    pub const fn is_up_connector(self) -> bool {
        matches!(self, Self::StairUp | Self::StairNsUp | Self::StairEwUp)
    }
}
