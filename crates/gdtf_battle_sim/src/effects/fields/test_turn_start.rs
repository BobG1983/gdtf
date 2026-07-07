//! GTW-658 — the boundary-frame DETERMINISM pin for the GTW-545 field clock (the
//! GTW-641 `tick_bleed` precedent, the `effects/bleed/test/turn_start.rs` shape): on
//! the boundary frame, with a same-frame enemy act in flight, the clock's effect
//! resolves BEFORE the act's — `tick_fields` is pinned `.before` the act dispatchers
//! in `acts/plugin/turn_clocks.rs` (clocks resolve AT the boundary, before the new
//! turn's act resolution). Without the pin the interleaving is a scheduler lottery
//! (bevy-traps.md #3); this test makes a lost lottery a red, not a flake.

use bevy::prelude::{App, Entity, Messages, MinimalPlugins};

use super::{FieldDamage, FieldDef, FieldDuration, FieldRegistry, FieldTicked, ImmuneArmorTypes};
use crate::{
    acts::{EndTurnRequested, MoveRejected, MoveRequested, MovementOccurred},
    armor::{ArmorType, WornBy},
    battle::{BattleInProgress, BattleRoster, BattleSimPlugin},
    ganger::{Faction, Hp, LifeState, Position},
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    test_support::{GangerEntityBuilder, full_vision, insert_sim_resources},
    turn::ActiveFaction,
    weapon::DamageType,
};

/// A fixed seed for the per-test RNG streams (arbitrary, not tuned).
const SEED: u64 = 0x5A1C_AC75;

/// Gang `0` is the player team (the litany's `PlayerFaction` gang).
const PLAYER: Faction = Faction::new(0);
/// The enemy team — the boundary frame's incoming (acting) faction.
const ENEMY: Faction = Faction::new(1);

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A field def of `damage` per turn with NO immune set, Permanent (Chem flavour —
/// the tick bypasses the matchup wheel, so the type is irrelevant to the drain).
fn field(damage: u16) -> FieldDef {
    FieldDef::new(
        FieldDamage::new(damage),
        DamageType::Chem,
        ImmuneArmorTypes::new([]),
        FieldDuration::Permanent,
    )
}

/// Build the FULL live-runtime harness (the bleed turn-start pin's composition):
/// [`MinimalPlugins`] + the real production [`BattleSimPlugin`] (the turn-cycle
/// engine, the dispatchers, and the GTW-658-pinned clock registration), seeded with
/// the canonical sim-resource litany, the [`BattleRoster`] census read, the player
/// holding [`ActiveFaction`], and the [`BattleInProgress`] gate witness — so this
/// test exercises the SAME registration the live runtime uses, not a hand-rolled
/// subset.
fn live_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(BattleSimPlugin);
    insert_sim_resources(&mut app, BattleSeed::new(SEED));
    app.insert_resource(BattleRoster::new([PLAYER, ENEMY]));
    app.insert_resource(ActiveFaction::new(PLAYER));
    app.insert_resource(BattleInProgress);
    app
}

/// Send one End-Turn request and run ONE update — the frame the enemy-phase
/// `TurnStarted` boundary lands on (the frame under test).
fn end_turn(app: &mut App) {
    app.world_mut().write_message(EndTurnRequested);
    app.update();
}

/// Read a spawned ganger's current Hp (returns `0` if absent, so no `unwrap`).
fn hp_of(app: &App, ganger: Entity) -> u16 {
    app.world().get::<Hp>(ganger).map_or(0, |h| **h)
}

/// Read a spawned ganger's current `LifeState` (defaults Alive if absent).
fn life_of(app: &App, ganger: Entity) -> LifeState {
    app.world()
        .get::<LifeState>(ganger)
        .copied()
        .unwrap_or(LifeState::Alive)
}

/// Drain the buffered [`FieldTicked`] signals emitted so far, in order.
fn drain_field_ticks(app: &mut App) -> Vec<FieldTicked> {
    app.world_mut()
        .resource_mut::<Messages<FieldTicked>>()
        .drain()
        .collect()
}

/// Drain the buffered [`MovementOccurred`] signals emitted so far.
fn drain_movements(app: &mut App) -> Vec<MovementOccurred> {
    app.world_mut()
        .resource_mut::<Messages<MovementOccurred>>()
        .drain()
        .collect()
}

/// Drain the buffered [`MoveRejected`] signals emitted so far — the vacuousness
/// guard: an empty drain proves the in-flight move passed every dispatch gate
/// (reachable + affordable), so the ONLY thing that stopped the step was the
/// boundary kill the clock resolved first.
fn drain_rejects(app: &mut App) -> Vec<MoveRejected> {
    app.world_mut()
        .resource_mut::<Messages<MoveRejected>>()
        .drain()
        .collect()
}

/// GTW-658 — the field clock's effect resolves BEFORE a same-frame enemy act's: on
/// the boundary frame an enemy mover stands on a LETHAL field cell with a real
/// one-cell [`MoveRequested`] off it in flight. The boundary tick drains the field
/// FIRST (one [`FieldTicked`], Hp → 0, Dead), so the same-frame act resolves against
/// the clock's outcome: the walk's fail-closed life gate drops it — NO step, NO
/// [`MovementOccurred`], the mover dies ON the field cell. An unpinned schedule that
/// ran the act first would step the mover off before it fell — the nondeterministic
/// interleaving this pin outlaws. The request itself is REJECT-FREE (no
/// [`MoveRejected`]), proving the pin (not a dispatch gate) is what held the mover.
#[test]
fn the_boundary_field_tick_resolves_before_a_same_frame_enemy_act() {
    let per_turn = 10u16;
    let start_hp = per_turn - 2; // LETHAL — the boundary tick empties the pool
    let field_cell = ground(10, 10);
    let dest = ground(11, 10);

    let mut app = live_app();
    // The route gate reads the squad fog — full vision so the move resolves on
    // geometry + occupancy alone (the acts-suite precondition).
    app.insert_resource(full_vision());

    // The fielded ENEMY mover — the boundary frame's actor. Spawned WITHOUT the
    // AI's required row (no Stance/Facing/Aiming), so the enemy brain plans nothing
    // for it and the hand-written request below is the ONLY enemy act in flight.
    let mover = GangerEntityBuilder::new()
        .at(field_cell)
        .faction(ENEMY)
        .life_state(LifeState::Alive)
        .hp(start_hp)
        .tu(100)
        .tu_max(100)
        .spawn(app.world_mut());
    // The occupant query requires a worn-armor relationship — one Plated piece,
    // NOT in the field's (empty) immune set, so the drain applies.
    app.world_mut()
        .spawn((ArmorType::Plated, WornBy::new(mover)));

    // The lethal field under the mover's feet.
    let mut registry = FieldRegistry::new();
    registry.spawn(field_cell, field(per_turn));
    app.insert_resource(registry);

    // Mid-turn frames settle occupancy (the maintenance chain projects the spawn) —
    // and the clock must NOT tick without a turn boundary.
    app.update();
    app.update();
    assert_eq!(
        hp_of(&app, mover),
        start_hp,
        "plain mid-turn frames must not tick the field clock (no turn boundary crossed)",
    );
    assert!(
        drain_field_ticks(&mut app).is_empty(),
        "no FieldTicked may be emitted mid-turn",
    );

    // The same-frame enemy act in flight: a one-cell move OFF the field, dispatched
    // on the SAME frame the enemy TurnStarted lands.
    app.world_mut()
        .write_message(MoveRequested::new(mover, dest));
    end_turn(&mut app);

    // THE PIN — the clock's effect resolved BEFORE the act's: the lethal boundary
    // tick killed the mover on the field cell, so the same-frame walk was dropped
    // by its fail-closed life gate — no step was ever taken.
    let ticks = drain_field_ticks(&mut app);
    assert_eq!(
        ticks.len(),
        1,
        "the boundary frame ticks the field clock exactly once: {ticks:?}",
    );
    assert!(
        ticks.iter().all(|tick| tick.at == field_cell),
        "the boundary FieldTicked lands at the field cell: {ticks:?}",
    );
    assert_eq!(hp_of(&app, mover), 0, "the lethal tick empties the Hp pool");
    assert_eq!(
        life_of(&app, mover),
        LifeState::Dead,
        "a field drain that empties HP KILLS (the GTW-544 DOT-kills precedent)",
    );
    let movements = drain_movements(&mut app);
    assert!(
        movements.is_empty(),
        "the same-frame act resolves AFTER the clock: the boundary kill means NO \
         step and NO MovementOccurred (GTW-658): {movements:?}",
    );
    let at = app.world().get::<Position>(mover).map(|p| **p);
    assert_eq!(
        at,
        Some(field_cell),
        "the mover dies ON the field cell — it never stepped off",
    );

    // Non-vacuousness: the in-flight move passed EVERY dispatch gate (no typed
    // reject) — the route was reachable and affordable, so only the pinned
    // clock-before-act order held the mover in place.
    let rejects = drain_rejects(&mut app);
    assert!(
        rejects.is_empty(),
        "the in-flight move must pass the dispatch gates (reachable + affordable) — \
         a reject would make this pin vacuous: {rejects:?}",
    );
}
