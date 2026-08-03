//! Resolve and apply shove requests.

use bevy::{
    ecs::system::SystemParam,
    prelude::{MessageReader, MessageWriter, Query, Res, ResMut, With},
};

use super::{
    apply::{ShoveFallEnv, ShoveTargetSurfaces, apply_shove},
    verb::resolve_shove,
};
use crate::{
    acts::{
        InjuryInflicted,
        downed::is_8_adjacent,
        request::{ShoveRequested, ShoveSource},
    },
    armor::{PieceArmorMut, Wears, WornBy},
    falls::FallOccurred,
    ganger::{Faction, Hp, LifeState, Luck, Position, Toughness, Tu, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::{InjuryRegistry, InjuryTables},
    occupancy::OccupancyGrid,
    rng::{InjuryRng, SeverityRng},
    surface::SurfaceGrid,
    tu::spend_tu,
    tuning::CombatTuning,
};

type ShoveGangerQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static mut Position,
        &'static mut Hp,
        &'static mut Wounds,
        &'static mut LifeState,
        &'static mut InflictedWounds,
        &'static Toughness,
        &'static Luck,
        &'static Faction,
    ),
>;

/// Grid and table resources for shove resolution.
#[derive(SystemParam)]
pub struct ShoveGrids<'w> {
    surface: Option<Res<'w, SurfaceGrid>>,
    occupancy: Option<Res<'w, OccupancyGrid>>,
    tuning: Option<Res<'w, CombatTuning>>,
    tables: Option<Res<'w, InjuryTables>>,
    registry: Option<Res<'w, InjuryRegistry>>,
}

/// RNG streams used when a shove causes a fall or injury.
#[derive(SystemParam)]
pub struct ShoveRngs<'w> {
    severity: Option<ResMut<'w, SeverityRng>>,
    injury: Option<ResMut<'w, InjuryRng>>,
}

/// Spend TU, resolve shove destination, and apply falls/injuries.
#[expect(
    clippy::too_many_arguments,
    reason = "the shove dispatch threads the request reader, the single ganger query (read + \
              fold), the shover-Tu query, the two armor relationship queries, the grouped grids \
              (ShoveGrids) + fall streams (ShoveRngs) bundles, and the FallOccurred + \
              InjuryInflicted writers — the irreducible access set (the apply_falls / \
              dispatch_melee argument-count precedent)"
)]
pub fn dispatch_shove(
    mut requests: MessageReader<ShoveRequested>,
    mut gangers: ShoveGangerQuery,
    mut tu_q: Query<&mut Tu>,
    wears: Query<&Wears>,
    mut pieces: Query<PieceArmorMut, With<WornBy>>,
    grids: ShoveGrids,
    rngs: ShoveRngs,
    mut fell: MessageWriter<FallOccurred>,
    mut injuries: MessageWriter<InjuryInflicted>,
) {
    let (Some(surface), Some(occupancy), Some(tuning)) =
        (grids.surface, grids.occupancy, grids.tuning)
    else {
        return;
    };
    let (Some(mut severity_rng), Some(mut injury_rng)) = (rngs.severity, rngs.injury) else {
        return;
    };
    let empty_tables = InjuryTables::default();
    let empty_registry = InjuryRegistry::default();
    let tables: &InjuryTables = grids.tables.as_deref().unwrap_or(&empty_tables);
    let registry: &InjuryRegistry = grids.registry.as_deref().unwrap_or(&empty_registry);

    for request in requests.read() {
        let Ok((&shover_pos, _, _, _, _, _, _, &shover_faction)) = gangers.get(request.shover)
        else {
            continue;
        };
        let Ok((&target_pos, _, _, &target_life, _, _, _, &target_faction)) =
            gangers.get(request.target)
        else {
            continue;
        };

        if request.source == ShoveSource::Deliberate {
            if !*is_8_adjacent(shover_pos, target_pos)
                || shover_faction == target_faction
                || !*target_life.is_active()
            {
                continue;
            }
            let Ok(mut shover_tu) = tu_q.get_mut(request.shover) else {
                continue;
            };
            spend_tu(&mut shover_tu, Tu::new(*tuning.shove_tu));
        }

        let outcome = resolve_shove(shover_pos, target_pos, request.target, &surface, &occupancy);

        let Ok((position, hp, wounds, life, inflicted, &toughness, &luck, _)) =
            gangers.get_mut(request.target)
        else {
            continue;
        };
        apply_shove(
            outcome,
            ShoveTargetSurfaces {
                position: position.into_inner(),
                hp: hp.into_inner(),
                wounds: wounds.into_inner(),
                life: life.into_inner(),
                inflicted: inflicted.into_inner(),
                toughness,
                luck,
            },
            request.target,
            &wears,
            &mut pieces,
            ShoveFallEnv {
                tuning: &tuning,
                tables,
                registry,
                severity_rng: &mut severity_rng,
                injury_rng: &mut injury_rng,
            },
            &mut fell,
            &mut injuries,
        );
    }
}
