//! The canvas camera's zoom scale and pan offset on the wire.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use crate::preview::view::PreviewPan;

/// Multiplicative zoom scale of the map canvas.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorZoomNet(f32);

impl EditorZoomNet {
    /// Wrap a zoom scale.
    #[must_use]
    pub(in crate::net_qa) const fn new(scale: f32) -> Self {
        Self(scale)
    }
}

/// Pan offset along X, in world units.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorPanXNet(f32);

impl EditorPanXNet {
    /// Wrap an X offset.
    #[must_use]
    pub(in crate::net_qa) const fn new(x: f32) -> Self {
        Self(x)
    }
}

/// Pan offset along Y, in world units.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorPanYNet(f32);

impl EditorPanYNet {
    /// Wrap a Y offset.
    #[must_use]
    pub(in crate::net_qa) const fn new(y: f32) -> Self {
        Self(y)
    }
}

/// Where the preview camera sits, as a world-space offset.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorPanNet {
    /// Offset along X.
    x: EditorPanXNet,
    /// Offset along Y.
    y: EditorPanYNet,
}

impl EditorPanNet {
    /// Mirror the preview pan's own offset.
    #[must_use]
    pub(in crate::net_qa) const fn from_pan(pan: PreviewPan) -> Self {
        Self {
            x: EditorPanXNet::new(pan.offset().x),
            y: EditorPanYNet::new(pan.offset().y),
        }
    }
}
