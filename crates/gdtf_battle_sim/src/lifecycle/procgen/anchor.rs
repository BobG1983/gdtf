//! Edge and corner anchors for deployment zones.

use crate::rng::ProcgenRng;

/// Board edge or corner used to place a deployment zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Anchor {
    /// Top-right corner.
    TopRight,
    /// Top-left corner.
    TopLeft,
    /// Bottom-right corner.
    BottomRight,
    /// Bottom-left corner.
    BottomLeft,
    /// Right edge, vertically centred.
    RightMiddle,
    /// Left edge, vertically centred.
    LeftMiddle,
    /// Top edge, horizontally centred.
    TopMiddle,
    /// Bottom edge, horizontally centred.
    BottomMiddle,
}

impl Anchor {
    /// Anchors valid for the player side.
    pub const PLAYER_ANCHORS: [Self; 4] = [
        Self::TopRight,
        Self::BottomRight,
        Self::RightMiddle,
        Self::BottomMiddle,
    ];

    /// Opposite side of the board.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::TopRight => Self::BottomLeft,
            Self::BottomLeft => Self::TopRight,
            Self::BottomRight => Self::TopLeft,
            Self::TopLeft => Self::BottomRight,
            Self::RightMiddle => Self::LeftMiddle,
            Self::LeftMiddle => Self::RightMiddle,
            Self::BottomMiddle => Self::TopMiddle,
            Self::TopMiddle => Self::BottomMiddle,
        }
    }

    /// Pick a random player anchor.
    #[must_use]
    pub fn choose(rng: &mut ProcgenRng) -> Self {
        let index: usize = rng.random_range(0..Self::PLAYER_ANCHORS.len());
        Self::PLAYER_ANCHORS
            .get(index)
            .copied()
            .unwrap_or(Self::TopRight)
    }
}
