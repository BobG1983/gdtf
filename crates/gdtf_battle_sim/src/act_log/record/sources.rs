use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    act_log::{
        provenance::ActProvenance,
        witness::{ActWitnesses, WatchingFactions},
    },
    acts::{
        FireDeclaration, InjuryInflicted, MeleeResolved, MeleeStruck, MoveCompleted, MoveRejected,
        MovementOccurred, ReloadResult, ThrowResolved,
    },
    armor_wear::ArmorBroken,
    battle::{BattleRoster, PlayerFaction},
    combatants::ganger::{Aiming, Facing, Hp, Position, Stance, Tu, Wounds},
    effects::{
        bleed::{BleedStarted, Bleeding},
        dot::{DotAfflicted, DotTicked},
        fields::{FieldAfflicted, FieldTicked},
        on_death::OnDeathOccurred,
    },
    falls::FallOccurred,
    ganger::{Faction, LifeState, Suppressed},
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
    magazine::Magazine,
    metric::CellLevel,
    occupancy_sync::TerrainPieceDestroyed,
    reaction::InterruptDeclared,
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
    turn::{ActiveFaction, TurnStarted},
    visibility::SquadVisibility,
    weapon::WieldedBy,
};

pub(super) fn cell_order(position: &Position) -> (i32, i32, i32) {
    let key = ***position;
    (key.z, key.y, key.x)
}

#[derive(SystemParam)]
pub struct ProvenanceSources<'w, 's> {
    pub(super) player:   Option<Res<'w, PlayerFaction>>,
    pub(super) active:   Option<Res<'w, ActiveFaction>>,
    pub(super) factions: Query<'w, 's, &'static Faction>,
}

impl ProvenanceSources<'_, '_> {
    pub(super) fn of(&self, actor: Entity) -> ActProvenance {
        let (Some(player), Ok(faction)) = (self.player.as_deref(), self.factions.get(actor)) else {
            return ActProvenance::Clock;
        };
        if *faction == **player {
            ActProvenance::Commanded
        } else {
            ActProvenance::AiTurn
        }
    }

    pub(super) const fn clock() -> ActProvenance {
        ActProvenance::Clock
    }

    pub(super) const fn turn_active(&self) -> bool {
        self.active.is_some()
    }
}

/// The fog and the cells the recorder resolves an act's witnesses from.
#[derive(SystemParam)]
pub struct ActObservation<'w, 's> {
    places: Query<'w, 's, &'static Position>,
    squad:  Option<Res<'w, SquadVisibility>>,
    player: Option<Res<'w, PlayerFaction>>,
    roster: Option<Res<'w, BattleRoster>>,
}

impl ActObservation<'_, '_> {
    /// The cell an entity stands on, when the sim has placed it.
    pub(super) fn cell_of(&self, entity: Entity) -> Option<CellLevel> {
        self.places.get(entity).ok().map(|position| **position)
    }

    /// The gang the player commands, when a battle has named one.
    pub(super) fn player_faction(&self) -> Option<Faction> {
        self.player.as_deref().map(|player| **player)
    }

    /// The player squad's fog, when a battle has built one.
    pub(super) fn squad(&self) -> Option<&SquadVisibility> {
        self.squad.as_deref()
    }

    /// Factions that could observe an act touching any of `cells`.
    pub(super) fn watching(&self, cells: &[CellLevel]) -> WatchingFactions {
        let player = self.player.as_deref().map(|player| **player);
        let mut seen: Vec<Faction> = self
            .roster
            .as_deref()
            .map(|roster| roster.factions().filter(|f| Some(*f) != player).collect())
            .unwrap_or_default();
        if let (Some(player), Some(squad)) = (player, self.squad.as_deref())
            && cells.iter().any(|at| *squad.is_cell_visible(at))
        {
            seen.push(player);
        }
        WatchingFactions::new(seen)
    }

    /// Every fielded faction — what an act nothing can hide is recorded against.
    fn everyone(&self) -> WatchingFactions {
        self.roster
            .as_deref()
            .map_or_else(WatchingFactions::nobody, |roster| {
                WatchingFactions::new(roster.factions())
            })
    }

    /// Factions that could put a name to the ganger standing where `entity` stands.
    pub(super) fn naming(&self, entity: Entity) -> WatchingFactions {
        match self.cell_of(entity) {
            Some(at) => self.watching(&[at]),
            None => WatchingFactions::nobody(),
        }
    }

    /// Witnesses for an act touching only the cell its actor stands on.
    pub(super) fn of_actor(&self, actor: Entity) -> ActWitnesses {
        let naming = self.naming(actor);
        ActWitnesses::new(naming.clone(), naming)
    }

    /// Witnesses for an act at a cell the deed names, by an actor standing there.
    pub(super) fn of_cell(&self, at: CellLevel) -> ActWitnesses {
        let watching = self.watching(&[at]);
        ActWitnesses::new(watching.clone(), watching)
    }

    /// Witnesses for an act at a cell the deed names, with nobody to name.
    pub(super) fn of_cell_unnamed(&self, at: CellLevel) -> ActWitnesses {
        ActWitnesses::new(self.watching(&[at]), WatchingFactions::nobody())
    }

    /// Witnesses for an act whose effect lands away from the actor that caused it.
    pub(super) fn of_effect(&self, cells: &[CellLevel], actor: Entity) -> ActWitnesses {
        ActWitnesses::new(self.watching(cells), self.naming(actor))
    }

    /// Witnesses for an act no fog can hide and nobody is named in.
    pub(super) fn everywhere_unnamed(&self) -> ActWitnesses {
        ActWitnesses::new(self.everyone(), WatchingFactions::nobody())
    }
}

#[derive(SystemParam)]
pub struct TurnSources<'w, 's> {
    pub(super) turns: MessageReader<'w, 's, TurnStarted>,
}

type PostureColumns = (
    Entity,
    &'static Position,
    &'static Facing,
    &'static Stance,
    &'static Aiming,
    Option<&'static Suppressed>,
);

#[derive(SystemParam)]
pub struct PostureSources<'w, 's> {
    pub(super) gangers: Query<'w, 's, PostureColumns>,
}

#[derive(SystemParam)]
pub struct MovementSources<'w, 's> {
    pub(super) steps:     MessageReader<'w, 's, MovementOccurred>,
    pub(super) moves:     MessageReader<'w, 's, MoveCompleted>,
    pub(super) refusals:  MessageReader<'w, 's, MoveRejected>,
    pub(super) positions: Query<'w, 's, (Entity, &'static Position)>,
}

/// Every ganger's cell and gang, for the check that one has just come into view.
#[derive(SystemParam)]
pub struct ViewSources<'w, 's> {
    pub(super) gangers: Query<'w, 's, (Entity, &'static Position, &'static Faction)>,
}

#[derive(SystemParam)]
pub struct FireSources<'w, 's> {
    pub(super) declarations: MessageReader<'w, 's, FireDeclaration>,
    pub(super) rounds:       MessageReader<'w, 's, ShotFired>,
    pub(super) interrupts:   MessageReader<'w, 's, InterruptDeclared>,
}

#[derive(SystemParam)]
pub struct ConsequenceMessages<'w, 's> {
    pub(super) reloads:       MessageReader<'w, 's, ReloadResult>,
    pub(super) injuries:      MessageReader<'w, 's, InjuryInflicted>,
    pub(super) falls:         MessageReader<'w, 's, FallOccurred>,
    pub(super) strikes:       MessageReader<'w, 's, MeleeStruck>,
    pub(super) deaths:        MessageReader<'w, 's, OnDeathOccurred>,
    pub(super) suppressions:  MessageReader<'w, 's, SuppressionApplied>,
    pub(super) armor_breaks:  MessageReader<'w, 's, ArmorBroken>,
    pub(super) dots:          MessageReader<'w, 's, DotAfflicted>,
    pub(super) fields:        MessageReader<'w, 's, FieldAfflicted>,
    pub(super) bleeds:        MessageReader<'w, 's, BleedStarted>,
    pub(super) bleed_ticks:   MessageReader<'w, 's, Bleeding>,
    pub(super) dot_ticks:     MessageReader<'w, 's, DotTicked>,
    pub(super) field_ticks:   MessageReader<'w, 's, FieldTicked>,
    pub(super) cover_smashed: MessageReader<'w, 's, TerrainPieceDestroyed>,
    pub(super) melee_landed:  MessageReader<'w, 's, MeleeResolved>,
    pub(super) throw_landed:  MessageReader<'w, 's, ThrowResolved>,
}

type VitalsColumns = (
    Entity,
    &'static Position,
    &'static Tu,
    &'static Hp,
    &'static Wounds,
    Option<&'static InflictedWounds>,
    Option<&'static InflictedInjuries>,
);

#[derive(SystemParam)]
pub struct ConsequenceState<'w, 's> {
    pub(super) vitals:    Query<'w, 's, VitalsColumns>,
    pub(super) magazines: Query<'w, 's, (Entity, &'static Magazine, &'static WieldedBy)>,
    pub(super) wielders:  Query<'w, 's, &'static Position>,
}

#[derive(SystemParam)]
pub struct LifeSources<'w, 's> {
    pub(super) gangers: Query<'w, 's, (Entity, &'static Position, &'static LifeState)>,
}

/// This frame's act messages, grouped by act family.
#[derive(SystemParam)]
pub struct ActMessages<'w, 's> {
    pub(super) turn:         TurnSources<'w, 's>,
    pub(super) movement:     MovementSources<'w, 's>,
    pub(super) fire:         FireSources<'w, 's>,
    pub(super) consequences: ConsequenceMessages<'w, 's>,
}

/// Current combatant state the log samples alongside those messages.
#[derive(SystemParam)]
pub struct ActState<'w, 's> {
    pub(super) posture: PostureSources<'w, 's>,
    pub(super) vitals:  ConsequenceState<'w, 's>,
    pub(super) life:    LifeSources<'w, 's>,
    pub(super) view:    ViewSources<'w, 's>,
}
