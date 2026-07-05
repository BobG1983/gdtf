//! The battle-lifecycle TEARDOWN driver: [`teardown_battle_on_request`] — the
//! battle-lifetime resource REMOVE set (the mirror of
//! [`runtime_seed`](super::setup_battle_on_request)'s seed set) + the terrain-entity
//! despawn (E10.5 / GTW-212).

use bevy::prelude::{Commands, MessageReader};

use crate::{
    ai::{ActCadence, EnemyActCooldown},
    battle::{
        messages::TeardownBattleRequested,
        resources::{BattleInProgress, BattleRoster, PlayerFaction},
    },
    cover::CoverLedger,
    occupancy::OccupancyGrid,
    rng::{FightRng, InjuryRng, LootRng, ProcgenRng, ReactionRng, SeverityRng, ShotRng},
    slab::SlabLedger,
    surface::SurfaceGrid,
    terrain::{entity::TerrainIndex, floor::FloorCostGrid},
    turn::ActiveFaction,
    vertical::VerticalLinkGraph,
    visibility::{OmniscientFog, SquadVisibility},
};

/// Remove all seven per-subsystem RNG stream resources from the world.
///
/// Called during [`teardown_battle_on_request`] to clean every battle-lifetime RNG
/// stream in one place (bevy-traps.md #1 — resources must be removed on state exit).
/// A [`remove_resource`](Commands::remove_resource) on an absent resource is a no-op,
/// so a spurious or double call is harmless. Defined as a standalone `fn` so the
/// teardown system body stays focused on ordering concerns and is easy to audit.
///
/// GTW-466 adds [`ReactionRng`] as the sixth stream and GTW-506 adds [`FightRng`] as
/// the seventh (each a data substrate — pinned at setup so its seed is fixed from the
/// adding ticket's boundary).
fn remove_rng_streams(commands: &mut bevy::prelude::Commands) {
    commands.remove_resource::<ShotRng>();
    commands.remove_resource::<SeverityRng>();
    commands.remove_resource::<LootRng>();
    commands.remove_resource::<InjuryRng>();
    commands.remove_resource::<ProcgenRng>();
    commands.remove_resource::<ReactionRng>();
    commands.remove_resource::<FightRng>();
}

/// **Teardown** the battle on [`TeardownBattleRequested`] — remove the
/// battle-lifetime resources, including the [`BattleInProgress`] gate witness (E10.5
/// / GTW-212).
///
/// Drains [`MessageReader<TeardownBattleRequested>`] and, when triggered, removes
/// the seven per-subsystem RNG streams ([`ShotRng`] / [`SeverityRng`] / [`LootRng`]
/// / [`InjuryRng`] / [`ProcgenRng`] / [`ReactionRng`] / [`FightRng`]) that GTW-14 /
/// GTW-466 / GTW-506 inserted at setup, the
/// [`setup_battle`](crate::situation::setup_battle)-inserted resources ([`CoverLedger`] / [`SurfaceGrid`] /
/// [`OccupancyGrid`] / [`VerticalLinkGraph`] /
/// [`SlabLedger`] / [`TerrainIndex`](crate::terrain::entity::TerrainIndex) /
/// [`FloorCostGrid`]), the [`BattleInProgress`] witness (closing the
/// [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate) gate so the
/// bundled runtime goes inert again), the [`PlayerFaction`], the [`BattleRoster`], the
/// [`ActiveFaction`] turn-cycle resource, the [`EnemyActCooldown`] + [`ActCadence`]
/// AI-pacing resources (GTW-461), the
/// [`SquadVisibility`](crate::visibility::SquadVisibility) squad fog (GTW-341), and the
/// [`OmniscientFog`](crate::visibility::OmniscientFog) AI move fog (GTW-70) (all
/// lifetimes track [`BattleInProgress`], so they are removed in the same teardown).
///
/// GTW-395: also despawns all [`TerrainCell`](crate::terrain::entity::TerrainCell)
/// entities (the per-tile terrain entities spawned in `setup_battle`) and removes the
/// pre-existing [`SlabLedger`] leak (it was inserted by `setup_battle` but never
/// removed until GTW-395).
///
/// GTW-396: removes the [`FloorCostGrid`] alongside the other battle-lifetime
/// resources (it is now inserted by `setup_battle` rather than by this function).
///
/// [`CombatTuning`](crate::tuning::CombatTuning) is deliberately NOT removed: it is
/// E10.4's persistent `Load` resource, untouched by this plugin. A
/// [`remove_resource`](Commands::remove_resource) on an absent resource is a no-op,
/// so a spurious / double teardown is harmless.
pub fn teardown_battle_on_request(
    mut requests: MessageReader<TeardownBattleRequested>,
    terrain_entities: bevy::prelude::Query<
        bevy::prelude::Entity,
        bevy::prelude::With<crate::terrain::entity::TerrainCell>,
    >,
    mut commands: Commands,
) {
    // Drain the buffer; act once if any teardown was requested (the removed set is
    // fixed, so draining N triggers and removing once is equivalent to N removes).
    let mut requested = false;
    for _request in requests.read() {
        requested = true;
    }
    if requested {
        // GTW-14: remove all five per-subsystem RNG streams (bevy-traps.md #1:
        // resources must be removed on exit; a remove_resource on an absent resource
        // is a no-op, so a spurious / double teardown is harmless).
        remove_rng_streams(&mut commands);
        commands.remove_resource::<CoverLedger>();
        commands.remove_resource::<SurfaceGrid>();
        // GTW-395: remove the SlabLedger (pre-existing leak fix — it was inserted at
        // setup.rs but never removed here until GTW-395. The teardown now owns it
        // alongside TerrainIndex for a clean battle boundary).
        commands.remove_resource::<SlabLedger>();
        // GTW-392: remove the BraceStairCells (the lower-endpoint stair-cell set
        // inserted at setup — battle-lifetime, removed alongside the other resources).
        commands.remove_resource::<crate::slab::BraceStairCells>();
        commands.remove_resource::<OccupancyGrid>();
        commands.remove_resource::<VerticalLinkGraph>();
        // GTW-395: remove the battle-lifetime TerrainIndex (spawned in setup_battle's
        // step 4, removed here alongside the other battle-lifetime resources).
        commands.remove_resource::<TerrainIndex>();
        // GTW-396: remove the battle-lifetime FloorCostGrid (now inserted by
        // setup_battle rather than by setup_battle_on_request; same lifetime as the
        // other battle-lifetime resources — removed alongside BattleInProgress).
        commands.remove_resource::<FloorCostGrid>();
        // GTW-545: remove the battle-lifetime FieldRegistry (the live area-damage-field
        // placements, inserted by setup_battle; same lifetime as the other battle grids —
        // removed alongside BattleInProgress so a fresh battle starts field-free).
        commands.remove_resource::<crate::fields::FieldRegistry>();
        // GTW-547: remove the battle-lifetime CoverOnDeathRegistry (the cover-cell on-death
        // effects, inserted by setup_battle; same lifetime as the other battle grids — removed
        // alongside BattleInProgress so a fresh battle starts with no stale cover effects).
        commands.remove_resource::<crate::on_death::CoverOnDeathRegistry>();
        // GTW-395: despawn all terrain entities (one per authored cover / slab piece).
        // Commands::despawn (bevy-traps #7 form — never world.spawn/despawn inside a
        // registered system): each entity is queued for despawn at the end of this frame.
        for entity in terrain_entities.iter() {
            commands.entity(entity).despawn();
        }
        // Close the gate witness alongside the battle-lifetime resources, so the
        // Simulate band goes inert (and panic-free) after the battle ends (GTW-212).
        commands.remove_resource::<BattleInProgress>();
        // Remove the PlayerFaction alongside, so its lifetime stays identical to
        // BattleInProgress (the later Res<PlayerFaction> readers gate on that window).
        commands.remove_resource::<PlayerFaction>();
        // Remove the BattleRoster alongside, so its lifetime stays identical to
        // BattleInProgress (the Simulate-band census reads it within that window).
        commands.remove_resource::<BattleRoster>();
        // Remove the ActiveFaction alongside, so the turn cycle's lifetime stays identical
        // to BattleInProgress (the turn-cycle engine reads it within that window; GTW-309).
        commands.remove_resource::<ActiveFaction>();
        // GTW-461: remove the enemy-act cooldown + cadence alongside, so the AI-pacing
        // resources' lifetime stays identical to BattleInProgress (the brain reads them
        // within that window).
        commands.remove_resource::<EnemyActCooldown>();
        commands.remove_resource::<ActCadence>();
        // Remove the SquadVisibility alongside, so the squad fog's lifetime stays identical
        // to BattleInProgress (recompute_visibility — the sole writer — reads its
        // ResMut within that window; GTW-341).
        commands.remove_resource::<SquadVisibility>();
        // GTW-70: remove the AI's OmniscientFog alongside, so the move-planning fog's
        // lifetime stays identical to BattleInProgress (the enemy brain + the faction-aware
        // dispatch_move read it within that window).
        commands.remove_resource::<OmniscientFog>();
    }
}
