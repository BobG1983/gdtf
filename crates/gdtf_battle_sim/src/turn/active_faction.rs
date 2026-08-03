use bevy::prelude::{Deref, Resource};

use crate::ganger::Faction;

const TEAM_COUNT: u8 = 2;

#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActiveFaction(Faction);

impl ActiveFaction {
                        #[must_use]
    pub const fn new(faction: Faction) -> Self {
        Self(faction)
    }

                                pub fn advance(&mut self) {
        let next = (*self.0).wrapping_add(1) % TEAM_COUNT;
        self.0 = Faction::new(next);
    }
}
