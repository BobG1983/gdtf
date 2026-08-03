use crate::rng::ProcgenRng;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Anchor {
        TopRight,
        TopLeft,
        BottomRight,
        BottomLeft,
        RightMiddle,
        LeftMiddle,
        TopMiddle,
        BottomMiddle,
}

impl Anchor {
                                pub const PLAYER_ANCHORS: [Self; 4] = [
        Self::TopRight,
        Self::BottomRight,
        Self::RightMiddle,
        Self::BottomMiddle,
    ];

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

                                #[must_use]
    pub fn choose(rng: &mut ProcgenRng) -> Self {
        let index: usize = rng.random_range(0..Self::PLAYER_ANCHORS.len());
        Self::PLAYER_ANCHORS
            .get(index)
            .copied()
            .unwrap_or(Self::TopRight)
    }
}
