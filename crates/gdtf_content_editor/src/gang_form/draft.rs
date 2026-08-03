//! GTW-505 `melee_weapon` key, so loading and re-saving a gang that authored one can
use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::ArmorName,
    ganger::{GangMember, GangName, GangRoster, GangerName},
    weapon::WeaponName,
};

const DEFAULT_MEMBER_NAME: &str = "New Member";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
        Pending,
        Done,
}

#[derive(Resource, Clone, PartialEq, Debug)]
pub struct GangDraft {
        name:     String,
        members:  Vec<GangMember>,
        autoload: AutoloadState,
}

impl GangDraft {
                #[must_use]
    pub const fn new_gang() -> Self {
        Self {
            name:     String::new(),
            members:  Vec::new(),
            autoload: AutoloadState::Done,
        }
    }

                #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

            pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

                pub fn load_gang(&mut self, name: &GangName, roster: &GangRoster) {
        name.as_str().clone_into(&mut self.name);
        self.members.clone_from(&roster.members);
        self.autoload = AutoloadState::Done;
    }

        #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

        pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

        #[must_use]
    pub fn members(&self) -> &[GangMember] {
        &self.members
    }

                    #[must_use]
    pub fn members_mut(&mut self) -> &mut [GangMember] {
        &mut self.members
    }

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
            armor:        ArmorName::new(String::new()),
            weapon:       WeaponName::new(String::new()),
            melee_weapon: None,
        });
    }

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
