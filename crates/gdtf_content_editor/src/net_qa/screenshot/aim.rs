use bevy::{camera::RenderTarget, ecs::system::SystemParam, prelude::*};
use bevy_egui::PrimaryEguiContext;

use super::config::EditorShotSource;
use crate::net_qa::present::{EditorQaCaptureTarget, aims_at};

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::net_qa) struct CaptureAimDetail(String);

impl CaptureAimDetail {
    pub(in crate::net_qa) const fn new(detail: String) -> Self {
        Self(detail)
    }

    pub(in crate::net_qa) fn as_str(&self) -> &str {
        &self.0
    }
}

pub(in crate::net_qa) enum CaptureAim {
    Confirmed,
    Refused(CaptureAimDetail),
}

#[derive(SystemParam)]
pub(in crate::net_qa) struct EditorCaptureAim<'w, 's> {
    target:  Option<Res<'w, EditorQaCaptureTarget>>,
    cameras: Query<'w, 's, &'static RenderTarget, With<PrimaryEguiContext>>,
}

impl EditorCaptureAim<'_, '_> {
    pub(in crate::net_qa) fn verify(&self, source: &EditorShotSource) -> CaptureAim {
        let EditorShotSource::Offscreen(wanted) = source else {
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
        let found: Vec<String> = self
            .cameras
            .iter()
            .map(|current| format!("{current:?}"))
            .collect();
        CaptureAim::Refused(CaptureAimDetail::new(format!(
            "the capture would read the offscreen target {wanted:?}, but the camera holding the \
             editor's primary egui context renders into {}. Nothing draws into that image, so \
             the PNG would be a blank frame.",
            if found.is_empty() {
                "nothing — no camera holds that context".to_owned()
            } else {
                found.join(", ")
            }
        )))
    }
}
