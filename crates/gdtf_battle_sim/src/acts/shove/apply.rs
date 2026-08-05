//! Apply shove outcome: move position or resolve a fall hit.

use bevy::{
    ecs::system::SystemParam,
    prelude::{Entity, MessageWriter},
};

use super::verb::ShoveOutcome;
use crate::{
    acts::InjuryInflicted,
    armor::{BodyPart, WornArmor},
    falls::{FallImpact, FallOccurred, FallWoundEnv, resolve_fall_hit},
    ganger::{Hp, LifeState, Luck, Position, Toughness, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::{InjuryRegistry, InjuryTables},
    metric::CellLevel,
    resolve_and_apply::{StruckPiece, TargetGanger},
    rng::{InjuryRng, SeverityRng},
    tuning::CombatTuning,
};

/// Mutable combat surfaces for the shove target.
pub(crate) struct ShoveTargetSurfaces<'a> {
    pub position:  &'a mut Position,
    pub hp:        &'a mut Hp,
    pub wounds:    &'a mut Wounds,
    pub life:      &'a mut LifeState,
    pub inflicted: &'a mut InflictedWounds,
    pub toughness: Toughness,
    pub luck:      Luck,
}

/// Tuning and RNGs used when a shove causes a fall.
pub(crate) struct ShoveFallEnv<'a> {
    pub tuning:       &'a CombatTuning,
    pub tables:       &'a InjuryTables,
    pub registry:     &'a InjuryRegistry,
    pub severity_rng: &'a mut SeverityRng,
    pub injury_rng:   &'a mut InjuryRng,
}

/// Messages a shove writes when the target goes over an edge.
#[derive(SystemParam)]
pub struct ShoveFallSignals<'w> {
    /// The fall itself.
    pub fell:     MessageWriter<'w, FallOccurred>,
    /// Any injury the landing rolled.
    pub injuries: MessageWriter<'w, InjuryInflicted>,
}

/// Move the target, or resolve fall damage and messages when they go over an edge.
pub(crate) fn apply_shove(
    outcome: ShoveOutcome,
    surfaces: ShoveTargetSurfaces<'_>,
    target_entity: Entity,
    armor: &mut WornArmor,
    env: ShoveFallEnv<'_>,
    signals: &mut ShoveFallSignals,
) {
    let ShoveTargetSurfaces {
        position,
        hp,
        wounds,
        life,
        inflicted,
        toughness,
        luck,
    } = surfaces;

    match outcome {
        ShoveOutcome::Blocked => {}

        ShoveOutcome::Moved { dest } => {
            *position = Position::new(dest);
        }

        ShoveOutcome::Fell { dest, landing } => {
            let start = dest.level();
            let dest_cell = dest.cell();

            *position = Position::new(CellLevel::new(dest_cell, landing.landing));

            let piece_view = armor
                .wears
                .get(target_entity)
                .ok()
                .and_then(|worn| worn.pieces().next())
                .and_then(|piece_entity| {
                    armor
                        .pieces
                        .get_mut(piece_entity)
                        .ok()
                        .map(|piece| StruckPiece {
                            floor:      *piece.floor,
                            protection: *piece.protection,
                            hardness:   *piece.hardness,
                            armor_type: *piece.armor_type,
                            integrity:  piece.integrity.into_inner(),
                        })
                });

            let rolled = resolve_fall_hit(
                FallImpact {
                    per_storey: env.tuning.per_storey_damage,
                    storeys: landing.storeys,
                    part: BodyPart::Torso,
                    target: TargetGanger {
                        hp,
                        wounds,
                        life,
                        piece: piece_view,
                        inflicted,
                        toughness,
                        luck,
                    },
                    target_entity,
                },
                FallWoundEnv {
                    tuning:       env.tuning,
                    tables:       env.tables,
                    registry:     env.registry,
                    severity_rng: env.severity_rng,
                    injury_rng:   env.injury_rng,
                },
            );

            if let Some(rolled) = rolled {
                signals
                    .injuries
                    .write(InjuryInflicted::from_rolled(target_entity, rolled));
            }
            signals.fell.write(FallOccurred::new(
                target_entity,
                start,
                landing.landing,
                landing.storeys,
            ));
        }
    }
}
