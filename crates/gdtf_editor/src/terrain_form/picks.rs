//! Terrain kind, footfall and view-row UI choices.

use bevy::prelude::Component;
use gdtf_battle_sim::terrain::{
    def::{OwedViews, owed_views_for},
    entity::TerrainPieceKind,
    piece::FootfallSound,
};

use super::draft::TerrainDraft;

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

    /// The sim piece kind this pick stands for.
    #[must_use]
    pub const fn piece_kind(self) -> TerrainPieceKind {
        match self {
            Self::Wall => TerrainPieceKind::Wall,
            Self::Cover => TerrainPieceKind::Cover,
            Self::Slab => TerrainPieceKind::Slab,
            Self::Emplacement => TerrainPieceKind::Emplacement,
        }
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

/// The views the open draft owes art for, one row each in the form's picker.
/// Read off the draft's own kind and tags, never off a projected def.
#[must_use]
pub fn view_rows(draft: &TerrainDraft) -> OwedViews {
    owed_views_for(draft.kind().piece_kind(), draft.tags())
}

/// Whether the draft owes more than one view, which is when the rows get an expander.
#[must_use]
pub fn offers_view_expander(draft: &TerrainDraft) -> bool {
    view_rows(draft).len() > 1
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
