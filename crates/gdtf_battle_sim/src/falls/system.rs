//! System: after slab destruction, drop standing gangers and apply fall damage.

use bevy::{
    ecs::system::SystemParam,
    prelude::{Entity, MessageReader, MessageWriter, Query, Res, ResMut},
};

use crate::{
    acts::InjuryInflicted,
    armor::{BodyPart, PieceArmorMut, Wears, WornBy},
    effects::on_death::OnDeathOccurred,
    falls::{
        FallOccurred,
        damage::{FallImpact, FallWoundEnv, resolve_fall_hit},
        resolve::resolve_drop,
    },
    ganger::{Hp, LifeState, Luck, Position, Toughness, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::{InjuryRegistry, InjuryTables},
    metric::{CellLevel, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::SlabDestroyed,
    resolve_and_apply::{StruckPiece, TargetGanger},
    rng::{InjuryRng, SeverityRng},
    surface::SurfaceGrid,
    tuning::CombatTuning,
};

type FallerQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        Entity,
        &'static mut Position,
        &'static mut Hp,
        &'static mut Wounds,
        &'static mut LifeState,
        &'static mut InflictedWounds,
        &'static Toughness,
        &'static Luck,
    ),
>;

/// Grids and tables needed to resolve falls.
#[derive(SystemParam)]
pub struct FallGrids<'w> {
    surface: Option<Res<'w, SurfaceGrid>>,
    occupancy: Option<Res<'w, OccupancyGrid>>,
    tuning: Option<Res<'w, CombatTuning>>,
    tables: Option<Res<'w, InjuryTables>>,
    registry: Option<Res<'w, InjuryRegistry>>,
}

/// RNG streams for fall wound rolls.
#[derive(SystemParam)]
pub struct FallRngs<'w> {
    severity: Option<ResMut<'w, SeverityRng>>,
    injury: Option<ResMut<'w, InjuryRng>>,
}

/// Armor queries for the falling ganger.
#[derive(SystemParam)]
pub struct FallArmor<'w, 's> {
    wears: Query<'w, 's, &'static Wears>,
    pieces: Query<'w, 's, PieceArmorMut, bevy::prelude::With<WornBy>>,
}

/// Outbound fall / injury / death messages.
#[derive(SystemParam)]
pub struct FallSignals<'w> {
    fell: MessageWriter<'w, FallOccurred>,
    injuries: MessageWriter<'w, InjuryInflicted>,
    deaths: MessageWriter<'w, OnDeathOccurred>,
}

/// On [`SlabDestroyed`], move gangers down and resolve fall hits.
pub fn apply_falls(
    mut destroyed: MessageReader<SlabDestroyed>,
    mut fallers: FallerQuery,
    mut armor: FallArmor,
    grids: FallGrids,
    rngs: FallRngs,
    mut signals: FallSignals,
) {
    let (Some(mut severity_rng), Some(mut injury_rng)) = (rngs.severity, rngs.injury) else {
        return;
    };

    let (Some(surface), Some(occupancy), Some(tuning), Some(tables), Some(registry)) = (
        grids.surface,
        grids.occupancy,
        grids.tuning,
        grids.tables,
        grids.registry,
    ) else {
        return;
    };

    for event in destroyed.read() {
        let (destroyed_cell, destroyed_level) = event.at.split();

        for (entity, mut position, mut hp, mut wounds, mut life, mut inflicted, toughness, luck) in
            &mut fallers
        {
            if !*life.is_active() {
                continue;
            }
            if position.cell() != destroyed_cell || position.level() != destroyed_level {
                continue;
            }

            if *occupancy.is_stair_cell(&position) {
                continue;
            }

            let start = position.level();
            let Some(landing) = resolve_drop(destroyed_cell, start, &surface) else {
                continue;
            };

            *position = Position::new(CellLevel::new(destroyed_cell, landing.landing));

            let part = BodyPart::Torso;
            let piece_view = armor
                .wears
                .get(entity)
                .ok()
                .and_then(|worn| worn.pieces().next())
                .and_then(|piece_entity| {
                    armor
                        .pieces
                        .get_mut(piece_entity)
                        .ok()
                        .map(|piece| StruckPiece {
                            floor: *piece.floor,
                            protection: *piece.protection,
                            hardness: *piece.hardness,
                            armor_type: *piece.armor_type,
                            integrity: piece.integrity.into_inner(),
                        })
                });

            let rolled = resolve_fall_hit(
                FallImpact {
                    per_storey: tuning.per_storey_damage,
                    storeys: landing.storeys,
                    part,
                    target: TargetGanger {
                        hp: &mut hp,
                        wounds: &mut wounds,
                        life: &mut life,
                        piece: piece_view,
                        inflicted: &mut inflicted,
                        toughness: *toughness,
                        luck: *luck,
                    },
                    target_entity: entity,
                },
                FallWoundEnv {
                    tuning: &tuning,
                    tables: &tables,
                    registry: &registry,
                    severity_rng: &mut severity_rng,
                    injury_rng: &mut injury_rng,
                },
            );

            emit_fall_signals(
                &mut signals,
                entity,
                *life,
                &position,
                start,
                landing,
                rolled,
            );
        }
    }
}

fn emit_fall_signals(
    signals: &mut FallSignals,
    entity: Entity,
    life: LifeState,
    position: &Position,
    start: Level,
    landing: crate::falls::DropLanding,
    rolled: Option<crate::injuries::RolledInjury>,
) {
    if let Some(rolled) = rolled {
        signals
            .injuries
            .write(InjuryInflicted::from_rolled(entity, rolled));
    }
    if life == LifeState::Dead {
        signals
            .deaths
            .write(OnDeathOccurred::new(entity, **position));
    }
    signals.fell.write(FallOccurred::new(
        entity,
        start,
        landing.landing,
        landing.storeys,
    ));
}
