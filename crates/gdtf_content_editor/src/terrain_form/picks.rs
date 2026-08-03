use bevy::prelude::Component;
use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::terrain::{entity::TerrainPieceKind, piece::FootfallSound};

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TerrainKindChoice {
        #[default]
    Wall,
        Cover,
        Slab,
                Emplacement,
}

impl TerrainKindChoice {
        pub const SEGMENT_ORDER: [Self; 4] = [Self::Wall, Self::Cover, Self::Slab, Self::Emplacement];

            #[must_use]
    pub fn from_segment(index: usize) -> Option<Self> {
        Self::SEGMENT_ORDER.get(index).copied()
    }

        #[must_use]
    pub fn segment_index(self) -> usize {
        Self::SEGMENT_ORDER
            .iter()
            .position(|kind| *kind == self)
            .unwrap_or(0)
    }

        #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Wall => "Wall",
            Self::Cover => "Cover",
            Self::Slab => "Slab",
            Self::Emplacement => "Emplacement",
        }
    }

                #[must_use]
    pub const fn has_height_band(self) -> bool {
        matches!(self, Self::Wall | Self::Cover | Self::Emplacement)
    }

            #[must_use]
    pub const fn offers_footfall(self) -> bool {
        matches!(self, Self::Slab)
    }
}

impl From<TerrainPieceKind> for TerrainKindChoice {
                    fn from(kind: TerrainPieceKind) -> Self {
        match kind {
            TerrainPieceKind::Wall => Self::Wall,
            TerrainPieceKind::Cover => Self::Cover,
            TerrainPieceKind::Slab => Self::Slab,
            TerrainPieceKind::Emplacement => Self::Emplacement,
        }
    }
}

#[must_use]
pub fn offered_graphic_roles() -> Vec<TileRole> {
    TileRole::ALL
        .into_iter()
        .filter(|role| role.def_authorable())
        .collect()
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FootfallChoice {
            #[default]
    None,
        Metal,
        Grate,
}

impl FootfallChoice {
        pub const ALL: [Self; 3] = [Self::None, Self::Metal, Self::Grate];

        #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Metal => "Metal",
            Self::Grate => "Grate",
        }
    }

        #[must_use]
    pub fn footfall(self) -> Option<FootfallSound> {
        match self {
            Self::None => Option::None,
            Self::Metal => Some(FootfallSound::new("footfall_metal".to_owned())),
            Self::Grate => Some(FootfallSound::new("footfall_grate".to_owned())),
        }
    }
}
