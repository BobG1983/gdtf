//! Plugin registering the capture pipeline for one payload type.

use core::marker::PhantomData;

use bevy::prelude::*;

use super::{
    dir::{ShotDir, ShotSequence},
    pump::drive_captures,
    queue::{CaptureCompletions, CaptureQueue},
    source::CaptureSource,
};
use crate::settle::{PollCap, SettleFrames};

/// System set holding the capture pump.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CaptureSystems;

/// Registers the queue, completions, tunables and pump for payload `P`.
pub struct CapturePipelinePlugin<P>(PhantomData<fn() -> P>);

impl<P> CapturePipelinePlugin<P> {
    /// Build the pipeline plugin for payload `P`.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<P> Default for CapturePipelinePlugin<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Send + Sync + 'static> Plugin for CapturePipelinePlugin<P> {
    fn build(&self, app: &mut App) {
        app.init_resource::<CaptureQueue<P>>()
            .init_resource::<CaptureCompletions<P>>()
            .init_resource::<ShotDir>()
            .init_resource::<ShotSequence>()
            .init_resource::<SettleFrames>()
            .init_resource::<PollCap>()
            .init_resource::<CaptureSource>();
        app.add_systems(Update, drive_captures::<P>.in_set(CaptureSystems));
    }
}
