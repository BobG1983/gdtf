//! Shared fixtures + helpers the battle-lifecycle test files glob-import — the
//! seeded constants, the `headless_app` harness, the message-buffer drains, and the
//! `world_mut()`-query `LifeState` setters (bevy-traps #7's headless-test carve-out).
//! The reusable ganger / situation / registry builders now live in the crate-central
//! [`crate::test_support`]; this file re-exports them and keeps only the
//! battle-lifecycle-specific harness.

pub(super) use bevy::{
    asset::AssetPlugin,
    ecs::message::Messages,
    prelude::{App, MinimalPlugins},
    scene::ScenePlugin,
};

// The canonical shared builders + registries + fixtures (consolidated out of this
// file's former local copies). The lifecycle fixtures below build over them, and the
// registry helpers keep their historical `weapon_registry` / `armor_registry` names
// for the concern files via the alias re-exports.
pub(super) use crate::test_support::{
    SituationBuilder, fixtures, ganger_at, key, test_armor_registry as armor_registry,
    test_weapon_registry as weapon_registry,
};
pub(super) use crate::{
    acts::FireRequested,
    armor::WornArmor,
    battle::{
        BattleInProgress, BattleLost, BattleReady, BattleRoster, BattleSimPlugin, BattleWon,
        PlayerFaction, SetupBattleRequested, TeardownBattleRequested,
    },
    cover::CoverLedger,
    ganger::{Faction, LifeState},
    metric::{Cell, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::CoverDestroyed,
    rng::{BattleSeed, SimRng},
    situation::{BattleSetupError, Situation, setup_battle},
    surface::SurfaceGrid,
    tuning::CombatTuning,
    vertical::{InvalidVerticalLink, LinkKind, VerticalLink, VerticalLinkGraph},
    weapon::{FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent},
};

/// An arbitrary (NOT shipped tuning) seed for a test battle's RNG stream.
pub(super) const SEED: u64 = 0x5A1C_AC75;

/// A valid two-ganger fixture situation (no cover / slabs / links — link-free
/// validates trivially). Delegates to the central
/// [`fixtures::two_ganger`](crate::test_support::fixtures::two_ganger): faction 0
/// and faction 1 gangers, `player_faction` defaulting to gang 0 (the AC2
/// default-seed precondition).
pub(super) fn two_ganger_situation() -> Situation {
    fixtures::two_ganger()
}

/// The two-ganger fixture with `player_faction` AUTHORED to gang 1 (overriding the
/// `Faction(0)` default) — the AC3 fixture proving the seed reads
/// `situation.player_faction`, not a hardcoded gang 0.
pub(super) fn two_ganger_situation_player_faction_one() -> Situation {
    fixtures::two_ganger_player_faction_one()
}

/// A situation with a DANGLING vertical link (an endpoint at a `(cell, level)` no
/// authored tile occupies) — `setup_battle` returns `Err(DanglingCell)` and
/// inserts no resource (the `setup_aborts_on_invalid_vertical_link` precedent).
pub(super) fn dangling_link_situation() -> (Situation, VerticalLink) {
    let present = key(4, 4, 0);
    let missing = key(4, 4, 1); // never authored — the link dangles off it
    let link = VerticalLink::new(present, missing, LinkKind::stair());
    // Built via the canonical [`SituationBuilder`] (GTW-324) — value-for-value identical to
    // the prior struct literal: one ganger, only `present` authored as a slab (`missing`
    // dangles), and the one dangling `vertical_link`. The returned link is unchanged.
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .slab_at(present) // only `present` authored; `missing` dangles
        .vertical_link(link) // `VerticalLink` is `Copy`, so the returned `link` is unchanged
        .build();
    (situation, link)
}

/// Build a headless app: [`MinimalPlugins`] (no window / renderer) + the
/// [`AssetPlugin`] + [`ScenePlugin`] the GTW-322 `bsn!` ganger spawn needs +
/// [`BattleSimPlugin`] — the `occupancy_sync` / `acts` headless precedent. The sim
/// crate alone, proving NO `gdtf_app` coupling.
///
/// [`AssetPlugin`] + [`ScenePlugin`] are required because `setup_battle` now spawns
/// each ganger as a `bsn!` [`Scene`](bevy::scene::Scene) via
/// `commands.spawn_scene` (GTW-322): the deferred `SpawnScene` flush PANICS in
/// `bevy_asset` without an `AssetServer` (the spike's pinned finding). In the real
/// app both ride `DefaultPlugins`, so this only mirrors production for the headless
/// harness. The ganger components materialize on the `SpawnScene` schedule, which
/// runs after `Update` within the SAME `app.update()`, so a single `update()` after a
/// `SetupBattleRequested` settles them.
///
/// Inserts [`CombatTuning::default`] up front, standing in for E10.4's PERSISTENT
/// `Load` resource (always present in the real app before a battle): once a setup
/// inserts the [`OccupancyGrid`] witness, the gated `Simulate` band runs the bundled
/// dispatch systems, which read `CombatTuning` — so it must be present (the
/// `acts.rs::insert_sim_resources` precedent). It is deliberately NOT one of the
/// battle-lifetime resources the teardown removes.
///
/// Inserts the central test [`WeaponRegistry`] + [`ArmorRegistry`] too (GTW-257 /
/// GTW-269): like `CombatTuning` they are PERSISTENT `Load` state present before a
/// battle, and `setup_battle_on_request` reads both to arm + armor each ganger. The
/// fixture gangers reference the central test weapon + armor keys, which the registries
/// hold, so a setup succeeds.
pub(super) fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning::default());
    app.insert_resource(weapon_registry());
    app.insert_resource(armor_registry());
    app
}

/// Drain the `BattleReady` buffer and return how many were emitted this run — the
/// setup-complete signal probe. `drain` empties the buffer, so a follow-up call
/// sees only messages written since (the test runs one `update()` then probes).
pub(super) fn drain_battle_ready(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .drain()
        .count()
}

/// Drain the `BattleWon` buffer and return how many were emitted since the last drain
/// — the win-census probe (mirrors `drain_battle_ready`).
pub(super) fn drain_battle_won(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<BattleWon>>()
        .drain()
        .count()
}

/// Drain the `BattleLost` buffer and return how many were emitted since the last drain
/// — the loss-census probe (mirrors `drain_battle_ready`).
pub(super) fn drain_battle_lost(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<BattleLost>>()
        .drain()
        .count()
}

/// A three-ganger fixture: one player ganger (`faction 0`) + two enemy gangers
/// (`faction 1`) — the AC1/AC2/AC4/AC6(b) win-side fixture (`PlayerFaction(0)` from the
/// `player_faction` default). Delegates to the central
/// [`fixtures::one_player_two_enemies`](crate::test_support::fixtures::one_player_two_enemies).
pub(super) fn one_player_two_enemy_situation() -> Situation {
    fixtures::one_player_two_enemies()
}

/// A player-only fixture: every ganger is `faction 0` — the AC6(a) degenerate /
/// empty-enemy-roster fixture (`has_enemy_of` is false, so the census never wins).
/// Delegates to the central
/// [`fixtures::player_only`](crate::test_support::fixtures::player_only).
pub(super) fn player_only_situation() -> Situation {
    fixtures::player_only()
}

/// Set the [`LifeState`] of every ganger whose [`Faction`] is `faction` to `to`, via a
/// `world_mut()` query in the test body (bevy-traps #7's headless-test carve-out — NOT
/// a registered system / helper taking `&mut World`). The accepted way to drive a
/// ganger out of the fight without re-running the damage pipeline.
pub(super) fn set_faction_life_state(app: &mut App, faction: u8, to: LifeState) {
    let target = Faction::new(faction);
    let world = app.world_mut();
    let mut query = world.query::<(&Faction, &mut LifeState)>();
    for (&fac, mut life) in query.iter_mut(world) {
        if fac == target {
            *life = to;
        }
    }
}

/// Set exactly ONE `Alive` ganger of `faction` to `to` (the first the query yields),
/// leaving the rest untouched — the AC2 "only one enemy down, the other still Alive"
/// driver. Returns whether a ganger was found and set (so the caller can assert the
/// fixture is sound). Only flips an `Alive` ganger so repeated calls down DISTINCT
/// gangers (never re-touch one already set).
pub(super) fn set_one_faction_ganger_life_state(app: &mut App, faction: u8, to: LifeState) -> bool {
    let target = Faction::new(faction);
    let world = app.world_mut();
    let mut query = world.query::<(&Faction, &mut LifeState)>();
    for (&fac, mut life) in query.iter_mut(world) {
        if fac == target && *life == LifeState::Alive {
            *life = to;
            return true;
        }
    }
    false
}
