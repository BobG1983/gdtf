//! Injury form draft resource.

use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{
        InjuryDef, InjuryEffect, InjuryName, InspectText, LogText, PopupText, PostHeal, StatDelta,
        StatTarget,
    },
    severity::Severity,
};

/// Default effect used by a new injury draft.
pub(crate) const DEFAULT_EFFECT: InjuryEffect = InjuryEffect::Modify {
    stat:   StatTarget::Speed,
    amount: StatDelta::new(-1),
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum AutoloadState {
    Pending,
    Done,
}

fn seed_def() -> InjuryDef {
    InjuryDef {
        name:         InjuryName::new(String::new()),
        category:     InjuryCategory::Head,
        severity:     Severity::Minor,
        popup_text:   PopupText::new(String::new()),
        log_text:     LogText::new(String::new()),
        inspect_text: InspectText::new(String::new()),
        effects:      vec![DEFAULT_EFFECT],
        post_heal:    PostHeal::Deferred,
    }
}

/// In-progress injury being authored.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct InjuryDraft {
    key:      String,
    def:      InjuryDef,
    autoload: AutoloadState,
}

impl InjuryDraft {
    /// Empty draft ready for a new injury.
    #[must_use]
    pub fn new_injury() -> Self {
        Self {
            key:      String::new(),
            def:      seed_def(),
            autoload: AutoloadState::Done,
        }
    }

    /// Whether the form should still try to autoload from the registry.
    #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

    /// Mark autoload complete.
    pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

    /// Load an existing injury into the draft.
    pub fn load_injury(&mut self, key: &InjuryName, def: &InjuryDef) {
        key.as_str().clone_into(&mut self.key);
        self.def = def.clone();
        self.autoload = AutoloadState::Done;
    }

    /// Injury key / file stem.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Set the injury key.
    pub fn set_key(&mut self, key: String) {
        self.key = key;
    }

    /// Full injury def.
    #[must_use]
    pub const fn def(&self) -> &InjuryDef {
        &self.def
    }

    /// Mutable access to the injury def.
    #[must_use]
    pub const fn def_mut(&mut self) -> &mut InjuryDef {
        &mut self.def
    }
}

impl Default for InjuryDraft {
    fn default() -> Self {
        Self {
            key:      String::new(),
            def:      seed_def(),
            autoload: AutoloadState::Pending,
        }
    }
}
