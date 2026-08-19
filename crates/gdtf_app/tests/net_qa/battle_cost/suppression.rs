//! What the suppression gate does to a walk quote: refused when the route cannot break away,
//! quoted legal when it moves away from the suppressor and ends behind cover.

use bevy::{app::App, ecs::entity::Entity};
use gdtf_app::qa_wire::{
    cell::CellLevelNet,
    cost::{CostActNet, CostRefusalNet},
};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::TerrainPieceKind,
    ganger::{Direction, Position, Suppressed, SuppressorCell},
    prelude::{Cell, CellLevel},
};

use super::support::{a_reachable_cell, a_reachable_cell_where, cost_calls, first_body, settle};
use crate::{
    battle_reads::a_player_ganger,
    command_exchange::exchange_in_battle,
    socket_support::{TestError, TestResult},
};

/// What the case lined up: the mover, and the routable cell it asked to walk to.
struct Pinned {
    actor: Entity,
    dest:  CellLevel,
}

/// What the allow case lined up: the mover, its route, and the cell it seeded cover on.
struct Freed {
    actor:  Entity,
    dest:   CellLevel,
    behind: CellLevel,
}

/// Where the actor stands right now.
fn standing_at(app: &App, actor: Entity) -> Option<CellLevel> {
    Some(**app.world().get_entity(actor).ok()?.get::<Position>()?)
}

/// A cell past `dest` along the way there, so `dest` is no farther from it than `start`.
///
/// The gate then refuses on geometry alone, whatever the generated map put cover on.
fn beyond(start: CellLevel, dest: CellLevel) -> CellLevel {
    let (from, to) = (start.cell(), dest.cell());
    CellLevel::new(
        Cell::new(to.x + (to.x - from.x), to.y + (to.y - from.y)),
        dest.level(),
    )
}

/// `dest` mirrored through `start`, so `dest` is twice as far from it as `start` is.
fn mirrored(start: CellLevel, dest: CellLevel) -> CellLevel {
    let (from, to) = (start.cell(), dest.cell());
    CellLevel::new(
        Cell::new(from.x + (from.x - to.x), from.y + (from.y - to.y)),
        start.level(),
    )
}

/// The cell one step from `dest` toward `suppressor`, which is the one the gate reads cover on.
fn toward_suppressor(dest: CellLevel, suppressor: CellLevel) -> Option<CellLevel> {
    let step = Direction::from_cells(dest.cell(), suppressor.cell())?.cell_step();
    Some(CellLevel::new(
        Cell::new(dest.cell().x + step.x, dest.cell().y + step.y),
        dest.level(),
    ))
}

/// Waist-high cover with no armour, which is all the gate needs to find at a cell.
const fn a_cover_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::Mid,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
        TerrainPieceKind::Cover,
    )
}

/// Suppress the actor from a cell the walk to `dest` cannot break away from.
fn pin(app: &mut App, actor: Entity, dest: CellLevel) -> Option<Pinned> {
    let start = standing_at(app, actor)?;
    app.world_mut()
        .get_entity_mut(actor)
        .ok()?
        .insert(Suppressed::new(SuppressorCell::new(beyond(start, dest))));
    Some(Pinned { actor, dest })
}

/// Seed cover behind `dest`, then suppress the actor from the far side of where it stands.
fn free(app: &mut App, actor: Entity, start: CellLevel, dest: CellLevel) -> Option<Freed> {
    let suppressor = mirrored(start, dest);
    let behind = toward_suppressor(dest, suppressor)?;
    app.world_mut()
        .get_resource_mut::<CoverLedger>()?
        .insert(behind, a_cover_entry());
    app.world_mut()
        .get_entity_mut(actor)
        .ok()?
        .insert(Suppressed::new(SuppressorCell::new(suppressor)));
    Some(Freed {
        actor,
        dest,
        behind,
    })
}

#[test]
fn a_suppressed_mover_is_refused_suppressed_rather_than_quoted_a_legal_walk() -> TestResult {
    let mut planned: Option<Pinned> = None;
    let (app, replies) = exchange_in_battle(|app: &mut App| {
        settle(app);
        let Some((actor, _at)) = a_player_ganger(app) else {
            return Vec::new();
        };
        let Some(dest) = a_reachable_cell(app, actor) else {
            return Vec::new();
        };
        let Some(pinned) = pin(app, actor, dest) else {
            return Vec::new();
        };
        let calls = cost_calls(
            pinned.actor,
            &[CostActNet::Move {
                dest: CellLevelNet::from_sim(pinned.dest),
            }],
        );
        planned = Some(pinned);
        calls
    })?;
    let Some(pinned) = planned else {
        return Err(TestError::from(
            "a running battle must field a player ganger with a cell it can walk to",
        ));
    };
    assert!(
        app.world()
            .get_entity(pinned.actor)
            .is_ok_and(|row| row.contains::<Suppressed>()),
        "the mover must still be suppressed when the reply landed, or the case proves nothing",
    );

    let body = first_body(replies)?;
    assert_eq!(
        body.refusal,
        Some(CostRefusalNet::Suppressed),
        "a walk the suppression gate turns down must be refused Suppressed, not priced legal: \
         {body:?}",
    );
    assert!(
        !*body.legal,
        "a suppressed mover's walk is not legal: {body:?}",
    );
    Ok(())
}

#[test]
fn a_suppressed_mover_breaking_away_behind_cover_is_quoted_a_legal_walk() -> TestResult {
    let mut planned: Option<Freed> = None;
    let (app, replies) = exchange_in_battle(|app: &mut App| {
        settle(app);
        let Some((actor, _at)) = a_player_ganger(app) else {
            return Vec::new();
        };
        let Some(start) = standing_at(app, actor) else {
            return Vec::new();
        };
        let Some(dest) = a_reachable_cell_where(app, actor, |cell| cell.cell() != start.cell())
        else {
            return Vec::new();
        };
        let Some(freed) = free(app, actor, start, dest) else {
            return Vec::new();
        };
        let calls = cost_calls(
            freed.actor,
            &[CostActNet::Move {
                dest: CellLevelNet::from_sim(freed.dest),
            }],
        );
        planned = Some(freed);
        calls
    })?;
    let Some(freed) = planned else {
        return Err(TestError::from(
            "a running battle must field a player ganger with another cell it can walk to",
        ));
    };
    assert!(
        app.world()
            .get_entity(freed.actor)
            .is_ok_and(|row| row.contains::<Suppressed>()),
        "the mover must still be suppressed when the reply landed, or the case proves nothing",
    );
    assert!(
        app.world()
            .get_resource::<CoverLedger>()
            .is_some_and(|ledger| ledger.peek(&freed.behind).is_some()),
        "the seeded cover must still be in the ledger when the reply landed, or the allow branch \
         was decided by something else",
    );

    let body = first_body(replies)?;
    assert_eq!(
        body.refusal, None,
        "a suppressed mover moving away from the suppressor and ending behind cover is not \
         refused: {body:?}",
    );
    assert!(
        *body.legal,
        "a walk the suppression gate allows must be quoted legal: {body:?}",
    );
    Ok(())
}
