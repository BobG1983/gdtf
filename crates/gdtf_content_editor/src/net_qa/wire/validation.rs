//! How far the content integrity pass has got, and what it found, on the wire.

use bevy::prelude::Deref;
use gdtf_assets::ContentFinding;
use serde::{Deserialize, Serialize};

/// Whether every reference check has run this load.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct ChecksCompleteNet(bool);

impl ChecksCompleteNet {
    /// Mirror whether the checks-complete marker is in the world.
    #[must_use]
    pub(in crate::net_qa) const fn new(complete: bool) -> Self {
        Self(complete)
    }
}

/// Whether the integrity report has been published.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct ValidationPublishedNet(bool);

impl ValidationPublishedNet {
    /// Mirror whether the published marker is in the world.
    #[must_use]
    pub(in crate::net_qa) const fn new(published: bool) -> Self {
        Self(published)
    }
}

/// One integrity finding, rendered the way the editor's own log renders it.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct ValidationFindingNet(String);

impl ValidationFindingNet {
    /// Render a finding through its own `Display`, so client and log agree.
    #[must_use]
    pub(in crate::net_qa) fn from_finding(finding: &ContentFinding) -> Self {
        Self(finding.to_string())
    }
}
