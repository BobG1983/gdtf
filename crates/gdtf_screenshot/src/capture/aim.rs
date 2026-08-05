//! Refuse a capture of an image nothing renders into.

use bevy::{camera::RenderTarget, ecs::system::SystemParam, prelude::*};

use super::source::{CaptureSource, aims_at};
use crate::present::QaCaptureTarget;

/// Why a capture was refused before it spawned.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CaptureAimDetail(String);

impl CaptureAimDetail {
    /// Wrap a refusal message.
    #[must_use]
    pub const fn new(detail: String) -> Self {
        Self(detail)
    }

    /// Borrow the message.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether something renders into the image a capture would read.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CaptureAim {
    /// The capture may spawn.
    Confirmed,
    /// Nothing draws into the image, so the PNG would be blank.
    Refused(CaptureAimDetail),
}

#[derive(SystemParam)]
pub(super) struct CaptureAimCheck<'w, 's> {
    target:  Option<Res<'w, QaCaptureTarget>>,
    cameras: Query<'w, 's, &'static RenderTarget, With<Camera>>,
}

impl CaptureAimCheck<'_, '_> {
    pub(super) fn verify(&self, source: &CaptureSource) -> CaptureAim {
        let CaptureSource::Offscreen(wanted) = source else {
            return CaptureAim::Confirmed;
        };
        let Some(target) = self.target.as_ref() else {
            return CaptureAim::Confirmed;
        };
        if *wanted != ***target {
            return CaptureAim::Confirmed;
        }
        if self.cameras.iter().any(|current| aims_at(current, wanted)) {
            return CaptureAim::Confirmed;
        }
        CaptureAim::Refused(self.mismatch(wanted))
    }

    fn mismatch(&self, wanted: &bevy::camera::ImageRenderTarget) -> CaptureAimDetail {
        let found: Vec<String> = self
            .cameras
            .iter()
            .map(|current| format!("{current:?}"))
            .collect();
        let rendered = if found.is_empty() {
            "nothing — this app has no camera".to_owned()
        } else {
            found.join(", ")
        };
        CaptureAimDetail::new(format!(
            "the capture would read {wanted:?}, but the cameras in this app render into \
             {rendered}. Nothing draws into that image, so the PNG would be a blank frame.",
        ))
    }
}
