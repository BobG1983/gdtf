//! The two battle-lifecycle DRIVER systems: [`setup_battle_on_request`] (seed RNG +
//! run [`setup_battle`], signal [`BattleReady`] on `Ok`) and
//! [`teardown_battle_on_request`] (remove the battle-lifetime resources) — E10.5 /
//! GTW-212 / GTW-257.

use bevy::prelude::{Commands, MessageReader, MessageWriter, Res, error};

use crate::{
    ai::{ActCadence, EnemyActCooldown},
    armor::ArmorRegistry,
    battle::{
        messages::{BattleReady, SetupBattleRequested, TeardownBattleRequested},
        resources::{BattleInProgress, BattleRoster, PlayerFaction},
    },
    cover::CoverLedger,
    ganger::GangRegistry,
    occupancy::OccupancyGrid,
    rng::{InjuryRng, LootRng, ProcgenRng, ReactionRng, SeverityRng, ShotRng},
    situation::{BattleRegistries, setup_battle},
    slab::SlabLedger,
    surface::SurfaceGrid,
    terrain::{def::TerrainDefRegistry, entity::TerrainIndex, floor::FloorCostGrid},
    tuning::{CombatTuning, GangerStatTuning},
    turn::ActiveFaction,
    vertical::VerticalLinkGraph,
    visibility::{OmniscientFog, SquadVisibility},
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};

/// **Setup** the battle on [`SetupBattleRequested`] — seed the per-subsystem RNG
/// streams and run [`setup_battle`], signalling [`BattleReady`] on success (E10.5).
///
/// Drains [`MessageReader<SetupBattleRequested>`] and per message:
///
/// 1. Calls [`setup_battle`] on the REAL [`Commands`] path, resolving each ganger's
///    weapon key against the [`WeaponRegistry`] (GTW-257), each ganger's armor key
///    against the [`ArmorRegistry`] (GTW-269), and each cover/slab terrain definition
///    UUID against the [`TerrainDefRegistry`] (GTW-491). On `Ok` the sim resources
///    ([`CoverLedger`] / [`SurfaceGrid`] / [`OccupancyGrid`] /
///    [`VerticalLinkGraph`] / [`FloorCostGrid`]) and the spawned ganger entities
///    land in the world, the six battle-lifetime per-subsystem RNG stream resources
///    ([`ShotRng`], [`SeverityRng`], [`LootRng`], [`InjuryRng`], [`ProcgenRng`],
///    [`ReactionRng`]) are inserted (derived from the message's
///    [`BattleSeed`](crate::rng::BattleSeed) via the stable FNV-1a-64 label-hash,
///    GTW-14; each stream independent — a draw on one cannot perturb another; all
///    portable `ChaCha12Rng`-backed, byte-stable across builds and platforms for
///    cross-run replay), the [`BattleInProgress`] witness is inserted (the battle-active
///    tag the [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate) band
///    gates on), the [`PlayerFaction`] is inserted seeded from
///    [`Situation::player_faction`](crate::situation::Situation), the
///    [`BattleRoster`] is captured from the situation's fielded gangers' factions, the
///    [`ActiveFaction`] turn-cycle resource is seeded to the same player faction (the
///    player acts first; GTW-309), the [`EnemyActCooldown`] (ready-to-act) + the default
///    [`ActCadence`] AI-pacing resources are inserted (GTW-461 — the brain emits at most
///    one enemy act per cadence-step), an EMPTY
///    [`SquadVisibility`](crate::visibility::SquadVisibility) squad fog is inserted
///    (GTW-341), the [`OmniscientFog`](crate::visibility::OmniscientFog) AI move fog is
///    inserted (GTW-70 — every in-bounds cell visible+explored) — all sharing
///    [`BattleInProgress`]'s lifetime — and a
///    [`BattleReady`] is written; on `Err` the typed
///    [`BattleSetupError`](crate::situation::BattleSetupError) (an invalid vertical link,
///    an unresolved weapon/armor/terrain key, or a below-minimum floor cost) is surfaced
///    via [`error!`] and NEITHER the six RNG streams / [`BattleInProgress`] /
///    [`PlayerFaction`] / [`BattleRoster`] / [`ActiveFaction`] NOR [`BattleReady`] is
///    written — the app never advances on a bad battle, the gate never opens, and a
///    failed setup leaves NO orphaned RNG stream resources. NO
///    `unwrap`/`expect`/`panic`.
///
/// The [`GangRegistry`] (GTW-414), [`WeaponRegistry`], [`ArmorRegistry`], and
/// [`TerrainDefRegistry`] (GTW-491) are each read as `Option<Res<_>>` (PERSISTENT `Load`
/// state); a setup requested before ANY loads fails closed (logged, no [`BattleReady`]). The
/// [`GangRegistry`] resolves each [`PlacedGanger`](crate::situation::PlacedGanger)'s
/// `(gang, member)` ref into the roster member `setup_battle` derives the ganger from.
///
/// [`CombatTuning`](crate::tuning::CombatTuning) is NOT inserted here: it is E10.4's
/// PERSISTENT `Load` resource, present throughout the battle for the acts to read. It
/// IS read here as `Option<Res<_>>` to supply the `fallback_floor_cost`
/// (`CombatTuning::move_costs.open`) used as the uniform floor cost (GTW-491 retires the
/// per-floor registry move-cost resolution; the move-cost-from-`default_floor` seam is
/// GTW-482).
#[expect(
    clippy::too_many_arguments,
    reason = "the params are the message reader + writer, the gang / weapon / MELEE-weapon \
              (GTW-505) / armor / terrain registries, and the stat + combat tuning — each a \
              distinct Bevy SystemParam (Option<Res<_>> for the Load-state registries); the \
              injection model cannot be refactored to fewer without a wrapper resource that \
              changes the API surface"
)]
pub fn setup_battle_on_request(
    mut requests: MessageReader<SetupBattleRequested>,
    mut ready: MessageWriter<BattleReady>,
    gangs: Option<Res<GangRegistry>>,
    weapons: Option<Res<WeaponRegistry>>,
    melee_weapons: Option<Res<MeleeWeaponRegistry>>,
    armor: Option<Res<ArmorRegistry>>,
    terrain: Option<Res<TerrainDefRegistry>>,
    stat_tuning: Option<Res<GangerStatTuning>>,
    combat_tuning: Option<Res<CombatTuning>>,
    mut commands: Commands,
) {
    for request in requests.read() {
        // The weapon registry is E10.4-style PERSISTENT `Load` state, present before
        // any battle in the real app. This system runs UNGATED (before the
        // BattleInProgress-gated Simulate band), so it takes `Option<Res<_>>` to stay
        // panic-free if a setup is somehow requested before the registry loaded
        // (bevy-traps #1): a missing registry fails closed — no setup, no BattleReady.
        // GTW-414: the gang registry is the same PERSISTENT `Load` state, read as
        // `Option<Res<_>>` so a setup somehow requested before the gangs folder loaded
        // fails closed — no setup, no BattleReady (bevy-traps #1, mirroring the weapon
        // registry guard). Without it the PlacedGanger gang/member refs cannot resolve.
        let Some(gangs) = gangs.as_deref() else {
            error!(
                "battle setup requested but no GangRegistry is loaded; no BattleReady will be \
                 signalled (the gangs folder must load before a battle starts)"
            );
            continue;
        };
        let Some(weapons) = weapons.as_deref() else {
            error!(
                "battle setup requested but no WeaponRegistry is loaded; no BattleReady will be \
                 signalled (the weapons folder must load before a battle starts)"
            );
            continue;
        };
        // GTW-505: the MELEE weapon registry is the same PERSISTENT `Load` state, read as
        // `Option<Res<_>>` so a setup somehow requested before the melee weapons folder
        // loaded fails closed — no setup, no BattleReady (bevy-traps #1, mirroring the
        // ranged weapon registry guard). Without it (and the shipped `fists` default) an
        // un-authored ganger's melee weapon cannot resolve.
        let Some(melee_weapons) = melee_weapons.as_deref() else {
            error!(
                "battle setup requested but no MeleeWeaponRegistry is loaded; no BattleReady will \
                 be signalled (the melee weapons folder must load before a battle starts)"
            );
            continue;
        };
        // The armor registry is the same PERSISTENT `Load` state (GTW-269), read as
        // `Option<Res<_>>` so a setup somehow requested before it loaded fails closed —
        // no setup, no BattleReady (bevy-traps #1, mirroring the weapon registry guard).
        let Some(armor) = armor.as_deref() else {
            error!(
                "battle setup requested but no ArmorRegistry is loaded; no BattleReady will be \
                 signalled (the armor folder must load before a battle starts)"
            );
            continue;
        };
        // GTW-384: the GangerStatTuning is PERSISTENT `Load` state like CombatTuning,
        // resolved with a const-default fallback (its loader inserts a default on a
        // failed/missing file), so a setup somehow requested before it loaded does NOT
        // fail closed — it derives from the const-default weights (combat must never be
        // BLOCKED by missing balance data, matching CombatTuning's defaulting). Bind the
        // const default to a local so the borrow outlives the setup call.
        let default_stat_tuning = GangerStatTuning::default();
        let stat_tuning = stat_tuning.as_deref().unwrap_or(&default_stat_tuning);

        // GTW-396: the fallback floor cost — used when the situation omits
        // `default_floor` or when no terrain-definition registry is available. Sourced from
        // `CombatTuning::move_costs.open` (4 by default) to preserve pre-GTW-396
        // behavior for un-migrated test fixtures. Combat must never be blocked by
        // missing balance data.
        let default_combat_tuning = CombatTuning::default();
        let fallback_floor_cost = combat_tuning
            .as_deref()
            .map_or(default_combat_tuning.move_costs.open, |ct| {
                ct.move_costs.open
            });

        // GTW-491: the TerrainDefRegistry is PERSISTENT `Load` state (GTW-487, the UUID-keyed
        // terrain model), read as `Option<Res<_>>`. When absent,
        // `setup_battle` is called with `terrain: None` which:
        //  (a) makes cover/slab UUIDs fail with TerrainNotFound (a real situation's
        //      cover/slabs reference def UUIDs that need the registry);
        //  (b) uses the fallback_floor_cost for the floor grid (correct for test fixtures
        //      that use SituationBuilder without authored terrain).
        // The real app always has the registry loaded before a battle starts. Tests that use
        // SituationBuilder without cover/slabs pass `None` implicitly — they never authored
        // terrain UUIDs.
        let terrain_ref = terrain.as_deref();

        // Pour the situation into the world via the authoritative setup. A bad
        //    vertical link, a missing weapon/armor/terrain key, or a below-minimum
        //    floor cost returns the typed error — log it (NEVER panic / unwrap) and
        //    write NO BattleReady, so the app's gate never fires (fail-closed). The
        //    GangerStatTuning derives each ganger's computed stats from its eight
        //    authored attributes (GTW-384).
        match setup_battle(
            &request.situation,
            BattleRegistries::new(
                gangs,
                weapons,
                melee_weapons,
                armor,
                stat_tuning,
                terrain_ref,
            ),
            fallback_floor_cost,
            &mut commands,
        ) {
            Ok(_setup) => {
                // The battle is live: insert the gate witness (alongside the setup_battle
                // grids + the seeded RNG streams) so the Simulate band's bundled runtime turns
                // on, seed the PlayerFaction from the situation, capture the BattleRoster
                // from the fielded gangers' factions (both share the BattleInProgress
                // lifetime — same Ok path, removed together on teardown), then signal
                // BattleReady. All happen ONLY on Ok.
                //
                // NOTE: the FloorCostGrid is now inserted by setup_battle itself (GTW-396
                // Decision B — it is built from the resolved floor specs and inserted
                // directly). This is a change from the earlier sim-slice that inserted it
                // here: the full registry-driven grid now comes from setup_battle.

                // GTW-14 / GTW-466: derive and insert the six battle-lifetime per-subsystem
                // RNG streams from the trigger's root seed ON THE Ok PATH ONLY, so they share
                // exactly the BattleInProgress lifetime (removed together on teardown) and
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
                // GTW-461: insert the enemy-act COOLDOWN ready-to-act (0 ticks) + the default
                // ActCadence on this same Ok path, sharing the BattleInProgress lifetime
                // (removed together on teardown). The cooldown paces `enemy_ai_turn` to at
                // most one enemy act per cadence-step, so the enemy turn resolves act-by-act
                // on screen instead of as a one-frame volley. Seeded `ready()` (0) so the
                // enemy's FIRST act is immediate — the dwell only applies BETWEEN acts.
                commands.insert_resource(EnemyActCooldown::ready());
                commands.insert_resource(ActCadence::default());
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
                ready.write(BattleReady);
            }
            Err(e) => {
                error!(
                    "battle setup failed: {e} (an invalid vertical link, an unresolved \
                     weapon/armor/terrain key, a floor cost below the A* admissibility \
                     floor, or two gangers stacked on one spawn cell); no BattleReady \
                     will be signalled"
                );
            }
        }
    }
}

/// Remove all six per-subsystem RNG stream resources from the world.
///
/// Called during [`teardown_battle_on_request`] to clean every battle-lifetime RNG
/// stream in one place (bevy-traps.md #1 — resources must be removed on state exit).
/// A [`remove_resource`](Commands::remove_resource) on an absent resource is a no-op,
/// so a spurious or double call is harmless. Defined as a standalone `fn` so the
/// teardown system body stays focused on ordering concerns and is easy to audit.
///
/// GTW-466 adds [`ReactionRng`] as the sixth stream (data substrate — no draw sites
/// yet; pinned here so its seed is fixed from the GTW-466 boundary).
fn remove_rng_streams(commands: &mut bevy::prelude::Commands) {
    commands.remove_resource::<ShotRng>();
    commands.remove_resource::<SeverityRng>();
    commands.remove_resource::<LootRng>();
    commands.remove_resource::<InjuryRng>();
    commands.remove_resource::<ProcgenRng>();
    commands.remove_resource::<ReactionRng>();
}

/// **Teardown** the battle on [`TeardownBattleRequested`] — remove the
/// battle-lifetime resources, including the [`BattleInProgress`] gate witness (E10.5
/// / GTW-212).
///
/// Drains [`MessageReader<TeardownBattleRequested>`] and, when triggered, removes
/// the six per-subsystem RNG streams ([`ShotRng`] / [`SeverityRng`] / [`LootRng`]
/// / [`InjuryRng`] / [`ProcgenRng`] / [`ReactionRng`]) that GTW-14 / GTW-466
/// inserted at setup, the
/// [`setup_battle`]-inserted resources ([`CoverLedger`] / [`SurfaceGrid`] /
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
