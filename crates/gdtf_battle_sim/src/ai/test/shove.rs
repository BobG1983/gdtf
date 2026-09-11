//! The shove arm: over an edge only, never across flat ground, never ahead of a swing, and
//! never when the pool cannot pay for it.

use super::support::*;
use crate::{
    acts::{ShoveRequested, ShoveSource, shove_tu_cost},
    falls::FallOccurred,
    surface::{SlabState, SurfaceGrid},
    tuning::CombatTuning,
};

// The shover, the shoved, and the app they act in.
struct Scene {
    app:    App,
    enemy:  Entity,
    player: Entity,
}

// The frame each request was drained on, so precedence can be read off the run.
struct LedgeDrive {
    melees: Vec<(usize, MeleeRequested)>,
    shoves: Vec<(usize, ShoveRequested)>,
}

impl LedgeDrive {
    // Earliest frame this pair's swing was drained on.
    fn first_melee(&self, attacker: Entity, target: Entity) -> Option<usize> {
        let swing = MeleeRequested::new(attacker, target);
        self.melees
            .iter()
            .filter(|(_, request)| *request == swing)
            .map(|(frame, _)| *frame)
            .min()
    }

    // Frames a deliberate shove was drained on, weapon-tag shoves left out.
    fn deliberate_shove_frames(&self) -> Vec<usize> {
        self.shoves
            .iter()
            .filter(|(_, request)| request.source == ShoveSource::Deliberate)
            .map(|(frame, _)| *frame)
            .collect()
    }
}

fn upper(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

fn shove_cost(app: &App) -> u8 {
    *shove_tu_cost(app.world().resource::<CombatTuning>())
}

fn stand_a_slab(app: &mut App, at: CellLevel) {
    if let Some(mut surface) = app.world_mut().get_resource_mut::<SurfaceGrid>() {
        surface.set_slab(at, SlabState::Present);
    }
}

// A ganger wielding nothing at all, so the aim, fire, reload and melee arms all answer None.
fn spawn_unarmed(
    world: &mut World,
    at: CellLevel,
    faction: Faction,
    facing: Direction,
    tu: u8,
) -> Entity {
    world
        .spawn((
            Position::new(at),
            Facing::new(facing),
            Stance::new(StanceKind::Standing),
            Aiming::new(false),
            Shooting::new(1.0),
            Fight::new(1.0),
            Tu::new(tu),
            TuMax::new(tu),
            faction,
            (
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id()
}

// Enemy and player side by side, with the enemy's pool quoted against the shove's own cost.
fn two_abreast(enemy_at: CellLevel, player_at: CellLevel, pool_of: impl Fn(u8) -> u8) -> Scene {
    let mut app = brain_app();
    let pool = pool_of(shove_cost(&app));
    let enemy = spawn_unarmed(app.world_mut(), enemy_at, ENEMY, Direction::East, pool);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 2);
    place_occupant(&mut app, enemy_at, enemy);
    place_occupant(&mut app, player_at, player);
    Scene { app, enemy, player }
}

// Slabs under both combatants at storey 2, and no slab east of the player to land on.
fn ledge_scene(pool_of: impl Fn(u8) -> u8) -> Scene {
    let enemy_at = upper(5, 5, 2);
    let player_at = upper(6, 5, 2);
    let mut scene = two_abreast(enemy_at, player_at, pool_of);
    stand_a_slab(&mut scene.app, enemy_at);
    stand_a_slab(&mut scene.app, player_at);
    scene
}

fn drain_shoves(app: &mut App) -> Vec<ShoveRequested> {
    app.world_mut()
        .resource_mut::<Messages<ShoveRequested>>()
        .drain()
        .collect()
}

fn drain_falls(app: &mut App) -> Vec<FallOccurred> {
    app.world_mut()
        .resource_mut::<Messages<FallOccurred>>()
        .drain()
        .collect()
}

fn drive_shoves(app: &mut App) -> Vec<ShoveRequested> {
    let mut shoves = Vec::new();
    drive_until_player_turn(app, |app| shoves.extend(drain_shoves(app)));
    shoves
}

fn drive_frames(app: &mut App) -> LedgeDrive {
    let mut drive = LedgeDrive {
        melees: Vec::new(),
        shoves: Vec::new(),
    };
    let mut frame = 0_usize;
    drive_until_player_turn(app, |app| {
        drive.melees.extend(
            drain_melees(app)
                .into_iter()
                .map(|request| (frame, request)),
        );
        drive.shoves.extend(
            drain_shoves(app)
                .into_iter()
                .map(|request| (frame, request)),
        );
        frame = frame.saturating_add(1);
    });
    drive
}

fn level_of(app: &App, ganger: Entity) -> Option<Level> {
    app.world().get::<Position>(ganger).map(|at| at.level())
}

fn position_of(app: &App, ganger: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(ganger).map(|at| **at)
}

fn deliberate(shoves: &[ShoveRequested]) -> Vec<ShoveRequested> {
    shoves
        .iter()
        .copied()
        .filter(|request| request.source == ShoveSource::Deliberate)
        .collect()
}

#[test]
fn an_enemy_on_a_ledge_shoves_the_adjacent_player_over_the_edge() {
    let Scene {
        mut app,
        enemy,
        player,
    } = ledge_scene(|cost| cost);

    let mut shoves = Vec::new();
    let mut falls = Vec::new();
    drive_until_player_turn(&mut app, |app| {
        shoves.extend(drain_shoves(app));
        falls.extend(drain_falls(app));
    });

    assert_eq!(
        deliberate(&shoves),
        vec![ShoveRequested::new(enemy, player)],
        "an enemy beside a player on a ledge must ask for exactly one deliberate shove: \
         {shoves:?}",
    );
    assert!(
        level_of(&app, player).is_some_and(|level| *level < 2),
        "the real dispatch must drop the shoved player below the ledge: {:?}",
        position_of(&app, player),
    );
    assert!(
        falls.iter().any(|fall| fall.ganger == player),
        "the shove must resolve as a fall for the player: {falls:?}",
    );
}

#[test]
fn an_enemy_on_flat_ground_never_shoves_the_adjacent_player() {
    let Scene { mut app, .. } = two_abreast(ground(5, 5), ground(6, 5), |cost| cost);

    let shoves = drive_shoves(&mut app);

    assert!(
        deliberate(&shoves).is_empty(),
        "a push across flat ground only slides the target one cell, so the arm must refuse it \
         even though the pool covers the cost: {shoves:?}",
    );
}

#[test]
fn a_melee_armed_enemy_swings_before_it_shoves() {
    let Scene {
        mut app,
        enemy,
        player,
    } = ledge_scene(|cost| cost.saturating_add(STRIKE_TU));
    give_melee(app.world_mut(), enemy);

    let drive = drive_frames(&mut app);

    let swung_on = drive.first_melee(enemy, player);
    assert!(
        drive
            .deliberate_shove_frames()
            .iter()
            .all(|frame| swung_on.is_some_and(|swing| *frame > swing)),
        "melee is preferred over shove, so no deliberate shove may land on or before the first \
         swing's frame: swing {swung_on:?}, shoves {:?}",
        drive.shoves,
    );
    assert!(
        swung_on.is_some(),
        "a melee-armed enemy beside the player must swing: {:?}",
        drive.melees,
    );
}

#[test]
fn an_enemy_that_cannot_pay_for_the_shove_asks_for_nothing_and_the_turn_ends() {
    let Scene {
        mut app, player, ..
    } = ledge_scene(|cost| cost.saturating_sub(1));
    let started_at = position_of(&app, player);

    let shoves = drive_shoves(&mut app);

    assert!(
        deliberate(&shoves).is_empty(),
        "a shove the pool cannot cover must never be requested: {shoves:?}",
    );
    assert_eq!(
        position_of(&app, player),
        started_at,
        "nothing may move the player when the shove is refused",
    );
}
