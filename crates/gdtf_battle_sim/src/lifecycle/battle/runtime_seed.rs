//! [`insert_battle_runtime`] — the battle-lifetime resource SEED set (RNG streams,
//! gate witness, factions, act log, fog) inserted on
//! [`setup_battle_on_request`](super::setup_battle_on_request)'s `Ok` path — the
//! mirror of [`teardown`](super::teardown_battle_on_request)'s remove set.

use bevy::prelude::Commands;

use crate::{
    act_log::ActLog,
    battle::{
        messages::SetupBattleRequested,
        resources::{BattleInProgress, BattleRoster, PlayerFaction},
    },
    occupancy::OccupancyGrid,
    rng::{FightRng, InjuryRng, LootRng, ProcgenRng, ReactionRng, SeverityRng, ShotRng},
    turn::ActiveFaction,
    visibility::{OmniscientFog, SquadVisibility},
};

/// The battle is live: insert the gate witness (alongside the `setup_battle` grids +
/// the seeded RNG streams) so the Simulate band's bundled runtime turns on, seed the
/// [`PlayerFaction`] from the situation, capture the [`BattleRoster`] from the fielded
/// gangers' factions (both share the [`BattleInProgress`] lifetime — same `Ok` path,
/// removed together on teardown). All happen ONLY on the setup driver's `Ok` arm.
///
/// NOTE: the [`FloorCostGrid`](crate::terrain::floor::FloorCostGrid) is inserted by
/// `setup_battle` itself (GTW-396 Decision B — it is built from the resolved floor
/// specs and inserted directly), not here.
pub(super) fn insert_battle_runtime(commands: &mut Commands, request: &SetupBattleRequested) {
    // GTW-14 / GTW-466 / GTW-506: derive and insert the seven battle-lifetime
    // per-subsystem RNG streams from the trigger's root seed ON THE Ok PATH ONLY,
    // so they share exactly the BattleInProgress lifetime (removed together on
    // teardown) and
    // a FAILED setup_battle leaves NO orphaned RNG resources. Each stream is an
    // independent ChaCha12Rng derived via fnv1a64(root, label); adding a draw on
    // one stream cannot perturb any other stream's output sequence.
    let root = request.seed;
    commands.insert_resource(ShotRng::from_root(root));
    commands.insert_resource(SeverityRng::from_root(root));
    commands.insert_resource(LootRng::from_root(root));
    commands.insert_resource(InjuryRng::from_root(root));
    commands.insert_resource(ProcgenRng::from_root(root));
    // GTW-466: reaction-fire RNG stream (data substrate — no draw sites yet;
    // pinned here so its seed is fixed from the GTW-466 boundary and a future
    // first draw replays correctly with no migration).
    commands.insert_resource(ReactionRng::from_root(root));
    // GTW-506: melee opposed-Fight RNG stream (data substrate — the §7
    // opposed_fight core draws from it, but no ECS system does yet; the live
    // melee ACT that owns the ResMut<FightRng> is GTW-507). A DEDICATED stream
    // (NOT ReactionRng) so melee and reaction-fire determinism stay isolated.
    commands.insert_resource(FightRng::from_root(root));

    commands.insert_resource(BattleInProgress);
    commands.insert_resource(PlayerFaction::new(request.situation.player_faction));
    commands.insert_resource(BattleRoster::new(
        request
            .situation
            .gangers
            .iter()
            .map(|ganger| ganger.faction),
    ));
    // GTW-309: seed the turn cycle to the PlayerFaction (the player acts
    // first), sharing the BattleInProgress lifetime (removed together on
    // teardown) so the turn-cycle engine's ActiveFaction read is panic-free.
    commands.insert_resource(ActiveFaction::new(request.situation.player_faction));
    // GTW-727: insert the EMPTY act log on this same Ok path, sharing the
    // BattleInProgress lifetime (removed together on teardown). Per-battle
    // lifetime is what makes sequence numbering restart at zero and the
    // transition-detection prior-value maps reset for free — no separate reset
    // pass, and no chance of a previous battle's last-known posture seeding the
    // next battle's first recorded change. It replaces the GTW-461 enemy-act
    // cadence resources this slot used to hold: pacing is no longer the sim's
    // job, so the sim no longer carries pacing state.
    commands.insert_resource(ActLog::default());
    // GTW-341: insert an EMPTY SquadVisibility on this same Ok path, sharing
    // the BattleInProgress lifetime. It starts empty; the BattleReady we write
    // below trips recompute_visibility (the SOLE writer) on the next update,
    // filling it with the spawn-time squad FOV — so the spawn FOV is covered by
    // the BattleReady trigger, not computed here (clause 4). The empty insert
    // makes the ResMut<SquadVisibility> read panic-free the moment the gated
    // Simulate band can run (bevy-traps.md #1).
    commands.insert_resource(SquadVisibility::default());
    // GTW-70: insert the AI's OMNISCIENT move fog on this same Ok path, sharing
    // the BattleInProgress lifetime. It is the enemy AI's move-planning fog
    // (every in-bounds cell visible+explored) AND the fog `dispatch_move`'s
    // `move_fog` selects for any non-player mover — one shared resource, so
    // planner and executor can never drift. The omniscient set is the FIXED
    // 60×60×8 grid extent (independent of slot contents), so a fresh default
    // grid yields the identical full cell set without needing the
    // Commands-queued battle grid to have been applied yet.
    commands.insert_resource(OmniscientFog::new(SquadVisibility::omniscient(
        &OccupancyGrid::new(),
    )));
}
