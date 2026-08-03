use crate::{ganger::Tu, metric::CellLevel};

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct PathCost(u32);

impl PathCost {
        pub const ZERO: Self = Self(0);

                    #[must_use]
    pub const fn new(cost: u32) -> Self {
        Self(cost)
    }

                            #[must_use]
    pub fn add_step(self, step: Tu) -> Self {
        Self(self.0 + u32::from(*step))
    }

                                    #[must_use]
    pub const fn to_tu(self) -> Tu {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "saturated to u8::MAX above before the cast, so it cannot truncate or wrap"
        )]
        let narrowed = if self.0 > u8::MAX as u32 {
            u8::MAX
        } else {
            self.0 as u8
        };
        Tu::new(narrowed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
        cells: Vec<CellLevel>,
                steps: Vec<Tu>,
        total: Tu,
}

impl Path {
                                    #[must_use]
    pub const fn new(cells: Vec<CellLevel>, steps: Vec<Tu>, total: Tu) -> Self {
        Self {
            cells,
            steps,
            total,
        }
    }

        #[must_use]
    pub fn cells(&self) -> &[CellLevel] {
        &self.cells
    }

                                    #[must_use]
    pub fn steps(&self) -> &[Tu] {
        &self.steps
    }

            #[must_use]
    pub const fn total(&self) -> Tu {
        self.total
    }

            #[must_use]
    pub fn start(&self) -> Option<CellLevel> {
        self.cells.first().copied()
    }

        #[must_use]
    pub fn goal(&self) -> Option<CellLevel> {
        self.cells.last().copied()
    }

            #[must_use]
    pub const fn len(&self) -> usize {
        self.cells.len()
    }

                #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathBlocked;
