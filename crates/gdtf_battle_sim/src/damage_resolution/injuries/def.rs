//! Authored injury definition loaded from content.

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::{InjuryEffect, InjuryName, InspectText, LogText, PopupText};
use crate::{armor::InjuryCategory, severity::Severity};

/// What happens after this injury is healed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Deserialize, Serialize)]
pub enum PostHeal {
    /// Effects stay until something else clears them.
    #[default]
    Deferred,
}

impl PostHeal {
    /// Default used by serde when the field is omitted.
    #[must_use]
    pub const fn deferred() -> Self {
        Self::Deferred
    }
}

/// One authored injury record.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub struct InjuryDef {
    /// Stable name key.
    pub name:         InjuryName,
    /// Body-part category this injury belongs to.
    pub category:     InjuryCategory,
    /// Severity tier.
    pub severity:     Severity,
    /// Short text for UI popups.
    pub popup_text:   PopupText,
    /// Text for the act log.
    pub log_text:     LogText,
    /// Longer inspect text.
    pub inspect_text: InspectText,
    /// Effects applied when the injury is gained.
    pub effects:      Vec<InjuryEffect>,
    /// Behavior after healing.
    #[serde(default = "PostHeal::deferred")]
    pub post_heal:    PostHeal,
}
