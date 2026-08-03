use bevy::prelude::Entity;

use super::facts::{MagazineFacts, PoseFacts, PositionFacts, VitalsFacts};
use crate::{
    acts::{InjuryInflicted, MoveRejection, ReloadOutcome, RoundCount},
    armor::BodyPart,
    effects::fields::FieldDamage,
    falls::StoreysFallen,
    ganger::{Faction, LifeState},
    metric::{Cell, CellLevel, Level},
    resolve_hit::HpDamage,
    shot_fired::ShotFired,
    weapon::{DamageType, DotDamage, ModeKind},
};

#[derive(Debug, Clone, PartialEq)]
pub enum ActDeed {
            TurnBegan {
                now_active: Faction,
    },

            PostureChanged {
                pose: PoseFacts,
    },

            Stepped {
                from:     Cell,
                to:       Cell,
                        position: PositionFacts,
    },

                    MovedTo {
                position: PositionFacts,
    },

            MoveRefused {
                reason: MoveRejection,
    },

                Fired {
                target: Option<Entity>,
                mode:   ModeKind,
                        rounds: RoundCount,
    },

            RoundResolved {
                shot: Box<ShotFired>,
    },

            Reloaded {
                outcome: ReloadOutcome,
    },

                MagazineChanged {
                magazine: MagazineFacts,
    },

                            Injured {
                injury: Box<InjuryInflicted>,
    },

                    VitalsChanged {
                vitals: VitalsFacts,
    },

            Fell {
                from_level: Level,
                to_level:   Level,
                storeys:    StoreysFallen,
    },

            Struck {
                target:    Entity,
                hp_damage: HpDamage,
    },

                DiedAt {
                at: CellLevel,
    },

            Suppressed {
                at: CellLevel,
    },

            ArmorBroke {
                part: BodyPart,
    },

            DotStarted {
                per_turn: DotDamage,
    },

            FieldStarted {
                at: CellLevel,
    },

            BleedStarted,

                Bled,

                    DotTicked {
                at:     CellLevel,
                amount: DotDamage,
    },

                    FieldTicked {
                at:     CellLevel,
                amount: FieldDamage,
    },

                CoverSmashed {
                at: CellLevel,
    },

                MeleeLanded {
                at:     CellLevel,
                damage: DamageType,
    },

            ThrowLanded {
                at:     CellLevel,
                damage: DamageType,
    },

                                        LifeChanged {
                from: LifeState,
                to:   LifeState,
                at:   PositionFacts,
    },
}
