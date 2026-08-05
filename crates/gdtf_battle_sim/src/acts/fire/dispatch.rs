//! System: process fire requests through arc, volley, and outcome messages.

use bevy::{ecs::system::ParamSet, prelude::MessageReader};

use super::{
    arc::{FireArcDecision, decide_fire_arc},
    emit::emit_round_signals,
    params::{BattleGridsParam, ShotRolls, TurnQuery},
    signals::{FireDeclaration, FireSignals, RoundCount},
};
use crate::{
    acts::request::FireRequested,
    fire::{FireOrder, ShooterQuery, StruckBodies, WieldedWeapons, fire},
    ganger::{Aiming, Direction, Facing, Tu, TuMax},
    injuries::{InjuryRegistry, InjuryTables},
    magazine::mode_tu_cost,
    metric::CellLevel,
    resolve_and_apply::WoundRoll,
    tu::spend_tu,
    weapon::DamageType,
};

/// Read fire requests, turn if needed, run the shot pipeline, emit signals.
pub fn dispatch_fire(
    mut requests: MessageReader<FireRequested>,
    mut shooter_set: ParamSet<(ShooterQuery, TurnQuery)>,
    mut bodies: StruckBodies,
    mut arms: WieldedWeapons,
    mut grids: BattleGridsParam,
    mut rolls: ShotRolls,
    mut signals: FireSignals,
) {
    let empty_tables = InjuryTables::default();
    let empty_registry = InjuryRegistry::default();
    let tables: &InjuryTables = rolls.tables.as_deref().unwrap_or(&empty_tables);
    let registry: &InjuryRegistry = rolls.registry.as_deref().unwrap_or(&empty_registry);
    for request in requests.read() {
        let shooters = shooter_set.p0();
        let Ok(((position, facing, _, aiming, _, _, tu_max, _), _, tu)) =
            shooters.get(request.shooter)
        else {
            continue;
        };
        let actor_cell = position.cell();
        let facing: Direction = **facing;
        let tu: Tu = *tu;
        let tu_max: TuMax = *tu_max;
        let aiming: Aiming = *aiming;

        let Some(weapon_entity) = arms.wields.get(request.shooter).ok().and_then(|w| {
            w.firing_weapon(
                |entity| arms.mounted.get(entity).is_ok(),
                |entity| arms.melee.get(entity).is_ok(),
            )
        }) else {
            continue;
        };
        let Ok((_, _, _, _, _, _, _, damage_type, ..)) = arms.weapons.get(weapon_entity) else {
            continue;
        };
        let damage: DamageType = *damage_type;

        let fire_cost = mode_tu_cost(&request.mode, &tu_max, &aiming, &rolls.tuning);

        match decide_fire_arc(
            facing,
            actor_cell,
            request.target_cell,
            tu,
            fire_cost,
            &rolls.tuning,
        ) {
            FireArcDecision::Reject => continue,
            FireArcDecision::TurnThenFire {
                facing: target_facing,
                turn_cost,
            } => {
                let mut turners = shooter_set.p1();
                let Ok((mut actor_facing, mut actor_tu)) = turners.get_mut(request.shooter) else {
                    continue;
                };
                *actor_facing = Facing::new(target_facing);
                spend_tu(&mut actor_tu, turn_cost);
            }
            FireArcDecision::FireInArc => {}
        }

        let aim_cell_level = CellLevel::new(request.target_cell, request.target_level);
        let target = grids.occupant_at(aim_cell_level);

        let order = FireOrder {
            shooter:      request.shooter,
            mode:         &request.mode,
            target_cell:  request.target_cell,
            target_level: request.target_level,
        };
        let mut shooters = shooter_set.p0();
        let volley = fire(
            order,
            &mut shooters,
            &mut arms,
            &mut bodies,
            grids.grids(),
            &mut rolls.shot,
            &mut WoundRoll {
                tuning: &rolls.tuning,
                severity_rng: &mut rolls.severity,
                tables,
                registry,
                injury_rng: &mut rolls.injury,
            },
        );

        emit_round_signals(request.shooter, damage, &volley, &mut signals);

        signals.declarations.write(FireDeclaration::new(
            request.shooter,
            target,
            request.mode.kind,
            RoundCount::from_emitted(volley.shots.len()),
        ));

        let weapon_shoves = signals.shove_tags.get(weapon_entity).is_ok_and(|tag| **tag);
        let struck_ganger = volley
            .reports
            .iter()
            .find_map(|report| report.verdict.struck_ganger());
        if let (true, Some(struck)) = (weapon_shoves, struck_ganger) {
            signals
                .shoves
                .write(crate::acts::request::ShoveRequested::new_weapon(
                    request.shooter,
                    struck,
                ));
        }
    }
}
