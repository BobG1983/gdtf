//! The two battle-lifecycle DRIVER systems: [`setup_battle_on_request`] (seed RNG +
//! run [`setup_battle`], signal [`BattleReady`] on `Ok`) and
//! [`teardown_battle_on_request`] (remove the battle-lifetime resources) — E10.5 /
//! GTW-212 / GTW-257.

use bevy::prelude::{Commands, MessageReader, MessageWriter, Res, error};

use crate::{
    armor::ArmorRegistry,
    battle::{
        messages::{BattleReady, SetupBattleRequested, TeardownBattleRequested},
        resources::{BattleInProgress, BattleRoster, PlayerFaction},
    },
    cover::CoverLedger,
    occupancy::OccupancyGrid,
    rng::SimRng,
    situation::setup_battle,
    slab::SlabLedger,
    surface::SurfaceGrid,
    terrain::{entity::TerrainIndex, floor::FloorCostGrid, piece::TerrainRegistry},
    tuning::{CombatTuning, GangerStatTuning},
    turn::ActiveFaction,
    vertical::VerticalLinkGraph,
    visibility::SquadVisibility,
    weapon::WeaponRegistry,
};

/// **Setup** the battle on [`SetupBattleRequested`] — seed the [`SimRng`] and run
/// [`setup_battle`], signalling [`BattleReady`] on success (E10.5).
///
/// Drains [`MessageReader<SetupBattleRequested>`] and per message:
///
/// 1. Inserts the battle-lifetime [`SimRng`] seeded from the message's
///    [`BattleSeed`](crate::rng::BattleSeed) (via
///    [`SimRng::from_seed`](crate::rng::SimRng::from_seed) — the deterministic stream the
///    acts draw from).
/// 2. Calls [`setup_battle`] on the REAL [`Commands`] path, resolving each ganger's
///    weapon key against the [`WeaponRegistry`] (GTW-257), each ganger's armor key
///    against the [`ArmorRegistry`] (GTW-269), and each cover/slab/floor piece key
///    against the [`TerrainRegistry`] (GTW-396). On `Ok` the sim resources
///    ([`CoverLedger`] / [`SurfaceGrid`] / [`OccupancyGrid`] /
///    [`VerticalLinkGraph`] / [`FloorCostGrid`]) and the spawned ganger entities
///    land in the world, the [`BattleInProgress`] witness is inserted (the battle-active
///    tag the [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate) band
///    gates on), the [`PlayerFaction`] is inserted seeded from
///    [`Situation::player_faction`](crate::situation::Situation), the
///    [`BattleRoster`] is captured from the situation's fielded gangers' factions, the
///    [`ActiveFaction`] turn-cycle resource is seeded to the same player faction (the
///    player acts first; GTW-309), an EMPTY
///    [`SquadVisibility`](crate::visibility::SquadVisibility) squad fog is inserted
///    (GTW-341) — all sharing [`BattleInProgress`]'s lifetime — and a
///    [`BattleReady`] is written; on `Err` the typed
///    [`BattleSetupError`](crate::situation::BattleSetupError) (an invalid vertical link,
///    an unresolved weapon/armor/terrain key, or a below-minimum floor cost) is surfaced
///    via [`error!`] and NEITHER [`BattleInProgress`] / [`PlayerFaction`] /
///    [`BattleRoster`] / [`ActiveFaction`] NOR [`BattleReady`] is written — the app
///    never advances on a bad battle, and the gate never opens. NO
///    `unwrap`/`expect`/`panic`.
///
/// The [`WeaponRegistry`], [`ArmorRegistry`], and [`TerrainRegistry`] are each read as
/// `Option<Res<_>>` (PERSISTENT `Load` state); a setup requested before ANY loads fails
/// closed (logged, no [`BattleReady`]).
///
/// [`CombatTuning`](crate::tuning::CombatTuning) is NOT inserted here: it is E10.4's
/// PERSISTENT `Load` resource, present throughout the battle for the acts to read. It
/// IS read here as `Option<Res<_>>` to supply the `fallback_floor_cost`
/// (`CombatTuning::move_costs.open`) used when the situation omits `default_floor` or
/// when the `TerrainRegistry` is absent — preserving pre-GTW-396 behavior for
/// un-migrated test fixtures.
#[expect(
    clippy::too_many_arguments,
    reason = "GTW-396: eight params is one over the clippy 7-param default; the extra \
              param is `Option<Res<TerrainRegistry>>` added for terrain key resolution — \
              the Bevy system injection model cannot be refactored to fewer params without \
              introducing a wrapper resource that changes the API surface"
)]
pub fn setup_battle_on_request(
    mut requests: MessageReader<SetupBattleRequested>,
    mut ready: MessageWriter<BattleReady>,
    weapons: Option<Res<WeaponRegistry>>,
    armor: Option<Res<ArmorRegistry>>,
    terrain: Option<Res<TerrainRegistry>>,
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
        let Some(weapons) = weapons.as_deref() else {
            error!(
                "battle setup requested but no WeaponRegistry is loaded; no BattleReady will be \
                 signalled (the weapons folder must load before a battle starts)"
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
        // `default_floor` or when no TerrainRegistry is available. Sourced from
        // `CombatTuning::move_costs.open` (4 by default) to preserve pre-GTW-396
        // behavior for un-migrated test fixtures. Combat must never be blocked by
        // missing balance data.
        let default_combat_tuning = CombatTuning::default();
        let fallback_floor_cost = combat_tuning
            .as_deref()
            .map_or(default_combat_tuning.move_costs.open, |ct| {
                ct.move_costs.open
            });

        // GTW-396: the TerrainRegistry is PERSISTENT `Load` state (GTW-394), read as
        // `Option<Res<_>>`. When absent, `setup_battle` is called with `terrain: None`
        // which:
        //  (a) makes cover/slab keys fail with TerrainNotFound (a real situation's
        //      cover/slabs reference piece names that need the registry);
        //  (b) skips floor resolution and uses the fallback_floor_cost (correct for
        //      test fixtures that use SituationBuilder without a default_floor).
        // The real app always has the registry loaded before a battle starts (GTW-394
        // gates Load→Intro on it). Tests that use SituationBuilder without cover/slabs
        // pass `None` implicitly — they never authored terrain keys.
        let terrain_ref = terrain.as_deref();

        // 1. Seed the battle-lifetime RNG from the trigger's seed.
        commands.insert_resource(SimRng::from_seed(request.seed));

        // 2. Pour the situation into the world via the authoritative setup. A bad
        //    vertical link, a missing weapon/armor/terrain key, or a below-minimum
        //    floor cost returns the typed error — log it (NEVER panic / unwrap) and
        //    write NO BattleReady, so the app's gate never fires (fail-closed). The
        //    GangerStatTuning derives each ganger's computed stats from its eight
        //    authored attributes (GTW-384).
        match setup_battle(
            &request.situation,
            weapons,
            armor,
            stat_tuning,
            terrain_ref,
            fallback_floor_cost,
            &mut commands,
        ) {
            Ok(_setup) => {
                // The battle is live: insert the gate witness (alongside the setup_battle
                // grids + the seeded SimRng) so the Simulate band's bundled runtime turns
                // on, seed the PlayerFaction from the situation, capture the BattleRoster
                // from the fielded gangers' factions (both share the BattleInProgress
                // lifetime — same Ok path, removed together on teardown), then signal
                // BattleReady. All happen ONLY on Ok.
                //
                // NOTE: the FloorCostGrid is now inserted by setup_battle itself (GTW-396
                // Decision B — it is built from the resolved floor specs and inserted
                // directly). This is a change from the earlier sim-slice that inserted it
                // here: the full registry-driven grid now comes from setup_battle.
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
                // GTW-341: insert an EMPTY SquadVisibility on this same Ok path, sharing
                // the BattleInProgress lifetime. It starts empty; the BattleReady we write
                // below trips recompute_visibility (the SOLE writer) on the next update,
                // filling it with the spawn-time squad FOV — so the spawn FOV is covered by
                // the BattleReady trigger, not computed here (clause 4). The empty insert
                // makes the ResMut<SquadVisibility> read panic-free the moment the gated
                // Simulate band can run (bevy-traps.md #1).
                commands.insert_resource(SquadVisibility::default());
                ready.write(BattleReady);
            }
            Err(e) => {
                error!(
                    "battle setup failed: {e:?} (an invalid vertical link, an unresolved \
                     weapon/armor/terrain key, or a floor cost below the A* admissibility \
                     floor); no BattleReady will be signalled"
                );
            }
        }
    }
}

/// **Teardown** the battle on [`TeardownBattleRequested`] — remove the
/// battle-lifetime resources, including the [`BattleInProgress`] gate witness (E10.5
/// / GTW-212).
///
/// Drains [`MessageReader<TeardownBattleRequested>`] and, when triggered, removes
/// [`SimRng`], the [`setup_battle`]-inserted resources ([`CoverLedger`] /
/// [`SurfaceGrid`] / [`OccupancyGrid`] / [`VerticalLinkGraph`] /
/// [`SlabLedger`] / [`TerrainIndex`](crate::terrain::entity::TerrainIndex) /
/// [`FloorCostGrid`]), the [`BattleInProgress`] witness (closing the
/// [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate) gate so the
/// bundled runtime goes inert again), the [`PlayerFaction`], the [`BattleRoster`], the
/// [`ActiveFaction`] turn-cycle resource, and the
/// [`SquadVisibility`](crate::visibility::SquadVisibility) squad fog (GTW-341) (all
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
        commands.remove_resource::<SimRng>();
        commands.remove_resource::<CoverLedger>();
        commands.remove_resource::<SurfaceGrid>();
        // GTW-395: remove the SlabLedger (pre-existing leak fix — it was inserted at
        // setup.rs but never removed here until GTW-395. The teardown now owns it
        // alongside TerrainIndex for a clean battle boundary).
        commands.remove_resource::<SlabLedger>();
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
        // Remove the SquadVisibility alongside, so the squad fog's lifetime stays identical
        // to BattleInProgress (recompute_visibility — the sole writer — reads its
        // ResMut within that window; GTW-341).
        commands.remove_resource::<SquadVisibility>();
    }
}
