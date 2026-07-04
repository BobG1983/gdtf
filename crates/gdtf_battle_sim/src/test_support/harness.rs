//! The knobbed headless sim-app builder + the ONE canonical sim-resource seeding
//! litany (GTW-576) — the shared halves every per-module `headless_app` composition
//! and every sim integration test builds on, replacing the per-file copies of the
//! `OccupancyGrid`/`SlabLedger`/five-RNG stanza.
//!
//! Per-module `headless_app` COMPOSITIONS stay bespoke where their plugin/resource
//! sets are deliberately different (the occupancy/openable/emplacement minimal
//! harnesses); what they share is the seeding half below. No helper here grows a
//! mode flag to serve two suites — a divergent composition composes the pieces
//! itself.

use bevy::{
    app::App, asset::AssetPlugin, platform::collections::HashSet, prelude::MinimalPlugins,
    scene::ScenePlugin,
};

use super::{
    registries::{
        test_armor_registry, test_melee_weapon_registry, test_terrain_registry,
        test_weapon_registry,
    },
    situation::test_gang_registry,
};
use crate::{
    acts::SimActsPlugin,
    battle::{BattleSimPlugin, PlayerFaction},
    cover::CoverLedger,
    ganger::Faction,
    injuries::{InjuryRegistry, InjuryTables},
    metric::{Cell, CellLevel, Level, MAX_LEVELS},
    occupancy::{GRID_HEIGHT, GRID_WIDTH, OccupancyGrid},
    rng::{BattleSeed, InjuryRng, LootRng, ProcgenRng, SeverityRng, ShotRng},
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    terrain::floor::FloorCostGrid,
    tuning::{CombatTuning, GangerStatTuning},
    vertical::VerticalLinkGraph,
    visibility::SquadVisibility,
};

/// The default per-test battle seed (the historical acts-suite seed — arbitrary,
/// never tuned). A test that pins a DIFFERENT stream overrides it via
/// [`SimAppBuilder::with_seed`] or passes its own [`BattleSeed`] to
/// [`insert_sim_resources`].
pub const TEST_SEED: u64 = 0x5A1C_AC75;

/// The player gang the canonical litany seeds (gang `0`) — override the inserted
/// [`PlayerFaction`] via [`SimAppBuilder::with_player_faction`].
pub const TEST_PLAYER_GANG: u8 = 0;

/// A [`SquadVisibility`] with the ENTIRE grid extent both VISIBLE and EXPLORED — the
/// "full vision" fog the move-dispatch tests route under (every cell routable, so the
/// GTW-353 visibility gate is a no-op and a route depends only on geometry +
/// occupancy). Mirrors the pathfinder-test `full_vision` fixture.
#[must_use]
pub fn full_vision() -> SquadVisibility {
    let mut all = HashSet::default();
    for level in 0..MAX_LEVELS {
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_possible_wrap,
                    reason = "x/y are 0..60 and level is 0..MAX_LEVELS (8) by the loop bounds, so \
                              the usize/u8 -> i32/u8 narrowing cannot truncate or wrap"
                )]
                let c = CellLevel::new(Cell::new(x as i32, y as i32), Level::new(level));
                all.insert(c);
            }
        }
    }
    SquadVisibility::new(all.clone(), all)
}

/// Insert the ONE canonical set of sim resources the `Simulate`-band dispatch
/// systems read — the grids ([`OccupancyGrid`] / [`SurfaceGrid`] / [`CoverLedger`] /
/// [`SlabLedger`] / [`BraceStairCells`]), the empty [`VerticalLinkGraph`], an EMPTY
/// (default) [`SquadVisibility`] fog, the [`PlayerFaction`] seed (gang
/// [`TEST_PLAYER_GANG`]), the five seeded per-subsystem RNG streams (GTW-14), EMPTY
/// injury content ([`InjuryTables`] / [`InjuryRegistry`] — the roll finds no bucket
/// and takes-then-discards its draw), [`CombatTuning::default`], and a uniform
/// [`FloorCostGrid`] at the default open cost (GTW-396 Decision B).
///
/// Everything is inserted EMPTY/default so a test that pins a grid, fog, faction,
/// tuning, or injury table simply RE-inserts its own value after this call —
/// `insert_resource` replaces. This is the seeding half every composition shares;
/// the plugin half stays per-suite (see [`SimAppBuilder`]).
pub fn insert_sim_resources(app: &mut App, seed: BattleSeed) {
    let tuning = CombatTuning::default();
    // GTW-396: a uniform FloorCostGrid at the default open cost — the harness resolves
    // no terrain registry, so the grid is built directly with the same move cost the
    // pre-GTW-396 tests expected. The move dispatch reads `Res<FloorCostGrid>`.
    let floor_costs = FloorCostGrid::new(tuning.move_costs.open, []);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(CoverLedger::new());
    // GTW-365: `dispatch_fire` reads `ResMut<SlabLedger>` — seed an empty ledger.
    app.insert_resource(SlabLedger::new());
    // GTW-392: `dispatch_fire` reads `Res<BraceStairCells>` — seed an empty set (no
    // stair links in the test arena, so no brace-stair cells are authored).
    app.insert_resource(BraceStairCells::empty());
    app.insert_resource(VerticalLinkGraph::default());
    app.insert_resource(SquadVisibility::default());
    app.insert_resource(PlayerFaction::new(Faction::new(TEST_PLAYER_GANG)));
    // GTW-14: the five per-subsystem RNG streams derived from the test seed.
    app.insert_resource(ShotRng::from_root(seed));
    app.insert_resource(SeverityRng::from_root(seed));
    app.insert_resource(LootRng::from_root(seed));
    app.insert_resource(InjuryRng::from_root(seed));
    app.insert_resource(ProcgenRng::from_root(seed));
    // GTW-438: `dispatch_fire` reads `Res<InjuryTables>` + `Res<InjuryRegistry>` (the
    // `roll_injury` inputs). Seed EMPTY ones so the fire path runs (the roll finds no
    // bucket and takes-then-discards its one InjuryRng draw); a test that pins an
    // injury overwrites them with populated resources before its run.
    app.insert_resource(InjuryTables::default());
    app.insert_resource(InjuryRegistry::default());
    app.insert_resource(tuning);
    app.insert_resource(floor_costs);
}

/// The knobbed headless sim-app builder (GTW-576) — `MinimalPlugins` plus exactly
/// the plugin/resource halves the knobs select. Builder-with-overrides, never one
/// fixed spawner: every knob defaults OFF, and a suite whose composition diverges
/// (an extra plugin, a populated grid, shipped tuning) adds/overrides on the built
/// [`App`] — `app.add_plugins(..)` / `app.insert_resource(..)` after `build()`.
///
/// - [`with_acts`](Self::with_acts) — [`SimActsPlugin`] + the canonical
///   [`insert_sim_resources`] litany (the dispatch band's read set), seeded from
///   [`with_seed`](Self::with_seed) (default [`TEST_SEED`]).
/// - [`with_battle`](Self::with_battle) — the full battle lifecycle:
///   `AssetPlugin` + `ScenePlugin` (the `bsn!` ganger spawn needs them) +
///   [`BattleSimPlugin`] + the persistent-`Load` stand-ins
///   ([`CombatTuning::default`] + [`GangerStatTuning::default`]). NO litany —
///   `setup_battle` inserts the battle-lifetime resources itself. Do NOT combine
///   with [`with_acts`](Self::with_acts): [`BattleSimPlugin`] already bundles
///   [`SimActsPlugin`], and a duplicate plugin add panics.
/// - [`with_registries`](Self::with_registries) — the five canonical test
///   registries (weapon / melee / armor / gang / terrain) a `setup_battle` resolves
///   authored keys against.
/// - [`with_full_vision`](Self::with_full_vision) — swap the default EMPTY fog for
///   [`full_vision`] (the acts/move-dispatch precondition).
/// - [`with_player_faction`](Self::with_player_faction) /
///   [`with_tuning`](Self::with_tuning) — override the litany's [`PlayerFaction`] /
///   [`CombatTuning`] seeds.
#[derive(Debug, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool IS an independent on/off knob selecting a plugin/resource half — \
              they are not mutually-entangled state (the one exclusion, acts vs battle, is \
              documented); a state machine would obscure the builder-with-overrides shape"
)]
pub struct SimAppBuilder {
    /// The root battle seed the litany's RNG streams derive from.
    seed:           u64,
    /// Add [`SimActsPlugin`] + the canonical resource litany.
    acts:           bool,
    /// Add the battle-lifecycle stack ([`BattleSimPlugin`] + asset/scene plugins).
    battle:         bool,
    /// Insert the five canonical test registries.
    registries:     bool,
    /// Swap the default EMPTY fog for [`full_vision`].
    full_vision:    bool,
    /// Override the litany's [`PlayerFaction`] gang.
    player_faction: Option<u8>,
    /// Override the litany's [`CombatTuning::default`] seed.
    tuning:         Option<CombatTuning>,
}

impl SimAppBuilder {
    /// A fresh builder: `MinimalPlugins` only, seeded with [`TEST_SEED`], every
    /// knob off.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            seed:           TEST_SEED,
            acts:           false,
            battle:         false,
            registries:     false,
            full_vision:    false,
            player_faction: None,
            tuning:         None,
        }
    }

    /// Derive the litany's five RNG streams from `seed` instead of [`TEST_SEED`].
    #[must_use]
    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Add [`SimActsPlugin`] and seed the canonical [`insert_sim_resources`] litany
    /// its ungated dispatch band reads.
    #[must_use]
    pub const fn with_acts(mut self) -> Self {
        self.acts = true;
        self
    }

    /// Add the battle-lifecycle stack: `AssetPlugin` + `ScenePlugin` +
    /// [`BattleSimPlugin`] + the persistent-`Load` tuning stand-ins. Mutually
    /// exclusive with [`with_acts`](Self::with_acts) (see the type docs).
    #[must_use]
    pub const fn with_battle(mut self) -> Self {
        self.battle = true;
        self
    }

    /// Insert the five canonical test registries (weapon / melee / armor / gang /
    /// terrain) — the persistent-`Load` stand-ins `setup_battle` resolves against.
    #[must_use]
    pub const fn with_registries(mut self) -> Self {
        self.registries = true;
        self
    }

    /// Seed [`full_vision`] instead of the default EMPTY fog (the acts-suite /
    /// move-dispatch precondition).
    #[must_use]
    pub const fn with_full_vision(mut self) -> Self {
        self.full_vision = true;
        self
    }

    /// Override the litany's [`PlayerFaction`] to `gang`.
    #[must_use]
    pub const fn with_player_faction(mut self, gang: u8) -> Self {
        self.player_faction = Some(gang);
        self
    }

    /// Override the litany's [`CombatTuning`] seed (e.g. the SHIPPED tuning, or a
    /// default with one leaf overlaid).
    #[must_use]
    pub fn with_tuning(mut self, tuning: CombatTuning) -> Self {
        self.tuning = Some(tuning);
        self
    }

    /// Build the app: `MinimalPlugins` + the knob-selected plugin/resource halves.
    pub fn build(self) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        if self.battle {
            // The GTW-322 `bsn!` ganger spawn needs an `AssetServer` + the scene
            // schedule; both ride `DefaultPlugins` in the real app, so the headless
            // battle harness mirrors them explicitly.
            app.add_plugins((AssetPlugin::default(), ScenePlugin));
            app.add_plugins(BattleSimPlugin);
            // The persistent-`Load` stand-ins `setup_battle` + the gated Simulate
            // band read (always present in the real app before a battle).
            app.insert_resource(CombatTuning::default());
            app.insert_resource(GangerStatTuning::default());
        }
        if self.acts {
            app.add_plugins(SimActsPlugin);
            insert_sim_resources(&mut app, BattleSeed::new(self.seed));
        }
        if self.full_vision {
            app.insert_resource(full_vision());
        }
        if let Some(gang) = self.player_faction {
            app.insert_resource(PlayerFaction::new(Faction::new(gang)));
        }
        if let Some(tuning) = self.tuning {
            app.insert_resource(tuning);
        }
        if self.registries {
            app.insert_resource(test_weapon_registry());
            app.insert_resource(test_melee_weapon_registry());
            app.insert_resource(test_armor_registry());
            app.insert_resource(test_gang_registry());
            app.insert_resource(test_terrain_registry());
        }
        app
    }
}

impl Default for SimAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}
