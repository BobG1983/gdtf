//! Live battles whose first player ganger is one TU short of the act the case asks about.

use bevy::{app::App, ecs::entity::Entity};
use cobalt_mcp_protocol::ports::McpPort;
use gdtf_battle_sim::{
    acts::{melee_tu_cost, shove_tu_cost},
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::TerrainPieceKind,
    ganger::{Hp, Position, Stance, StanceKind, Tu, Wounds},
    occupancy::{OccupancyGrid, TerrainKind},
    posture::stance_tu_cost,
    prelude::{Cell, CellLevel},
    tuning::CombatTuning,
    weapon::{FightMode, MeleeWeapon, Wields},
};
use gdtf_game::qa_wire::cell::CellLevelNet;

use crate::mcp::{
    battle_cost::support::settle,
    battle_reads::{a_player_ganger, an_enemy_ganger, one_step_from, two_steps_from},
    socket_support::{TestError, battle_app_listening},
};

/// Who acts, what its pool was cut to, and what one attempt at the act costs.
pub(super) struct Broke {
    pub(super) actor: Entity,
    pub(super) pool:  Tu,
    pub(super) cost:  Tu,
}

/// Put a ganger's pool at `pool`, and report it back.
pub(super) fn set_pool(app: &mut App, actor: Entity, pool: Tu) -> Result<Tu, TestError> {
    let Ok(mut row) = app.world_mut().get_entity_mut(actor) else {
        return Err("the ganger the world just answered with must still exist".into());
    };
    row.insert(pool);
    Ok(pool)
}

/// What a ganger's pool holds right now.
pub(super) fn pool_of(app: &App, actor: Entity) -> Option<Tu> {
    app.world().get::<Tu>(actor).copied()
}

/// How a ganger stands right now.
pub(super) fn stance_of(app: &App, actor: Entity) -> Option<StanceKind> {
    app.world().get::<Stance>(actor).map(|stance| **stance)
}

/// Where a ganger stands right now.
pub(super) fn cell_of(app: &App, ganger: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(ganger).map(|at| **at)
}

/// A ganger's hit points and wound count, so a case can say it took no harm.
pub(super) fn vitals_of(app: &App, ganger: Entity) -> (Option<Hp>, Option<Wounds>) {
    let world = app.world();
    (
        world.get::<Hp>(ganger).copied(),
        world.get::<Wounds>(ganger).copied(),
    )
}

/// A stance the ganger is not already in, so asking for it is a real change.
pub(super) const fn other_stance(held: StanceKind) -> StanceKind {
    match held {
        StanceKind::Prone => StanceKind::Standing,
        StanceKind::Standing | StanceKind::Crouching => StanceKind::Prone,
    }
}

/// Cut a ganger's pool to one short of `cost`, and report what it now holds.
fn cut_pool_below(app: &mut App, actor: Entity, cost: Tu) -> Result<Tu, TestError> {
    if *cost == 0 {
        return Err("the act must cost something, or a below-cost pool proves nothing".into());
    }
    set_pool(app, actor, Tu::new(*cost - 1))
}

/// What one strike with the melee weapon this ganger holds charges.
fn strike_cost(app: &App, actor: Entity) -> Option<Tu> {
    let world = app.world();
    let weapon = world
        .get::<Wields>(actor)?
        .melee_weapon(|entity| world.get::<MeleeWeapon>(entity).is_some())?;
    world.get::<FightMode>(weapon).map(melee_tu_cost)
}

/// The combat tuning the running battle was built with.
fn tuning_of(app: &App) -> Result<CombatTuning, TestError> {
    app.world()
        .get_resource::<CombatTuning>()
        .cloned()
        .ok_or_else(|| "a running battle carries combat tuning".into())
}

/// The first player ganger of a settled battle, with where it stands.
fn an_actor(app: &App) -> Result<(Entity, CellLevelNet), TestError> {
    a_player_ganger(app)
        .ok_or_else(|| "a running battle must field a ganger the player commands".into())
}

/// The cell halfway between `start` and the cell two steps along the same line.
fn between(start: CellLevel, landing: CellLevel) -> CellLevel {
    let (from, to) = (start.cell(), landing.cell());
    CellLevel::new(
        Cell::new(from.x + (to.x - from.x) / 2, from.y + (to.y - from.y) / 2),
        start.level(),
    )
}

/// A live battle whose first player ganger cannot afford to change stance.
pub(super) fn too_poor_to_change_stance() -> Result<(App, McpPort, (Broke, StanceKind)), TestError>
{
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let (actor, _at) = an_actor(&app)?;
    let cost = stance_tu_cost(&tuning_of(&app)?.stance_change_tu);
    let pool = cut_pool_below(&mut app, actor, cost)?;
    let Some(stance) = stance_of(&app, actor) else {
        return Err("a player ganger stands some way".into());
    };
    Ok((app, port, (Broke { actor, pool, cost }, stance)))
}

/// A live battle whose first player ganger cannot afford to shove the enemy beside it.
pub(super) fn too_poor_to_shove() -> Result<(App, McpPort, (Broke, Entity)), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let (actor, at) = an_actor(&app)?;
    let Some(enemy) = an_enemy_ganger(&app) else {
        return Err("a generated battle must field at least one living enemy".into());
    };
    let Some(second) = two_steps_from(&app, at) else {
        return Err("the generated map must offer two clear cells in a line from the actor".into());
    };
    let beside = between(at.to_sim(), second.to_sim());
    let Ok(mut row) = app.world_mut().get_entity_mut(enemy) else {
        return Err("the enemy the world just answered with must still exist".into());
    };
    row.insert(Position::new(beside));
    settle(&mut app);
    let cost = shove_tu_cost(&tuning_of(&app)?);
    let pool = cut_pool_below(&mut app, actor, cost)?;
    Ok((app, port, (Broke { actor, pool, cost }, enemy)))
}

/// Put a cover piece at `at`, in the ledger the smash depletes and the grid it stands on.
fn stand_cover_at(app: &mut App, at: CellLevel) -> Result<(), TestError> {
    let entry = CoverEntry::seeded(
        CoverHp::new(1_000),
        HeightBand::Mid,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
        TerrainPieceKind::Cover,
    );
    let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() else {
        return Err("a running battle carries a cover ledger".into());
    };
    ledger.insert(at, entry);
    let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() else {
        return Err("a running battle carries an occupancy grid".into());
    };
    grid.set_terrain(at, TerrainKind::Cover);
    Ok(())
}

/// A live battle whose first player ganger cannot afford to smash the cover beside it.
pub(super) fn too_poor_to_smash() -> Result<(App, McpPort, (Broke, CellLevel)), TestError> {
    let (mut app, port) = battle_app_listening()?;
    settle(&mut app);
    let (actor, at) = an_actor(&app)?;
    let Some(beside) = one_step_from(&app, at) else {
        return Err("the generated map must offer one clear cell beside the actor".into());
    };
    let beside = beside.to_sim();
    stand_cover_at(&mut app, beside)?;
    settle(&mut app);
    let Some(cost) = strike_cost(&app, actor) else {
        return Err("a player ganger must hold a melee weapon to strike with".into());
    };
    let pool = cut_pool_below(&mut app, actor, cost)?;
    Ok((app, port, (Broke { actor, pool, cost }, beside)))
}
