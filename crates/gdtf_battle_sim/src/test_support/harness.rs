//! Bevy app harness for `battle_sim` unit tests.

use bevy::{
    app::App, asset::AssetPlugin, platform::collections::HashSet, prelude::MinimalPlugins,
    scene::ScenePlugin,
};

use super::{
    registries::{test_armor_registry, test_melee_weapon_registry, test_weapon_registry},
    situation::test_gang_registry,
    terrain::test_terrain_registry,
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
                let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) else {
                    continue;
                };
                all.insert(CellLevel::new(Cell::new(x, y), Level::new(level)));
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

/// An optional piece a test app can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimAppPart {
    /// Acts plugin plus the core sim resources.
    Acts,
    /// Full battle sim plugin.
    Battle,
    /// Test weapon, melee weapon, armor, gang, and terrain registries.
    Registries,
    /// Vision covering the whole grid.
    FullVision,
}

/// Builder for a minimal Bevy app with optional sim plugins and resources.
#[derive(Debug, Clone)]
pub struct SimAppBuilder {
    seed:           u64,
    parts:          HashSet<SimAppPart>,
    player_faction: Option<u8>,
    tuning:         Option<CombatTuning>,
}

impl SimAppBuilder {
    /// Defaults: no plugins, fixed test seed.
    #[must_use]
    pub fn new() -> Self {
        Self {
            seed:           TEST_SEED,
            parts:          HashSet::default(),
            player_faction: None,
            tuning:         None,
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
    pub fn with_acts(self) -> Self {
        self.with_part(SimAppPart::Acts)
    }

    /// Add the full battle sim plugin.
    #[must_use]
    pub fn with_battle(self) -> Self {
        self.with_part(SimAppPart::Battle)
    }

    /// Insert test weapon/armor/gang/terrain registries.
    #[must_use]
    pub fn with_registries(self) -> Self {
        self.with_part(SimAppPart::Registries)
    }

    /// Cover the whole grid with vision.
    #[must_use]
    pub fn with_full_vision(self) -> Self {
        self.with_part(SimAppPart::FullVision)
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

    #[must_use]
    fn with_part(mut self, part: SimAppPart) -> Self {
        self.parts.insert(part);
        self
    }

    /// Build the app.
    pub fn build(self) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        if self.parts.contains(&SimAppPart::Battle) {
            app.add_plugins((
                AssetPlugin {
                    watch_for_changes_override: Some(false),
                    ..AssetPlugin::default()
                },
                ScenePlugin,
            ));
            app.add_plugins(BattleSimPlugin);
            app.insert_resource(CombatTuning::default());
            app.insert_resource(GangerStatTuning::default());
        }
        if self.parts.contains(&SimAppPart::Acts) {
            app.add_plugins(SimActsPlugin);
            insert_sim_resources(&mut app, BattleSeed::new(self.seed));
        }
        if self.parts.contains(&SimAppPart::FullVision) {
            app.insert_resource(full_vision());
        }
        if let Some(gang) = self.player_faction {
            app.insert_resource(PlayerFaction::new(Faction::new(gang)));
        }
        if let Some(tuning) = self.tuning {
            app.insert_resource(tuning);
        }
        if self.parts.contains(&SimAppPart::Registries) {
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
