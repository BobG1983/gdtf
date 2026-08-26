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
pub const DEFAULT_EFFECT: InjuryEffect = InjuryEffect::Modify {
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

    /// Authored effects.
    #[must_use]
    pub fn effects(&self) -> &[InjuryEffect] {
        &self.def.effects
    }

    /// Whether an effect may be removed. An injury keeps at least one.
    #[must_use]
    pub const fn can_remove_effect(&self) -> bool {
        self.def.effects.len() > 1
    }

    /// Append the form's default effect.
    pub fn add_effect(&mut self) {
        self.def.effects.push(DEFAULT_EFFECT);
    }

    /// Remove an effect by index. Returns whether one was removed.
    pub fn remove_effect(&mut self, index: usize) -> bool {
        if !self.can_remove_effect() || index >= self.def.effects.len() {
            return false;
        }
        self.def.effects.remove(index);
        true
    }

    /// Rewrite an effect by index. Returns whether one was written.
    pub fn set_effect(&mut self, index: usize, effect: InjuryEffect) -> bool {
        match self.def.effects.get_mut(index) {
            Some(slot) => {
                *slot = effect;
                true
            }
            None => false,
        }
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
