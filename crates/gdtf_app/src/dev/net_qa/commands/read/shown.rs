//! The playback-gated battle view a QA read sees, and the card it builds from it.

use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    prelude::*,
};
use gdtf_battle_presenter::{DrawnPosition, DrawnVitals, ShownSquadVisibility};
use gdtf_battle_sim::{battle::PlayerFaction, ganger::Position, prelude::CellLevel};

use crate::{
    dev::net_qa::wire::{
        act_payload::StanceNet,
        roster::{FactionNet, GangerCardNet, GangerNameNet},
        token::GangerToken,
        vitals::{HpMaxNet, HpNet, TuMaxNet, TuNet, WoundsMaxNet, WoundsNet},
        wound::{InjuryNet, WoundNet},
    },
    states::running::game::battlescape::{
        inspect_panel::{
            decide::ShownBattle,
            shadow::{ShownCoverLedger, ShownOccupancyGrid},
        },
        stat_block::StatBlockDataItem,
    },
};

/// The shadow resources the inspect panel reads, as one system param.
#[derive(SystemParam)]
pub(in crate::dev::net_qa) struct ShownBattleReads<'w> {
    grid:   Option<Res<'w, ShownOccupancyGrid>>,
    ledger: Option<Res<'w, ShownCoverLedger>>,
    squad:  Option<Res<'w, ShownSquadVisibility>>,
    player: Option<Res<'w, PlayerFaction>>,
}

impl ShownBattleReads<'_> {
    /// Borrow the shadows as the view the inspect panel decides from.
    #[must_use]
    pub(in crate::dev::net_qa) fn shown(&self) -> ShownBattle<'_> {
        ShownBattle::new(
            self.grid.as_deref().map(ShownOccupancyGrid::grid),
            self.ledger.as_deref().map(ShownCoverLedger::ledger),
            self.squad.as_deref().map(ShownSquadVisibility::visibility),
            self.player.as_deref(),
        )
    }
}

/// Where a ganger stands on screen: its drawn cell, which lags the sim while an act plays out.
#[derive(QueryData)]
pub(in crate::dev::net_qa) struct DrawnCell {
    live:  &'static Position,
    drawn: Option<&'static DrawnPosition>,
}

impl DrawnCellItem<'_, '_> {
    /// The cell the sprite is drawn on, falling back to the sim cell before playback seeds one.
    #[must_use]
    pub(in crate::dev::net_qa) fn at(&self) -> CellLevel {
        self.drawn.map_or(**self.live, |drawn| *drawn.position())
    }
}

/// Build a roster card from the row the stat block draws, reporting drawn vitals.
#[must_use]
pub(in crate::dev::net_qa) fn ganger_card(
    entity: Entity,
    row: &StatBlockDataItem<'_, '_>,
) -> GangerCardNet {
    let tu = row.drawn.map_or(*row.tu, DrawnVitals::tu);
    let hp = row.drawn.map_or(*row.hp, DrawnVitals::hp);
    let wounds = row.drawn.map_or(*row.wounds, DrawnVitals::wounds);
    let taken = row
        .drawn
        .map_or(row.inflicted, |drawn| Some(drawn.inflicted()));
    let injuries = row
        .drawn
        .map_or(row.injuries, |drawn| Some(drawn.injuries()));

    GangerCardNet {
        token:        GangerToken::new(entity.to_bits()),
        name:         row.name.map(|name| GangerNameNet::new((**name).clone())),
        faction:      FactionNet::from_sim(*row.faction),
        stance:       StanceNet::from_sim(*row.stance),
        tu:           TuNet::new(*tu),
        tu_max:       TuMaxNet::new(**row.tu_max),
        hp:           HpNet::new(*hp),
        hp_max:       HpMaxNet::new(row.hp_max.map_or(*hp, |max| **max)),
        wounds:       WoundsNet::new(*wounds),
        wounds_max:   WoundsMaxNet::new(row.wounds_max.map_or(*wounds, |max| **max)),
        wounds_taken: taken.map_or_else(Vec::new, |list| {
            list.iter().copied().map(WoundNet::from_sim).collect()
        }),
        injuries:     injuries.map_or_else(Vec::new, |list| {
            list.gained().iter().map(InjuryNet::from_sim).collect()
        }),
    }
}
