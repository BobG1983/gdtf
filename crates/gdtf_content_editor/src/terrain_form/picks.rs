//! Terrain kind and footfall UI choices.

use bevy::prelude::Component;
use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::terrain::{entity::TerrainPieceKind, piece::FootfallSound};

/// Terrain kind selected in the form.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TerrainKindChoice {
    /// Wall.
    #[default]
    Wall,
    /// Cover.
    Cover,
    /// Floor slab.
    Slab,
    /// Emplacement with optional mounted weapon.
    Emplacement,
}

impl TerrainKindChoice {
    /// Segment control order left-to-right.
    pub const SEGMENT_ORDER: [Self; 4] = [Self::Wall, Self::Cover, Self::Slab, Self::Emplacement];

    /// Kind for a segment index, if in range.
    #[must_use]
    pub fn from_segment(index: usize) -> Option<Self> {
        Self::SEGMENT_ORDER.get(index).copied()
    }

    /// Index of this kind in [`Self::SEGMENT_ORDER`].
    #[must_use]
    pub fn segment_index(self) -> usize {
        Self::SEGMENT_ORDER
            .iter()
            .position(|kind| *kind == self)
            .unwrap_or(0)
    }

    /// Short UI label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Wall => "Wall",
            Self::Cover => "Cover",
            Self::Slab => "Slab",
            Self::Emplacement => "Emplacement",
        }
    }

    /// Whether height band applies.
    #[must_use]
    pub const fn has_height_band(self) -> bool {
        matches!(self, Self::Wall | Self::Cover | Self::Emplacement)
    }

    /// Whether footfall choice applies.
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

/// Graphic roles authors may pick for a terrain def.
#[must_use]
pub fn offered_graphic_roles() -> Vec<TileRole> {
    TileRole::ALL
        .into_iter()
        .filter(|role| role.def_authorable())
        .collect()
}

/// Footfall sound choice for slabs.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FootfallChoice {
    /// No footfall sound.
    #[default]
    None,
    /// Metal footfall.
    Metal,
    /// Grate footfall.
    Grate,
}

impl FootfallChoice {
    /// All choices in UI order.
    pub const ALL: [Self; 3] = [Self::None, Self::Metal, Self::Grate];

    /// Short UI label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Metal => "Metal",
            Self::Grate => "Grate",
        }
    }

    /// Map to a sim footfall sound, if any.
    #[must_use]
    pub fn footfall(self) -> Option<FootfallSound> {
        match self {
            Self::None => Option::None,
            Self::Metal => Some(FootfallSound::new("footfall_metal".to_owned())),
            Self::Grate => Some(FootfallSound::new("footfall_grate".to_owned())),
        }
    }

    /// Inverse of [`Self::footfall`].
    #[must_use]
    pub fn from_sound(sound: Option<&FootfallSound>) -> Self {
        Self::ALL
            .into_iter()
            .find(|choice| choice.footfall().as_ref() == sound)
            .unwrap_or(Self::None)
    }
}
