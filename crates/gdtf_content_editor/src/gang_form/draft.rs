//! Gang roster form draft resource.

use bevy::prelude::*;
use gdtf_battle_sim::ganger::{GangMember, GangName, GangRoster, GangerName};

const DEFAULT_MEMBER_NAME: &str = "New Member";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
    Pending,
    Done,
}

/// In-progress gang roster being authored.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct GangDraft {
    name:     String,
    members:  Vec<GangMember>,
    autoload: AutoloadState,
}

impl GangDraft {
    /// Range a member attribute input offers.
    pub const ATTRIBUTE_RANGE: core::ops::RangeInclusive<f32> = 0.0..=100.0;

    /// Empty draft ready for a new gang.
    #[must_use]
    pub const fn new_gang() -> Self {
        Self {
            name:     String::new(),
            members:  Vec::new(),
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

    /// Load an existing gang into the draft.
    pub fn load_gang(&mut self, name: &GangName, roster: &GangRoster) {
        name.as_str().clone_into(&mut self.name);
        self.members.clone_from(&roster.members);
        self.autoload = AutoloadState::Done;
    }

    /// Display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the display name.
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Member list.
    #[must_use]
    pub fn members(&self) -> &[GangMember] {
        &self.members
    }

    /// Mutable member list.
    #[must_use]
    pub fn members_mut(&mut self) -> &mut [GangMember] {
        &mut self.members
    }

    /// Append a default member.
    pub fn add_member(&mut self) {
        self.members.push(GangMember {
            name:         GangerName::new(DEFAULT_MEMBER_NAME.to_owned()),
            speed:        default(),
            aim:          default(),
            strength:     default(),
            toughness:    default(),
            reflexes:     default(),
            cool:         default(),
            grit:         default(),
            luck:         default(),
            armor:        None,
            weapon:       None,
            melee_weapon: None,
        });
    }

    /// Remove a member by index. Returns whether one was removed.
    pub fn remove_member(&mut self, index: usize) -> bool {
        if index < self.members.len() {
            self.members.remove(index);
            true
        } else {
            false
        }
    }
}

impl Default for GangDraft {
    fn default() -> Self {
        Self {
            name:     String::new(),
            members:  Vec::new(),
            autoload: AutoloadState::Pending,
        }
    }
}
