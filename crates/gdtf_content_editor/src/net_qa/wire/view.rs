//! What the canvas draws: which storeys, how deep the onion goes, and where the camera sits.

use bevy::prelude::Deref;
use gdtf_battle_presenter::{IsolateView, ViewMode};
use serde::{Deserialize, Serialize};

use super::camera::{EditorPanNet, EditorZoomNet};

/// Whether the canvas draws down to the active storey or every storey.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorViewModeNet {
    /// Hide storeys above the active level.
    DownToActive,
    /// Show every storey with context treatment.
    FullView,
}

impl EditorViewModeNet {
    /// Mirror the presenter's own view mode, with no wildcard arm.
    pub(in crate::net_qa) const fn from_mode(mode: ViewMode) -> Self {
        match mode {
            ViewMode::DownToActive => Self::DownToActive,
            ViewMode::FullView => Self::FullView,
        }
    }
}

/// How many storeys below the active one still draw as context.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorContextDepthNet(u8);

impl EditorContextDepthNet {
    /// Wrap a storey count.
    #[must_use]
    pub(in crate::net_qa) const fn new(storeys: u8) -> Self {
        Self(storeys)
    }
}

/// Whether the canvas isolates the active storey, and how deep the onion goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorIsolateViewNet {
    /// No isolation; the view mode alone decides.
    Off,
    /// Only the active storey plus this many below stay visible.
    On(EditorContextDepthNet),
}

impl EditorIsolateViewNet {
    /// Mirror the presenter's own isolation, with no wildcard arm.
    #[must_use]
    pub(in crate::net_qa) fn from_isolate(isolate: IsolateView) -> Self {
        match isolate {
            IsolateView::Off => Self::Off,
            IsolateView::On(depth) => Self::On(EditorContextDepthNet::new(*depth)),
        }
    }
}

/// Everything that decides what the canvas draws and from where.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorViewNet {
    /// Down to the active storey, or every storey.
    mode:    EditorViewModeNet,
    /// Onion isolation around the active storey.
    isolate: EditorIsolateViewNet,
    /// Canvas zoom scale.
    zoom:    EditorZoomNet,
    /// Preview camera pan offset.
    pan:     EditorPanNet,
}

impl EditorViewNet {
    /// Gather the four values the canvas draws from.
    #[must_use]
    pub(in crate::net_qa) const fn new(
        mode: EditorViewModeNet,
        isolate: EditorIsolateViewNet,
        zoom: EditorZoomNet,
        pan: EditorPanNet,
    ) -> Self {
        Self {
            mode,
            isolate,
            zoom,
            pan,
        }
    }
}
