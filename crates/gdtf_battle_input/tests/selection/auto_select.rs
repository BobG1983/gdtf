//! Deterministic auto-select of the first player ganger (GTW-255).

use bevy::prelude::*;
use gdtf_battle_input::{GdtfBattleInputPlugin, SelectedShooter};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::LifeState,
    prelude::{Cell, CellLevel, Faction, Level},
    test_support::GangerEntityBuilder,
};

use super::harness::*;

// ---------------------------------------------------------------------------------
// GTW-255 — auto-select the deterministic first player ganger at battle start.
// ---------------------------------------------------------------------------------

/// Spawns a ganger with a `Faction` + a `Position` at `cell` (the components the GTW-255
/// `auto_select_first_player_ganger` query reads) and returns its entity. Distinct from
/// `place_player_ganger` (which seeds occupancy for the click path) — the auto-select
/// reasons off `Position`, not the occupancy grid.
fn placed_ganger(app: &mut App, faction: Faction, cell: CellLevel) -> Entity {
    GangerEntityBuilder::new()
        .faction(faction)
        .at(cell)
        .spawn(app.world_mut())
}

/// AC1 — with no selection + player gangers present, one update auto-selects the
/// deterministic player ganger (the lowest `(level, y, x)` `Position` cell), and it is a
/// PLAYER-faction entity, never the enemy. Two player gangers at distinct cells + one
/// enemy: the lower-ordered player cell wins.
///
/// PIN (cell-ordering vs Entity-id AND spawn/iteration order). The cell-winner `mid` is
/// deliberately the MIDDLE spawn, so it is NEITHER the first found by query iteration NOR
/// the `Entity`-`Ord` minimum:
///
/// - `(level, y, x)` cell ordering → picks `mid` (cell `(x=1, y=1)` → key `(0, 1, 1)`,
///   below `first`'s `(0, 3, 2)` and `last`'s `(0, 9, 9)`). y=1 < y=3 also pins
///   y-before-x (`Cell::new(x, y)`).
/// - A forbidden first-found `.next()` / spawn-order pick → `first` (the first-spawned,
///   first in iteration) ≠ `mid` → FAILS.
/// - A forbidden `min_by_key(entity)` (Entity-id order, explicitly forbidden by the
///   ticket) → the `Entity`-`Ord` minimum, which in Bevy 0.18 is NOT the lowest spawn
///   index (`Entity::cmp` is opaque, not allocation-monotonic — exactly why the contract
///   forbids it). Empirically the minimum here is `last`, ≠ `mid` → FAILS.
///
/// So only the `(level, y, x)` cell ordering passes; both forbidden interpretations fail.
#[test]
fn auto_selects_deterministic_first_player_ganger() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    // First spawned (first in query iteration) — higher cell `(x=2, y=3)` → key `(0,3,2)`.
    let first = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(2, 3), level),
    );
    // An enemy at the globally-lowest cell — must NEVER be auto-selected.
    let enemy = placed_ganger(
        &mut app,
        ENEMY_FACTION,
        CellLevel::new(Cell::new(0, 0), level),
    );
    // The deterministic cell-winner — lowest player `(level, y, x)` cell `(x=1, y=1)` →
    // key `(0,1,1)`. Spawned in the MIDDLE: not first-found, not the Entity-Ord min.
    let mid = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(1, 1), level),
    );
    // Last spawned — highest cell `(x=9, y=9)` → key `(0,9,9)`; in Bevy 0.18 this is the
    // `Entity`-`Ord` minimum, so it is what a forbidden `min_by_key(entity)` would pick.
    let _last = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(9, 9), level),
    );

    assert_eq!(
        selected(&app),
        None,
        "precondition: nothing selected at start"
    );

    app.update();

    assert_eq!(
        selected(&app),
        Some(mid),
        "the lowest-Position player ganger by (level, y, x) must be auto-selected — by \
         cell ordering, NOT Entity id (forbidden pick = `_last`) NOR spawn/iteration \
         order (forbidden pick = `first`)",
    );
    assert_ne!(
        selected(&app),
        Some(first),
        "a spawn/iteration-order pick (first-spawned) must NOT win — only cell ordering",
    );
    assert_ne!(
        selected(&app),
        Some(enemy),
        "the enemy ganger (even at the lowest cell) must NEVER be auto-selected",
    );
}

/// AC1 (ordering, level-major) — the `(level, y, x)` ordering is LEVEL-major: a player
/// ganger on a lower storey wins over one with a smaller `(y, x)` on a higher storey.
///
/// PIN (cell-ordering vs Entity-id AND spawn/iteration order). The lower-storey cell-
/// winner is spawned in the MIDDLE, so it is NEITHER the first found NOR the `Entity`-`Ord`
/// minimum:
///
/// - level-major cell ordering → `lower_storey` (storey 0 beats both storey-2 and
///   storey-3 despite its larger `(y, x)`).
/// - forbidden first-found / spawn order → `high_first` (storey 2, spawned first) → FAILS.
/// - forbidden `min_by_key(entity)` → the `Entity`-`Ord` minimum, which in Bevy 0.18 is
///   the last-spawned `high_last` (storey 3), not the cell-winner → FAILS.
#[test]
fn auto_select_orders_level_major() {
    let mut app = selection_app(Level::new(0));

    // Higher storey, smaller (y, x) — spawned FIRST / first in iteration, must LOSE.
    let _high_first = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(0, 0), Level::new(2)),
    );
    // Lower storey, larger (y, x) — spawned MIDDLE (neither first-found nor Entity-Ord
    // min), must WIN on the level-major ordering.
    let lower_storey = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(50, 50), Level::new(0)),
    );
    // Higher storey still — spawned LAST, so in Bevy 0.18 it is the `Entity`-`Ord` minimum
    // (what a forbidden `min_by_key(entity)` would pick), yet must LOSE on storey.
    let _high_last = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(0, 0), Level::new(3)),
    );

    app.update();

    assert_eq!(
        selected(&app),
        Some(lower_storey),
        "a lower storey wins the (level, y, x) ordering even with a larger (y, x) — by \
         cell ordering, NOT Entity id (forbidden pick = `_high_last`) NOR spawn/iteration \
         order (forbidden pick = `_high_first`)",
    );
}

/// AC2 — an existing selection is NOT overridden: with a player ganger pre-selected (and
/// other lower-ordered player gangers present), several updates leave it UNCHANGED (the
/// auto-select only fills an EMPTY selection).
#[test]
fn auto_select_does_not_override_existing_selection() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    // A higher-ordered player ganger is pre-selected; a lower-ordered one also exists.
    let chosen = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(9, 9), level),
    );
    let _lower = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(0, 0), level),
    );
    app.world_mut()
        .insert_resource(SelectedShooter::new(chosen));

    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        selected(&app),
        Some(chosen),
        "an existing selection must NOT be overridden by the auto-select",
    );
}

/// AC3 — enemy-only roster → no selection: with only enemy-faction gangers (no player
/// faction fielded), the selection stays `None` (an enemy is never auto-selected).
#[test]
fn auto_select_enemy_only_stays_none() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    placed_ganger(
        &mut app,
        ENEMY_FACTION,
        CellLevel::new(Cell::new(0, 0), level),
    );
    placed_ganger(
        &mut app,
        ENEMY_FACTION,
        CellLevel::new(Cell::new(1, 1), level),
    );

    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        selected(&app),
        None,
        "with no player-faction ganger present the selection must stay None",
    );
}

/// GTW-729 — a STALE selection clears and ADVANCES when the selected ganger goes Downed
/// mid-turn (struck down by an enemy reaction while it was the acting unit). A Downed ganger
/// cannot act, so a selection stranded on it is a dead selection: `clear_downed_selection`
/// drops it, and `auto_select_first_player_ganger` (ordered `.after`) refills it with the next
/// Alive player ganger the SAME update (clear → advance).
///
/// Pin-discriminating (verification.md Rule 2): without `clear_downed_selection` the selection
/// would persist on the just-Downed ganger, failing the advance assertion.
#[test]
fn a_selection_that_downs_clears_and_advances_to_an_alive_ganger() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    // The lower-ordered ALIVE player ganger the auto-select advances the selection to.
    let advance_to = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(0, 0), level),
    );
    // The higher-ordered player ganger we select then DOWN — spawned Alive so we can flip its
    // LifeState in the test body.
    let downs = GangerEntityBuilder::new()
        .faction(PLAYER_FACTION)
        .life_state(LifeState::Alive)
        .at(CellLevel::new(Cell::new(9, 9), level))
        .spawn(app.world_mut());
    app.world_mut().insert_resource(SelectedShooter::new(downs));

    // Settle: while both are Alive the selection holds on the chosen ganger (auto-select only
    // fills an empty selection, so it never overrides this one).
    app.update();
    assert_eq!(
        selected(&app),
        Some(downs),
        "precondition: the higher-ordered ganger is selected while it is Alive",
    );

    // The selected ganger is struck down (an enemy reaction) mid-turn.
    app.world_mut().entity_mut(downs).insert(LifeState::Downed);
    app.update();

    assert_ne!(
        selected(&app),
        Some(downs),
        "a selection stranded on a just-Downed ganger must NOT persist (it cannot act)",
    );
    assert_eq!(
        selected(&app),
        Some(advance_to),
        "the stale selection clears and ADVANCES to the next Alive player ganger the same update",
    );
}

/// AC4 — inert outside a live battle: WITHOUT `BattleInProgress`, even with player
/// gangers present the auto-select does not run (the gate held), the selection stays
/// `None`, and there is no panic.
#[test]
fn auto_select_inert_without_battle_in_progress() {
    let level = Level::new(0);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    // NOTE: no BattleInProgress inserted; PlayerFaction present so only the battle gate
    // is the witness under test.
    app.world_mut().insert_resource(ActiveLevel::new(level));
    // GTW-521 — `dispatch_act_intents` also mutates the presenter-owned `ViewMode`.
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));

    // A player ganger that WOULD be auto-selected if the system ran.
    placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(0, 0), level),
    );

    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        selected(&app),
        None,
        "the auto-select must be inert (selection stays None) without BattleInProgress",
    );
}
