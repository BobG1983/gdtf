//! Frame countdown shared by the settle and poll stages.

use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Deref)]
pub(super) struct FramesLeft(u32);

pub(super) enum FrameTick {
    Live,
    Expired,
}

impl FramesLeft {
    pub(super) const fn new(frames: u32) -> Self {
        Self(frames)
    }

    pub(super) const fn tick(&mut self) -> FrameTick {
        if self.0 == 0 {
            return FrameTick::Expired;
        }
        self.0 -= 1;
        FrameTick::Live
    }
}
