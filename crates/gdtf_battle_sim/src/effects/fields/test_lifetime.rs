//! GTW-659 — the `Turns(n)` LIFETIME pin on the REAL schedule: an authored
//! `Turns(n)` field drains its occupant on exactly `n` round boundaries and is
//! REMOVED by the same boundary that spends its last turn (never `n + 1` — the
//! drain-before-expire order inside one [`tick_fields`] run does not add a round
//! for a representable, positive `n`; the zero-turn state that DID lie by one is
//! unrepresentable since GTW-659). Driven through the live production wiring
//! (the `test_turn_start.rs` composition): [`BattleSimPlugin`]'s turn-cycle
//! engine + the `enemy_phase_started`-gated clock, one boundary per full round.

use bevy::prelude::{App, Entity, Messages, MinimalPlugins};

use super::{FieldDamage, FieldDef, FieldDuration, FieldRegistry, FieldTicked, ImmuneArmorTypes};
use crate::{
    acts::EndTurnRequested,
    armor::{ArmorType, WornBy},
    battle::{BattleInProgress, BattleRoster, BattleSimPlugin},
    ganger::{Faction, Hp, LifeState},
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    test_support::{GangerEntityBuilder, field_turns, full_vision, insert_sim_resources},
    turn::ActiveFaction,
    weapon::DamageType,
};

/// A fixed seed for the per-test RNG streams (arbitrary, not tuned).
const SEED: u64 = 0x659F_1E1D;

/// Gang `0` is the player team (the litany's `PlayerFaction` gang).
const PLAYER: Faction = Faction::new(0);
/// The enemy team — the faction whose `TurnStarted` fires the once-per-round clock.
const ENEMY: Faction = Faction::new(1);

/// The flat per-round drain the fixture field deals.
const PER_TURN: u16 = 3;
/// A pool deep enough to survive every round this suite ticks.
const START_HP: u16 = 100;

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A [`PER_TURN`]-damage field def with NO immune set and a finite `Turns(turns)`
/// lifetime (Chem flavour — the tick bypasses the matchup wheel).
fn finite_field(turns: u8) -> FieldDef {
    FieldDef::new(
        FieldDamage::new(PER_TURN),
        DamageType::Chem,
        ImmuneArmorTypes::new([]),
        FieldDuration::Turns(field_turns(turns)),
    )
}

/// Build the FULL live-runtime harness (the `test_turn_start.rs` composition):
/// [`MinimalPlugins`] + the real production [`BattleSimPlugin`] (the turn-cycle
/// engine and the GTW-658-pinned, `enemy_phase_started`-gated clock registration),
/// seeded with the canonical sim-resource litany — so these pins exercise the SAME
/// registration and cadence the live runtime uses, not a hand-rolled subset.
fn live_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(BattleSimPlugin);
    insert_sim_resources(&mut app, BattleSeed::new(SEED));
    app.insert_resource(full_vision());
    app.insert_resource(BattleRoster::new([PLAYER, ENEMY]));
    app.insert_resource(ActiveFaction::new(PLAYER));
    app.insert_resource(BattleInProgress);
    app
}

/// Stand one ENEMY ganger on `cell` (with the worn-armor relationship the occupant
/// query requires — one Plated piece, never immune to the fixture field) and give
/// the occupancy-maintenance chain two frames to project the spawn. Spawned WITHOUT
/// the AI's required row (no Stance/Facing/Aiming), so the enemy brain plans
/// nothing and the turn cycle is the only actor.
fn stand_ganger_on(app: &mut App, cell: CellLevel) -> Entity {
    let occupant = GangerEntityBuilder::new()
        .at(cell)
        .faction(ENEMY)
        .life_state(LifeState::Alive)
        .hp(START_HP)
        .tu(100)
        .tu_max(100)
        .spawn(app.world_mut());
    app.world_mut()
        .spawn((ArmorType::Plated, WornBy::new(occupant)));
    app.update();
    app.update();
    occupant
}

/// Cross ONE full-round boundary: end the player's turn (the enemy `TurnStarted`
/// lands this frame and the once-per-round clock fires), then run one more frame —
/// the enemy brain, with nothing to plan, emitted its OWN `EndTurnRequested` on the
/// boundary frame, so this frame hands control back to the player (a player
/// `TurnStarted` — no clock). Exactly one manual end-turn per round: a second one
/// would stack with the brain's and double-advance the cycle.
fn cross_one_round(app: &mut App) {
    app.world_mut().write_message(EndTurnRequested);
    app.update();
    app.update();
}

/// Read a spawned ganger's current Hp (returns `0` if absent, so no `unwrap`).
fn hp_of(app: &App, ganger: Entity) -> u16 {
    app.world().get::<Hp>(ganger).map_or(0, |h| **h)
}

/// The live field count in the registry (0 if the resource is absent).
fn field_count(app: &App) -> usize {
    app.world()
        .get_resource::<FieldRegistry>()
        .map_or(0, FieldRegistry::len)
}

/// Drain the buffered [`FieldTicked`] signals emitted so far, in order.
fn drain_field_ticks(app: &mut App) -> Vec<FieldTicked> {
    app.world_mut()
        .resource_mut::<Messages<FieldTicked>>()
        .drain()
        .collect()
}

/// GTW-659 (C4): `Turns(1)` = exactly ONE draining round — the first boundary
/// drains the occupant once and the SAME boundary's expiry step removes the
/// placement; the next boundary drains nothing (no second round, no `n + 1`).
#[test]
fn a_turns_one_field_drains_exactly_one_round_then_is_gone() {
    let cell = ground(10, 10);
    let mut app = live_app();
    let occupant = stand_ganger_on(&mut app, cell);

    let mut registry = FieldRegistry::new();
    registry.spawn(cell, finite_field(1));
    app.insert_resource(registry);
    assert!(
        drain_field_ticks(&mut app).is_empty(),
        "no FieldTicked may be emitted before a round boundary",
    );

    // Round 1 — the one and only draining round: one tick, and the placement is
    // already GONE (the boundary that spends the last turn removes it).
    cross_one_round(&mut app);
    let ticks = drain_field_ticks(&mut app);
    assert_eq!(
        ticks.len(),
        1,
        "Turns(1): the first round boundary drains exactly once: {ticks:?}",
    );
    assert_eq!(
        hp_of(&app, occupant),
        START_HP - PER_TURN,
        "Turns(1): exactly one per-turn drain lands",
    );
    assert_eq!(
        field_count(&app),
        0,
        "Turns(1): the boundary that spends the last turn REMOVES the placement",
    );

    // Round 2 — the field is gone: no further tick, no further drain.
    cross_one_round(&mut app);
    assert!(
        drain_field_ticks(&mut app).is_empty(),
        "Turns(1) must NOT drain a second round (the authored count is the round count)",
    );
    assert_eq!(
        hp_of(&app, occupant),
        START_HP - PER_TURN,
        "Turns(1): the total drain stays one per-turn amount",
    );
}

/// GTW-659 (C4): `Turns(2)` = exactly TWO draining rounds — present after the
/// first boundary, removed by the second, silent on the third.
#[test]
fn a_turns_two_field_drains_exactly_two_rounds_then_is_gone() {
    let cell = ground(12, 12);
    let mut app = live_app();
    let occupant = stand_ganger_on(&mut app, cell);

    let mut registry = FieldRegistry::new();
    registry.spawn(cell, finite_field(2));
    app.insert_resource(registry);

    // Round 1 — first drain; one turn remains, so the placement survives.
    cross_one_round(&mut app);
    assert_eq!(
        drain_field_ticks(&mut app).len(),
        1,
        "Turns(2): the first round boundary drains once",
    );
    assert_eq!(
        field_count(&app),
        1,
        "Turns(2): one turn remains after the first round — the field is still live",
    );

    // Round 2 — second drain; the same boundary spends the last turn and removes it.
    cross_one_round(&mut app);
    assert_eq!(
        drain_field_ticks(&mut app).len(),
        1,
        "Turns(2): the second round boundary drains once more",
    );
    assert_eq!(
        hp_of(&app, occupant),
        START_HP - 2 * PER_TURN,
        "Turns(2): exactly two per-turn drains land in total",
    );
    assert_eq!(
        field_count(&app),
        0,
        "Turns(2): the second boundary REMOVES the placement",
    );

    // Round 3 — nothing left: no third drain (n, never n + 1).
    cross_one_round(&mut app);
    assert!(
        drain_field_ticks(&mut app).is_empty(),
        "Turns(2) must NOT drain a third round (the authored count is the round count)",
    );
    assert_eq!(
        hp_of(&app, occupant),
        START_HP - 2 * PER_TURN,
        "Turns(2): the total drain stays two per-turn amounts",
    );
}
