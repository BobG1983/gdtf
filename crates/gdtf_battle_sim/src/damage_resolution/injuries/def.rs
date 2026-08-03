//! The authored per-injury record [`InjuryDef`] and its parsed-but-unread
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::{InjuryEffect, InjuryName, InspectText, LogText, PopupText};
use crate::{armor::InjuryCategory, severity::Severity};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Deserialize, Serialize)]
pub enum PostHeal {
            #[default]
    Deferred,
}

impl PostHeal {
        /// `#[serde(default = "PostHeal::deferred")]` source so an authored injury may
        #[must_use]
    pub const fn deferred() -> Self {
        Self::Deferred
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub struct InjuryDef {
            pub name:         InjuryName,
            pub category:     InjuryCategory,
        pub severity:     Severity,
        pub popup_text:   PopupText,
        pub log_text:     LogText,
        pub inspect_text: InspectText,
        pub effects:      Vec<InjuryEffect>,
                #[serde(default = "PostHeal::deferred")]
    pub post_heal:    PostHeal,
}
