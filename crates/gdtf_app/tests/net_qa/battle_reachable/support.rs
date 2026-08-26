//! What a reachable case asks, how it decodes, and the same search run off the screen's picture.

use bevy::{app::App, ecs::entity::Entity, prelude::World};
use gdtf_app::{
    qa_wire::{
        cell::CellLevelNet, cost::CostRefusalNet, reachable::ReachableCellNet, token::GangerToken,
    },
    test_support::ShownOccupancyGrid,
};
use gdtf_battle_presenter::{DrawnPosition, DrawnVitals, ShownSquadVisibility};
use gdtf_battle_sim::{
    acts::DismountSurcharge,
    emplacement::Mounted,
    floor::FloorCostGrid,
    ganger::{Faction, Position, Tu},
    injuries::{InflictedInjuries, MovementCostFactor},
    occupancy::OccupancyGrid,
    pathfinder::{Departure, MoveGrids, PlanningView, reachable_within},
    prelude::CellLevel,
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, SquadVisibility},
};
use gdtf_qa_protocol::{
    command::RunOptions,
    message::{QaRequest, QaResponse},
};
use serde::Deserialize;

use crate::{
    command_exchange::{BATTLE_REACHABLE, ran_body, run},
    socket_support::TestError,
};

/// The reply shape `battle.reachable` publishes, decoded the way a client decodes it.
#[derive(Debug, Deserialize)]
pub(crate) struct ReachableBody {
    pub(crate) cells:   Option<Vec<ReachableCellNet>>,
    pub(crate) refusal: Option<CostRefusalNet>,
}

/// One `battle.reachable` request, addressed to `actor`.
pub(crate) fn reachable_call(actor: GangerToken) -> QaRequest {
    run(
        BATTLE_REACHABLE,
        &format!("(actor:{})", *actor),
        RunOptions::default(),
    )
}

/// The body of the first reply, or a failure naming what came back instead.
pub(crate) fn first_reachable_body(replies: Vec<QaResponse>) -> Result<ReachableBody, TestError> {
    let Some(reply) = replies.into_iter().next() else {
        return Err("`battle.reachable` produced no reply at all".into());
    };
    let body = ran_body(BATTLE_REACHABLE, reply)?;
    ron::de::from_str::<ReachableBody>(&body).map_err(|fault| {
        format!("a reachable body must decode into its published shape: {fault}. body: {body}")
            .into()
    })
}

/// The cells one reply listed, or a failure naming the refusal that arrived instead.
pub(crate) fn listed_cells(body: &ReachableBody) -> Result<&[ReachableCellNet], TestError> {
    match body.cells.as_deref() {
        Some(cells) => Ok(cells),
        None => Err(format!(
            "`battle.reachable` must list the cells this actor reaches, and answered no list \
             at all: {body:?}"
        )
        .into()),
    }
}

/// Whether the reply lists `at`, whatever it charges for reaching it.
pub(crate) fn lists(cells: &[ReachableCellNet], at: CellLevel) -> bool {
    let wanted = CellLevelNet::from_sim(at);
    cells.iter().any(|entry| entry.at == wanted)
}

/// Require the reply to list exactly `expected`, naming every cell and every cost that disagrees.
pub(crate) fn assert_same_cells(listed: &[ReachableCellNet], expected: &[(CellLevel, Tu)]) {
    let missing: Vec<CellLevel> = expected
        .iter()
        .filter(|(at, _)| !lists(listed, *at))
        .map(|(at, _)| *at)
        .collect();
    let extra: Vec<CellLevelNet> = listed
        .iter()
        .filter(|entry| !expected.iter().any(|(at, _)| entry.at.to_sim() == *at))
        .map(|entry| entry.at)
        .collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "the reply must name the cells the sim's own search reached: missing {missing:?}, \
         reported but not reached {extra:?}",
    );

    let wrong: Vec<(CellLevel, u8, u8)> = expected
        .iter()
        .filter_map(|(at, cost)| {
            let quoted = listed
                .iter()
                .find(|entry| entry.at.to_sim() == *at)
                .map(|entry| *entry.cost)?;
            (quoted != **cost).then_some((*at, **cost, quoted))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "each cell carries what the search charged for reaching it; these came back with \
         another number, as (cell, charged, reported): {wrong:?}",
    );

    let reported: Vec<CellLevel> = listed.iter().map(|entry| entry.at.to_sim()).collect();
    let searched: Vec<CellLevel> = expected.iter().map(|(at, _)| *at).collect();
    assert_eq!(
        reported, searched,
        "the reply keeps the order the search returned, neither re-sorted nor de-duplicated",
    );
}

/// Every cell `actor` reaches on the picture the screen has drawn, in the search's own order.
pub(crate) fn reachable_on_screen(app: &App, actor: Entity) -> Option<Vec<(CellLevel, Tu)>> {
    let (start, budget) = drawn_start_and_budget(app, actor)?;
    reachable_from(app, actor, start, budget)
}

/// Where the screen draws `actor` and the TU pool it is showing for it.
pub(crate) fn drawn_start_and_budget(app: &App, actor: Entity) -> Option<(CellLevel, Tu)> {
    let row = app.world().get_entity(actor).ok()?;
    let start = row
        .get::<DrawnPosition>()
        .map_or(**row.get::<Position>()?, |drawn| *drawn.position());
    let budget = row
        .get::<DrawnVitals>()
        .map_or(*row.get::<Tu>()?, DrawnVitals::tu);
    Some((start, budget))
}

/// Every cell `actor` reaches from `start` on `budget`, walking the screen's grid and fog.
pub(crate) fn reachable_from(
    app: &App,
    actor: Entity,
    start: CellLevel,
    budget: Tu,
) -> Option<Vec<(CellLevel, Tu)>> {
    let world = app.world();
    let row = world.get_entity(actor).ok()?;
    if row
        .get::<Mounted>()
        .is_some_and(|riding| riding.emplacement().is_some())
    {
        return None;
    }
    let mover = *row.get::<Faction>()?;
    let factor = row.get::<InflictedInjuries>().map_or(
        MovementCostFactor::IDENTITY,
        InflictedInjuries::movement_cost_factor,
    );
    let grid: &OccupancyGrid = world.get_resource::<ShownOccupancyGrid>()?;
    let fog = world
        .get_resource::<ShownSquadVisibility>()
        .map(ShownSquadVisibility::visibility)?;
    let planning = planning_view(world, fog, mover);
    Some(reachable_within(
        &Departure::anywhere(start),
        budget,
        DismountSurcharge::NONE,
        MoveGrids {
            occupancy:   grid,
            links:       world.get_resource::<VerticalLinkGraph>()?,
            floor_costs: world.get_resource::<FloorCostGrid>()?,
            tuning:      world.get_resource::<CombatTuning>()?,
        },
        factor,
        &planning,
    ))
}

fn planning_view<'a>(
    world: &'a World,
    fog: &'a SquadVisibility,
    mover: Faction,
) -> PlanningView<'a, impl Fn(Entity) -> FactionRelation + 'a> {
    PlanningView::new(fog, move |occupant| {
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
