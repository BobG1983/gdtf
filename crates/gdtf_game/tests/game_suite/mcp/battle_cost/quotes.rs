//! Every act priced off gear, posture or tuning quotes the sim helper that charges it.

use bevy::{
    app::App,
    ecs::entity::Entity,
    prelude::{Component, World},
};
use gdtf_battle_sim::{
    acts::{
        enter_emplacement_tu_cost, exit_emplacement_tu_cost, melee_tu_cost, open_door_tu_cost,
        reload_tu_cost, shove_tu_cost, throw_grenade_tu_cost,
    },
    emplacement::{EmplacementEntrySides, EmplacementFacing, EmplacementState},
    entity::TerrainCell,
    ganger::{Facing, Tu},
    magazine::Magazine,
    openable::OpenState,
    posture::{afforded_turn_tu_cost, set_aiming_tu_cost, stance_tu_cost},
    terrain::facing::TerrainFacing,
    tuning::CombatTuning,
    weapon::{FightMode, MeleeWeapon, Wields},
};
use gdtf_game::qa_wire::{
    act_payload::{AimNet, FacingNet, MeleeTargetNet, StanceNet},
    cell::CellLevelNet,
    cost::CostActNet,
    token::{DoorToken, EmplacementToken, GangerToken},
    vitals::TuNet,
};

use super::support::{cost_body, cost_calls, settle};
use crate::mcp::{
    battle_reads::{a_player_ganger, an_enemy_ganger, one_cardinal_step_from},
    command_exchange::exchange_in_battle,
    socket_support::{TestError, TestResult},
};

/// The actor and the live targets the priced acts are asked against.
struct Bench {
    actor: Entity,
    at:    CellLevelNet,
    enemy: GangerToken,
    door:  DoorToken,
    seat:  EmplacementToken,
}

/// Everything this case prices, each act aimed at something the battle actually holds.
fn priced_acts(bench: &Bench) -> Vec<CostActNet> {
    vec![
        CostActNet::Reload,
        CostActNet::SetStance {
            stance: StanceNet::Prone,
        },
        CostActNet::SetFacing {
            facing: FacingNet::South,
        },
        CostActNet::SetAiming {
            aim: AimNet::new(true),
        },
        CostActNet::Shove {
            target: bench.enemy,
        },
        CostActNet::OpenDoor { target: bench.door },
        CostActNet::EnterEmplacement { target: bench.seat },
        CostActNet::ExitEmplacement { target: bench.seat },
        CostActNet::ThrowGrenade { target: bench.at },
        CostActNet::Melee {
            target: MeleeTargetNet::Ganger(bench.enemy),
        },
    ]
}

/// What the sim's own helper charges for `act`, read off the same world that answered.
fn quoted_by_the_sim(app: &App, actor: Entity, act: &CostActNet) -> Option<Tu> {
    let world = app.world();
    let tuning = world.get_resource::<CombatTuning>()?;
    match act {
        CostActNet::Reload => Some(reload_tu_cost(part::<Magazine>(world, gun(world, actor)?)?)),
        CostActNet::SetStance { .. } => Some(stance_tu_cost(&tuning.stance_change_tu)),
        CostActNet::SetFacing { facing } => Some(afforded_turn_tu_cost(
            **part::<Facing>(world, actor)?,
            facing.to_sim(),
            part::<Tu>(world, actor)?,
            &tuning.turn_tu,
        )),
        CostActNet::SetAiming { .. } => Some(set_aiming_tu_cost()),
        CostActNet::Shove { .. } => Some(shove_tu_cost(tuning)),
        CostActNet::OpenDoor { .. } => Some(open_door_tu_cost(tuning)),
        CostActNet::EnterEmplacement { .. } => Some(enter_emplacement_tu_cost(tuning)),
        CostActNet::ExitEmplacement { .. } => Some(exit_emplacement_tu_cost(tuning)),
        CostActNet::ThrowGrenade { .. } => Some(throw_grenade_tu_cost(tuning)),
        CostActNet::Melee { .. } => Some(melee_tu_cost(part::<FightMode>(
            world,
            blade(world, actor)?,
        )?)),
        CostActNet::Move { .. } | CostActNet::Fire { .. } => None,
    }
}

#[test]
fn every_gear_posture_and_reach_act_quotes_its_own_sim_helper() -> TestResult {
    let mut planned: Option<Result<(Entity, Vec<CostActNet>), TestError>> = None;
    let (app, replies) = exchange_in_battle(|app| {
        settle(app);
        match bench(app) {
            Ok(bench) => {
                let acts = priced_acts(&bench);
                let calls = cost_calls(bench.actor, &acts);
                planned = Some(Ok((bench.actor, acts)));
                calls
            }
            Err(fault) => {
                planned = Some(Err(fault));
                Vec::new()
            }
        }
    })?;
    let Some(planned) = planned else {
        return Err("the fixture never reported what it lined up to price".into());
    };
    let (actor, acts) = planned?;
    let (Some(tuning), Some(pool)) = (
        app.world().get_resource::<CombatTuning>(),
        part::<Tu>(app.world(), actor),
    ) else {
        return Err("the same world must still hold the tuning and the actor's pool".into());
    };
    assert!(
        **pool >= *enter_emplacement_tu_cost(tuning),
        "the actor's pool must cover the enter cost, or the legality assertion below reads \
         CannotAfford rather than the entry-sides gate",
    );
    assert_eq!(
        replies.len(),
        acts.len(),
        "every act asked must come back with its own reply",
    );
    for (act, reply) in acts.iter().zip(replies) {
        let body = cost_body(reply)?;
        let Some(expected) = quoted_by_the_sim(&app, actor, act) else {
            return Err(
                format!("the same world must still price {act:?} with a sim helper").into(),
            );
        };
        assert_eq!(
            body.cost,
            Some(TuNet::new(*expected)),
            "the quote for {act:?} must be exactly what the sim charges: {body:?}",
        );
        if matches!(act, CostActNet::EnterEmplacement { .. }) {
            assert_eq!(
                body.refusal, None,
                "the seat stands on a cardinal cell beside the actor, so nothing refuses the \
                 enter — ActNotAllowed would say it is on no entry side: {body:?}",
            );
            assert!(
                *body.legal,
                "an act the fixture aims at something the battle actually holds is legal: \
                 {body:?}",
            );
        }
    }
    Ok(())
}

/// The actor, a living enemy, and a door and emplacement standing beside the actor.
///
/// The map this seed generates holds neither a door nor an emplacement, so the case stands
/// one of each on a clear cell next to the actor before it asks.
fn bench(app: &mut App) -> Result<Bench, TestError> {
    let Some((actor, at)) = a_player_ganger(app) else {
        return Err("a running battle must field a ganger the player commands".into());
    };
    let Some(enemy) = an_enemy_ganger(app) else {
        return Err("a running battle must hold a living enemy to aim reach acts at".into());
    };
    let Some(beside) = one_cardinal_step_from(app, at) else {
        return Err(
            "the actor must have a clear CARDINAL cell beside it to stand terrain on, or the \
             emplacement stands on no entry side of its own"
                .into(),
        );
    };
    let door = app
        .world_mut()
        .spawn((TerrainCell::new(beside.to_sim()), OpenState::Closed))
        .id();
    let seat = app
        .world_mut()
        .spawn((
            TerrainCell::new(beside.to_sim()),
            EmplacementState::Vacant,
            EmplacementEntrySides::new(TerrainFacing::ALL.to_vec()),
            EmplacementFacing::new(TerrainFacing::default()),
        ))
        .id();
    Ok(Bench {
        actor,
        at,
        enemy: GangerToken::new(enemy.to_bits()),
        door: DoorToken::new(door.to_bits()),
        seat: EmplacementToken::new(seat.to_bits()),
    })
}

/// One component off an entity, absent when the world does not hold it.
fn part<C: Component>(world: &World, entity: Entity) -> Option<&C> {
    world.get_entity(entity).ok()?.get::<C>()
}

/// The gun the actor holds, found the way the command finds it.
fn gun(world: &World, actor: Entity) -> Option<Entity> {
    part::<Wields>(world, actor)?.ranged_weapon(|entity| is_melee(world, entity))
}

/// The melee weapon the actor holds, found the way the command finds it.
fn blade(world: &World, actor: Entity) -> Option<Entity> {
    part::<Wields>(world, actor)?.melee_weapon(|entity| is_melee(world, entity))
}

fn is_melee(world: &World, entity: Entity) -> bool {
    part::<MeleeWeapon>(world, entity).is_some()
}
