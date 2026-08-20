//! What the screen can show about a ganger when a combat log line is built.

use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    prelude::{Entity, Query, Res},
};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::{Faction, GangerName, Position},
    prelude::CellLevel,
    visibility::{ActObserved, ActVisibility, ActorIdentified, classify_act},
};

use super::event::LogName;
use crate::{actors::fog::ShownSquadVisibility, playback::DrawnPosition};

/// One ganger as the panel sees it: its name, the cell it is drawn on, and its gang.
#[derive(QueryData)]
pub struct PanelRow {
    name:    Option<&'static GangerName>,
    live:    Option<&'static Position>,
    drawn:   Option<&'static DrawnPosition>,
    faction: Option<&'static Faction>,
}

/// The drawn cells and the shown fog a combat log line is decided from.
/// The fog is the playback-gated shadow, not live squad visibility.
#[derive(SystemParam)]
pub struct PanelSight<'w, 's> {
    gangers: Query<'w, 's, PanelRow>,
    fog:     Option<Res<'w, ShownSquadVisibility>>,
    player:  Option<Res<'w, PlayerFaction>>,
}

impl PanelSight<'_, '_> {
    /// Whether the screen is lighting this cell.
    #[must_use]
    pub fn shows(&self, at: CellLevel) -> ActObserved {
        ActObserved::new(
            self.fog
                .as_deref()
                .is_some_and(|fog| *fog.visibility().is_cell_visible(&at)),
        )
    }

    /// The cell a ganger's sprite is drawn on, falling back to the sim cell.
    #[must_use]
    pub fn cell_of(&self, who: Entity) -> Option<CellLevel> {
        let row = self.gangers.get(who).ok()?;
        row.drawn.map_or_else(
            || row.live.map(|live| **live),
            |drawn| Some(*drawn.position()),
        )
    }

    /// Whether the screen can put a name to this ganger.
    /// Own-squad gangers, and any the screen cannot place, are always named.
    #[must_use]
    pub fn identifies(&self, who: Entity) -> ActorIdentified {
        ActorIdentified::new(self.can_see(who))
    }

    /// Whether the screen shows what happens where this ganger stands.
    #[must_use]
    pub fn shows_where(&self, who: Entity) -> ActObserved {
        ActObserved::new(self.can_see(who))
    }

    /// What the screen may say about an act by this ganger, on its own cell.
    #[must_use]
    pub fn about(&self, who: Entity) -> ActVisibility {
        classify_act(self.shows_where(who), self.identifies(who))
    }

    /// The name to print, or `Someone` when the screen cannot identify the ganger.
    #[must_use]
    pub fn name_of(&self, who: Entity) -> LogName {
        if !self.can_see(who) {
            return LogName::new("Someone");
        }
        self.gangers
            .get(who)
            .ok()
            .and_then(|row| row.name.map(LogName::from_ganger))
            .unwrap_or_else(|| LogName::new("Someone"))
    }

    /// The name for an act by this ganger, or nothing when the screen never saw it.
    #[must_use]
    pub fn named(&self, who: Entity) -> Option<LogName> {
        match self.about(who) {
            ActVisibility::Withheld => None,
            ActVisibility::Named | ActVisibility::Unnamed => Some(self.name_of(who)),
        }
    }

    // Own squad and anything the screen cannot place are visible; the rest need a lit cell.
    fn can_see(&self, who: Entity) -> bool {
        let Ok(row) = self.gangers.get(who) else {
            return true;
        };
        let player = self.player.as_deref().map(|player| **player);
        if row.faction.is_some() && row.faction.copied() == player {
            return true;
        }
        match self.cell_of(who) {
            Some(at) => *self.shows(at),
            None => true,
        }
    }
}
