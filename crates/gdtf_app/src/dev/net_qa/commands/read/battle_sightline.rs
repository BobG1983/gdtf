use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    prelude::*,
};
use gdtf_battle_input::{SelectedFireMode, SelectedShooter, firing_weapon_of};
use gdtf_battle_presenter::PresenterSystems;
use gdtf_battle_sim::{
    acts::can_engage,
    ganger::{Aiming, Facing, LifeState, Position, TuMax},
    injuries::{HandsAvailable, InflictedInjuries},
    magazine::{FireActor, Magazine, can_fire, mode_tu_cost},
    prelude::{CellLevel, Faction, Tu},
    tuning::CombatTuning,
    weapon::{Handedness, MeleeWeapon, MountedWeapon, WieldedBy, Wields},
};
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use serde::{Deserialize, Serialize};

use super::{availability::battle_is_running, shown::ShownBattleReads};
use crate::{
    dev::net_qa::{
        facts::GameFacts,
        wire::{
            cell::CellLevelNet,
            sight::{CanEngageNet, CanSeeNet, SightlineNet},
            token::GangerToken,
        },
    },
    states::running::game::battlescape::inspect_panel::shadow::{
        promote_shown_cover, promote_shown_occupancy,
    },
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BattleSightlineArgs {
    /// The cell to ask about, as `battle.visible` and `battle.offers` report cells.
    at: CellLevelNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BattleSightlineReply {
    at:        CellLevelNet,
    shooter:   Option<GangerToken>,
    sightline: SightlineNet,
}

pub(crate) struct BattleSightline;

impl QaCommand for BattleSightline {
    type Args = BattleSightlineArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = BattleSightlineReply;

    const NAME: CommandName = CommandName::from_static("battle.sightline");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Ask whether the squad can see a cell and whether the selected shooter could fire at it \
         this turn. Seeing reads the playback-gated fog the screen draws, so it agrees with \
         battle.visible and battle.inspect; engaging is the sim's own firing-arc gate priced \
         with the selected fire mode, and its own fire-readiness gate, which needs the shooter \
         alive with a loaded gun, free hands and the time to shoot. With nothing selected it \
         answers NoShooter.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        battle_is_running(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_battle_sightline
                .after(QaCommandSystems::Claim)
                .after(PresenterSystems::Compose)
                .after(promote_shown_occupancy)
                .after(promote_shown_cover),
        );
    }
}

/// What the sightline answer is read from: the screen's view plus the sim's firing gates.
#[derive(SystemParam)]
pub(super) struct SightlineReads<'w, 's> {
    shown:     ShownBattleReads<'w>,
    selected:  Option<Res<'w, SelectedShooter>>,
    fire_mode: Option<Res<'w, SelectedFireMode>>,
    tuning:    Option<Res<'w, CombatTuning>>,
    factions:  Query<'w, 's, &'static Faction>,
    shooters:  Query<'w, 's, ShooterRow>,
    wields:    Query<'w, 's, &'static Wields>,
    guns:      Query<'w, 's, GunRow, With<WieldedBy>>,
    mounted:   Query<'w, 's, (), With<MountedWeapon>>,
    melee:     Query<'w, 's, (), With<MeleeWeapon>>,
}

/// Pose, pool and body the two firing gates read off the selected shooter.
#[derive(QueryData)]
pub(super) struct ShooterRow {
    position: &'static Position,
    facing:   &'static Facing,
    tu:       &'static Tu,
    tu_max:   &'static TuMax,
    aiming:   &'static Aiming,
    life:     &'static LifeState,
    injuries: Option<&'static InflictedInjuries>,
}

/// What fire readiness reads off the gun the shooter fires.
#[derive(QueryData)]
pub(super) struct GunRow {
    magazine:   &'static Magazine,
    handedness: &'static Handedness,
}

impl SightlineReads<'_, '_> {
    /// The selected shooter, absent when nothing is selected.
    #[must_use]
    pub(super) fn shooter(&self) -> Option<Entity> {
        self.selected.as_deref().and_then(|selected| **selected)
    }

    /// Both halves of the answer, or the refusal when nothing is selected.
    #[must_use]
    pub(super) fn sightline(&self, at: CellLevel) -> SightlineNet {
        match self.shooter() {
            Some(entity) => SightlineNet::Answered {
                can_see:    self.can_see(at),
                can_engage: self.can_engage(entity, at),
            },
            None => SightlineNet::NoShooter,
        }
    }

    fn can_see(&self, at: CellLevel) -> CanSeeNet {
        let shown = self.shown.shown();
        let occupant = shown
            .grid()
            .and_then(|grid| grid.occupant(&at))
            .and_then(|occupant| self.factions.get(occupant).ok().copied());
        let seen = match occupant {
            Some(faction) => shown.ganger_visible(at, faction),
            None => shown.cell_visible(at),
        };
        CanSeeNet::new(seen.is_squad_visible())
    }

    fn can_engage(&self, shooter: Entity, at: CellLevel) -> CanEngageNet {
        let (Some(fire_mode), Some(tuning)) = (self.fire_mode.as_deref(), self.tuning.as_deref())
        else {
            return CanEngageNet::new(false);
        };
        let Ok(row) = self.shooters.get(shooter) else {
            return CanEngageNet::new(false);
        };
        let Some(gun) = firing_weapon_of(shooter, &self.wields, &self.mounted, &self.melee)
            .and_then(|weapon| self.guns.get(weapon).ok())
        else {
            return CanEngageNet::new(false);
        };
        let cost = mode_tu_cost(fire_mode, row.tu_max, row.aiming, tuning);
        let in_arc = can_engage(
            **row.facing,
            row.position.cell(),
            at.cell(),
            *row.tu,
            cost,
            tuning,
        );
        let shot = FireActor {
            life:            row.life,
            tu:              row.tu,
            tu_max:          row.tu_max,
            aiming:          row.aiming,
            magazine:        gun.magazine,
            handedness:      *gun.handedness,
            hands_available: row
                .injuries
                .map_or_else(HandsAvailable::default, InflictedInjuries::hands_available),
        };
        let ready = can_fire(&shot, fire_mode, at.cell(), at.level(), tuning);
        CanEngageNet::new(*in_arc && *ready)
    }
}

fn handle_battle_sightline(
    reads: SightlineReads,
    mut queue: ResMut<PendingQueue<CommandCall<BattleSightline>>>,
) {
    if queue.is_empty() {
        return;
    }
    let token = reads
        .shooter()
        .map(|entity| GangerToken::new(entity.to_bits()));
    for (args, responder) in take_calls::<BattleSightline>(&mut queue) {
        responder.answer(&BattleSightlineReply {
            at:        args.at,
            shooter:   token,
            sightline: reads.sightline(args.at.to_sim()),
        });
    }
}
