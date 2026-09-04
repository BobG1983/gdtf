//! What a cost case reads off the live world, and how it asks and decodes.

use bevy::{app::App, ecs::entity::Entity, prelude::World};
use cobalt_mcp_protocol::{
    command::RunOptions,
    message::{QaRequest, QaResponse},
};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    acts::DismountSurcharge,
    floor::FloorCostGrid,
    ganger::{Aiming, Facing, Faction, Position, Stance, Tu, TuMax},
    injuries::{InflictedInjuries, MovementCostFactor},
    magazine::mode_tu_cost,
    occupancy::OccupancyGrid,
    pathfinder::{Departure, MoveGrids, PlanningView, find_path, reachable_within},
    prelude::CellLevel,
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, SquadVisibility},
    weapon::{FireMode, FireModeSpec, MeleeWeapon, Wields},
};
use gdtf_game::qa_wire::{
    cost::{CostActNet, CostLegalNet, CostRefusalNet},
    token::GangerToken,
    vitals::TuNet,
};
use serde::Deserialize;

use crate::{
    battle_reads::player_gangers,
    command_exchange::{BATTLE_COST, ran_body, run},
    socket_support::TestError,
};

/// Frames a case runs so the app's own systems settle before it reads or writes.
const SETTLE_FRAMES: u8 = 8;

/// The reply shape `battle.cost` publishes, decoded the way a client decodes it.
#[derive(Debug, Deserialize)]
pub(crate) struct CostBody {
    pub(crate) cost:    Option<TuNet>,
    pub(crate) legal:   CostLegalNet,
    pub(crate) refusal: Option<CostRefusalNet>,
}

/// The actor pose, the selection and the gun's mode a cost call must leave as it found them.
#[derive(Debug, PartialEq)]
pub(crate) struct Pose {
    tu:        Tu,
    stance:    Stance,
    facing:    Facing,
    aiming:    Aiming,
    position:  Position,
    shooter:   Option<Entity>,
    fire_mode: Option<FireModeSpec>,
}

/// Run the app far enough that the systems writing around this point have finished.
pub(crate) fn settle(app: &mut App) {
    for _ in 0..SETTLE_FRAMES {
        app.update();
    }
}

/// One `battle.cost` request, addressed to `actor` and asking about `act`.
pub(crate) fn cost_call(actor: GangerToken, act: &CostActNet) -> Option<QaRequest> {
    let encoded = ron::ser::to_string(act).ok()?;
    Some(run(
        BATTLE_COST,
        &format!("(actor:{},act:{encoded})", *actor),
        RunOptions::default(),
    ))
}

/// One `battle.cost` request per act, in the order given.
pub(crate) fn cost_calls(actor: Entity, acts: &[CostActNet]) -> Vec<QaRequest> {
    let token = GangerToken::new(actor.to_bits());
    acts.iter()
        .filter_map(|act| cost_call(token, act))
        .collect()
}

/// The body of the first reply, or a failure naming what came back instead.
pub(crate) fn first_body(replies: Vec<QaResponse>) -> Result<CostBody, TestError> {
    let Some(reply) = replies.into_iter().next() else {
        return Err("`battle.cost` produced no reply at all".into());
    };
    cost_body(reply)
}

/// The body of one reply, decoded into the shape the command publishes.
pub(crate) fn cost_body(reply: QaResponse) -> Result<CostBody, TestError> {
    let body = ran_body(BATTLE_COST, reply)?;
    ron::de::from_str::<CostBody>(&body).map_err(|fault| {
        format!("a cost body must decode into its published shape: {fault} — {body}").into()
    })
}

/// The actor's pose, the selection and the gun's mode, as the live world holds them.
pub(crate) fn pose(app: &App, actor: Entity) -> Option<Pose> {
    let world = app.world();
    let row = world.get_entity(actor).ok()?;
    Some(Pose {
        tu:        *row.get::<Tu>()?,
        stance:    *row.get::<Stance>()?,
        facing:    *row.get::<Facing>()?,
        aiming:    *row.get::<Aiming>()?,
        position:  *row.get::<Position>()?,
        shooter:   world
            .get_resource::<SelectedShooter>()
            .and_then(|selected| **selected),
        fire_mode: crate::fire_mode_support::selected_mode(app),
    })
}

/// What the sim's own A* charges for walking the actor to `dest`, off the live world.
pub(crate) fn route_cost(app: &App, actor: Entity, dest: CellLevel) -> Option<Tu> {
    let world = app.world();
    let (grids, squad) = terrain(app)?;
    let mover = mover(world, actor)?;
    let planning = planning_view(world, squad, mover.faction);
    find_path(mover.start, dest, grids, mover.factor, &planning)
        .ok()
        .map(|path| path.total())
}

/// The first cell the actor can actually walk to and be charged for.
pub(crate) fn a_reachable_cell(app: &App, actor: Entity) -> Option<CellLevel> {
    a_reachable_cell_where(app, actor, |_| true)
}

/// The first cell the actor can walk to and be charged for that `wanted` also accepts.
pub(crate) fn a_reachable_cell_where(
    app: &App,
    actor: Entity,
    wanted: impl Fn(&CellLevel) -> bool,
) -> Option<CellLevel> {
    let world = app.world();
    let (grids, squad) = terrain(app)?;
    let mover = mover(world, actor)?;
    let planning = planning_view(world, squad, mover.faction);
    reachable_within(
        &Departure::anywhere(mover.start),
        mover.budget,
        DismountSurcharge::NONE,
        grids,
        mover.factor,
        &planning,
    )
    .into_iter()
    .find(|(cell, cost)| *cell != mover.start && **cost > 0 && wanted(cell))
    .map(|(cell, _)| cell)
}

/// A player ganger holding a ranged weapon, with every mode that weapon offers.
///
/// No two modes share a kind, so the kind a call names picks exactly one price.
pub(crate) fn a_gun_with_modes(app: &App) -> Option<(Entity, Vec<FireModeSpec>)> {
    player_gangers(app).into_iter().find_map(|actor| {
        let modes = fire_modes(app.world(), actor)?;
        (!modes.is_empty() && kinds_are_distinct(&modes)).then_some((actor, modes))
    })
}

/// Every fire mode the actor's ranged weapon offers, if it holds one.
fn fire_modes(world: &World, actor: Entity) -> Option<Vec<FireModeSpec>> {
    let held = world.get_entity(actor).ok()?.get::<Wields>()?;
    let weapon = held.ranged_weapon(|entity| {
        world
            .get_entity(entity)
            .is_ok_and(|row| row.contains::<MeleeWeapon>())
    })?;
    Some(
        world
            .get_entity(weapon)
            .ok()?
            .get::<FireMode>()?
            .iter()
            .copied()
            .collect(),
    )
}

/// Whether no two of these modes answer to the same kind, so a kind names exactly one price.
fn kinds_are_distinct(modes: &[FireModeSpec]) -> bool {
    modes.iter().enumerate().all(|(place, spec)| {
        !modes
            .iter()
            .skip(place + 1)
            .any(|other| other.kind == spec.kind)
    })
}

/// What the sim charges the actor for one shot in `spec`, off the live world.
pub(crate) fn fire_cost(app: &App, actor: Entity, spec: &FireModeSpec) -> Option<Tu> {
    let world = app.world();
    let row = world.get_entity(actor).ok()?;
    Some(mode_tu_cost(
        spec,
        row.get::<TuMax>()?,
        row.get::<Aiming>()?,
        world.get_resource::<CombatTuning>()?,
    ))
}

/// Where the mover starts, what it fights for, what it can spend and how it walks.
struct Mover {
    start:   CellLevel,
    faction: Faction,
    budget:  Tu,
    factor:  MovementCostFactor,
}

fn mover(world: &World, actor: Entity) -> Option<Mover> {
    let row = world.get_entity(actor).ok()?;
    Some(Mover {
        start:   **row.get::<Position>()?,
        faction: *row.get::<Faction>()?,
        budget:  *row.get::<Tu>()?,
        factor:  row.get::<InflictedInjuries>().map_or(
            MovementCostFactor::IDENTITY,
            InflictedInjuries::movement_cost_factor,
        ),
    })
}

fn terrain(app: &App) -> Option<(MoveGrids<'_>, &SquadVisibility)> {
    let world = app.world();
    Some((
        MoveGrids {
            occupancy:   world.get_resource::<OccupancyGrid>()?,
            links:       world.get_resource::<VerticalLinkGraph>()?,
            floor_costs: world.get_resource::<FloorCostGrid>()?,
            tuning:      world.get_resource::<CombatTuning>()?,
        },
        world.get_resource::<SquadVisibility>()?,
    ))
}

fn planning_view<'a>(
    world: &'a World,
    squad: &'a SquadVisibility,
    mover: Faction,
) -> PlanningView<'a, impl Fn(Entity) -> FactionRelation + 'a> {
    PlanningView::new(squad, move |occupant| {
        match world
            .get_entity(occupant)
            .ok()
            .and_then(|row| row.get::<Faction>().copied())
        {
            Some(faction) if faction == mover => FactionRelation::OwnSquad,
            _ => FactionRelation::Other,
        }
    })
}
