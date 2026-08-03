use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    act_log::provenance::ActProvenance,
    acts::{
        FireDeclaration, InjuryInflicted, MeleeResolved, MeleeStruck, MoveRejected,
        MovementOccurred, ReloadResult, ThrowResolved,
    },
    armor_wear::ArmorBroken,
    battle::PlayerFaction,
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
    occupancy_sync::CoverDestroyed,
    reaction::InterruptDeclared,
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
    turn::{ActiveFaction, TurnStarted},
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
        pub(super) refusals:  MessageReader<'w, 's, MoveRejected>,
            pub(super) positions: Query<'w, 's, (Entity, &'static Position)>,
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
        pub(super) cover_smashed: MessageReader<'w, 's, CoverDestroyed>,
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
