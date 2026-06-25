//! The 8→4 facing-frame map and the structural atlas-index sum.

use gdtf_battle_sim::{Direction, Facing, Faction};

use super::roles::CharacterRoles;

/// The 0..=3 within-actor column offset selecting which of the sheet's 4 frames a
/// facing draws.
///
/// A named newtype over `usize` (no-bare-types: a frame offset is a domain value, not a
/// bare `usize`), [`Deref`](std::ops::Deref)ing to it. Added to a faction's base
/// [`TileIndex`](crate::TileIndex) to form the drawn atlas index
/// (`faction_base + facing_frame`). The four frames are the sheet's per-actor order:
/// `0` LEFT (W), `1` DOWN (S), `2` UP (N), `3` RIGHT (E).
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FacingFrame(usize);

impl FacingFrame {
    /// The `col+0` LEFT (W) frame.
    pub const LEFT: Self = Self(0);
    /// The `col+1` DOWN (S) frame.
    pub const DOWN: Self = Self(1);
    /// The `col+2` UP (N) frame.
    pub const UP: Self = Self(2);
    /// The `col+3` RIGHT (E) frame.
    pub const RIGHT: Self = Self(3);
}

/// The pure 8->4 facing-frame map: which of the sheet's 4 frames a [`Direction`] draws.
///
/// The sheet ships 4 frames per actor, not 8, so the 8 sim facings collapse to 4 sprite
/// frames by nearest cardinal with a vertical bias (keeping the face legible). The map
/// is EXACTLY (the S5 contract):
///
/// - [`North`](Direction::North) -> [`UP`](FacingFrame::UP),
///   [`NorthEast`](Direction::NorthEast) -> [`UP`](FacingFrame::UP),
///   [`NorthWest`](Direction::NorthWest) -> [`UP`](FacingFrame::UP)
/// - [`East`](Direction::East) -> [`RIGHT`](FacingFrame::RIGHT)
/// - [`SouthEast`](Direction::SouthEast) -> [`DOWN`](FacingFrame::DOWN),
///   [`South`](Direction::South) -> [`DOWN`](FacingFrame::DOWN),
///   [`SouthWest`](Direction::SouthWest) -> [`DOWN`](FacingFrame::DOWN)
/// - [`West`](Direction::West) -> [`LEFT`](FacingFrame::LEFT)
///
/// A pure `const fn` (NOT data-driven): it is a fixed property of the 4-frame sheet, and
/// encoding it in code keeps it exhaustively unit-testable.
#[must_use]
pub const fn facing_frame(direction: Direction) -> FacingFrame {
    match direction {
        Direction::North | Direction::NorthEast | Direction::NorthWest => FacingFrame::UP,
        Direction::East => FacingFrame::RIGHT,
        Direction::SouthEast | Direction::South | Direction::SouthWest => FacingFrame::DOWN,
        Direction::West => FacingFrame::LEFT,
    }
}

/// The drawn atlas index for a ganger: its faction's base actor tile plus its facing
/// frame.
///
/// `faction_base + facing_frame` — the base read STRUCTURALLY from [`CharacterRoles`]
/// (the data table) and the offset from the [`facing_frame`] map, never a hardcoded
/// literal. Returned as a flat `usize` for [`TextureAtlas::index`](bevy::prelude::TextureAtlas).
#[must_use]
pub(super) fn atlas_index(roles: &CharacterRoles, faction: Faction, facing: Facing) -> usize {
    *roles.base_for(faction) + *facing_frame(*facing)
}
