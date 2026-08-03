use bevy::prelude::*;
use gdtf_battle_sim::{level::GridSize, metric::Level};

#[derive(Resource, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CurrentEditLevel(Level);

impl CurrentEditLevel {
            #[must_use]
    pub const fn ground() -> Self {
        Self(Level::new(0))
    }

                #[must_use]
    pub fn stepped(self, delta: LevelStep, size: GridSize) -> Self {
        let current = i32::from(*self.0);
        let max = i32::from(*size.levels()).saturating_sub(1);
        let next = (current + delta.delta()).clamp(0, max);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "next is clamped to [0, levels-1] with levels <= MAX_LEVELS (u8), so it fits \
                      a u8 without wrap or sign-flip"
        )]
        let storey = next as u8;
        Self(Level::new(storey))
    }

                #[must_use]
    pub fn clamped(self, size: GridSize) -> Self {
        self.stepped(LevelStep::none(), size)
    }

                        #[must_use]
    pub fn jumped(target: Level, size: GridSize) -> Self {
        Self(target).clamped(size)
    }

        #[must_use]
    pub const fn level(self) -> Level {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LevelStep(i32);

impl LevelStep {
        #[must_use]
    pub const fn up() -> Self {
        Self(1)
    }

        #[must_use]
    pub const fn down() -> Self {
        Self(-1)
    }

        #[must_use]
    pub const fn none() -> Self {
        Self(0)
    }

        const fn delta(self) -> i32 {
        self.0
    }
}
