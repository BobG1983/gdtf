//! Bevy app harness for `battle_sim` unit tests.

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

/// Default seed used by the test harness.
pub const TEST_SEED: u64 = 0x5A1C_AC75;

/// Default player gang index.
pub const TEST_PLAYER_GANG: u8 = 0;

/// Visibility covering the full grid.
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

/// Insert core sim resources into an app.
pub fn insert_sim_resources(app: &mut App, seed: BattleSeed) {
    let tuning = CombatTuning::default();
    let floor_costs = FloorCostGrid::new(tuning.move_costs.open, []);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(CoverLedger::new());
    app.insert_resource(SlabLedger::new());
    app.insert_resource(BraceStairCells::empty());
    app.insert_resource(VerticalLinkGraph::default());
    app.insert_resource(SquadVisibility::default());
    app.insert_resource(PlayerFaction::new(Faction::new(TEST_PLAYER_GANG)));
    app.insert_resource(ShotRng::from_root(seed));
    app.insert_resource(SeverityRng::from_root(seed));
    app.insert_resource(LootRng::from_root(seed));
    app.insert_resource(InjuryRng::from_root(seed));
    app.insert_resource(ProcgenRng::from_root(seed));
    app.insert_resource(InjuryTables::default());
    app.insert_resource(InjuryRegistry::default());
    app.insert_resource(tuning);
    app.insert_resource(floor_costs);
}

/// Builder for a minimal Bevy app with optional sim plugins and resources.
#[derive(Debug, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool IS an independent on/off knob selecting a plugin/resource half — \
              they are not mutually-entangled state (the one exclusion, acts vs battle, is \
              documented); a state machine would obscure the builder-with-overrides shape"
)]
pub struct SimAppBuilder {
    seed: u64,
    acts: bool,
    battle: bool,
    registries: bool,
    full_vision: bool,
    player_faction: Option<u8>,
    tuning: Option<CombatTuning>,
}

impl SimAppBuilder {
    /// Defaults: no plugins, fixed test seed.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            seed: TEST_SEED,
            acts: false,
            battle: false,
            registries: false,
            full_vision: false,
            player_faction: None,
            tuning: None,
        }
    }

    /// Override the RNG seed.
    #[must_use]
    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Add the acts plugin and core resources.
    #[must_use]
    pub const fn with_acts(mut self) -> Self {
        self.acts = true;
        self
    }

    /// Add the full battle sim plugin.
    #[must_use]
    pub const fn with_battle(mut self) -> Self {
        self.battle = true;
        self
    }

    /// Insert test weapon/armor/gang/terrain registries.
    #[must_use]
    pub const fn with_registries(mut self) -> Self {
        self.registries = true;
        self
    }

    /// Cover the whole grid with vision.
    #[must_use]
    pub const fn with_full_vision(mut self) -> Self {
        self.full_vision = true;
        self
    }

    /// Set the player faction index.
    #[must_use]
    pub const fn with_player_faction(mut self, gang: u8) -> Self {
        self.player_faction = Some(gang);
        self
    }

    /// Override combat tuning.
    #[must_use]
    pub fn with_tuning(mut self, tuning: CombatTuning) -> Self {
        self.tuning = Some(tuning);
        self
    }

    /// Build the app.
    pub fn build(self) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        if self.battle {
            app.add_plugins((AssetPlugin::default(), ScenePlugin));
            app.add_plugins(BattleSimPlugin);
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
