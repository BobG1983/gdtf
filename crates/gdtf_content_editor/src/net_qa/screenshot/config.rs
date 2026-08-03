use bevy::{camera::ImageRenderTarget, image::Image, prelude::*};
use gdtf_screenshot::{PollCap, SettleFrames};

#[derive(Resource, Clone, Copy, Debug, Deref)]
pub struct EditorShotSettle(SettleFrames);

impl EditorShotSettle {
            #[must_use]
    pub const fn new(frames: SettleFrames) -> Self {
        Self(frames)
    }
}

impl Default for EditorShotSettle {
        fn default() -> Self {
        Self(SettleFrames::DEFAULT_EGUI)
    }
}

#[derive(Resource, Clone, Copy, Debug, Deref)]
pub struct EditorShotPollBudget(PollCap);

impl EditorShotPollBudget {
            #[must_use]
    pub const fn new(frames: PollCap) -> Self {
        Self(frames)
    }
}

impl Default for EditorShotPollBudget {
        fn default() -> Self {
        Self(PollCap::DEFAULT)
    }
}

#[derive(Resource, Clone, Debug)]
pub enum EditorShotSource {
                                                                            PrimaryWindow,
                                                            Offscreen(ImageRenderTarget),
}

impl Default for EditorShotSource {
                /// The handle is a PLACEHOLDER — `#[derive(Default)]` cannot pick a variant that carries
                                                                    fn default() -> Self {
        Self::Offscreen(ImageRenderTarget::from(Handle::<Image>::default()))
    }
}
