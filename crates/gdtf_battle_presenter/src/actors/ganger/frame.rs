use gdtf_battle_sim::{
    ganger::Facing,
    prelude::{Direction, Faction},
};

use super::roles::CharacterRoles;

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FacingFrame(usize);

impl FacingFrame {
        pub const LEFT: Self = Self(0);
        pub const DOWN: Self = Self(1);
        pub const UP: Self = Self(2);
        pub const RIGHT: Self = Self(3);
}

#[must_use]
pub const fn facing_frame(direction: Direction) -> FacingFrame {
    match direction {
        Direction::North | Direction::NorthEast | Direction::NorthWest => FacingFrame::UP,
        Direction::East => FacingFrame::RIGHT,
        Direction::SouthEast | Direction::South | Direction::SouthWest => FacingFrame::DOWN,
        Direction::West => FacingFrame::LEFT,
    }
}

#[must_use]
pub(super) fn atlas_index(roles: &CharacterRoles, faction: Faction, facing: Facing) -> usize {
    *roles.base_for(faction) + *facing_frame(*facing)
}
