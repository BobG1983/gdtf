pub(super) use bevy::{ecs::message::Messages, prelude::App};

pub(super) use crate::{
    acts::FireRequested,
    armor::Wears,
    battle::{
        BattleInProgress, BattleLost, BattleReady, BattleRoster, BattleWon, PlayerFaction,
        SetupBattleRequested, TeardownBattleRequested,
    },
    cover::CoverLedger,
    ganger::{Faction, LifeState},
    metric::{Cell, Level},
    occupancy::OccupancyGrid,
    occupancy_sync::TerrainPieceDestroyed,
    rng::{BattleSeed, ShotRng},
    situation::{BattleRegistries, BattleSetupError, Situation, setup_battle},
    surface::SurfaceGrid,
    test_support::{
        SimAppBuilder, SituationBuilder, fixtures, ganger_at, key,
        test_armor_registry as armor_registry, test_gang_registry,
        test_melee_weapon_registry as melee_weapon_registry,
        test_weapon_registry as weapon_registry,
    },
    tuning::{CombatTuning, GangerStatTuning},
    vertical::{InvalidVerticalLink, LinkKind, VerticalLink, VerticalLinkGraph},
    weapon::{FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent},
};

pub(super) const SEED: u64 = 0x5A1C_AC75;

pub(super) fn two_ganger_situation() -> Situation {
    fixtures::two_ganger()
}

pub(super) fn two_ganger_situation_player_faction_one() -> Situation {
    fixtures::two_ganger_player_faction_one()
}

pub(super) fn dangling_link_situation() -> (Situation, VerticalLink) {
    let present = key(4, 4, 0);
    let missing = key(4, 4, 1);
    let link = VerticalLink::new(present, missing, LinkKind::stair());
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .slab_at(present)
        .vertical_link(link)
        .build();
    (situation, link)
}

pub(super) fn headless_app() -> App {
    SimAppBuilder::new().with_battle().with_registries().build()
}

pub(super) fn drain_battle_ready(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .drain()
        .count()
}

pub(super) fn drain_battle_won(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<BattleWon>>()
        .drain()
        .count()
}

pub(super) fn drain_battle_lost(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<BattleLost>>()
        .drain()
        .count()
}

pub(super) fn one_player_two_enemy_situation() -> Situation {
    fixtures::one_player_two_enemies()
}

pub(super) fn player_only_situation() -> Situation {
    fixtures::player_only()
}

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
