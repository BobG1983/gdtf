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
    surface::SurfaceGrid,
    turn::ActiveFaction,
    vertical::VerticalLinkGraph,
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
///    weapon key against the [`WeaponRegistry`] (GTW-257) and each ganger's armor key
///    against the [`ArmorRegistry`] (GTW-269). On `Ok` the four sim
///    resources ([`CoverLedger`] / [`SurfaceGrid`] / [`OccupancyGrid`] /
///    [`VerticalLinkGraph`]) and the spawned ganger entities (each armed with its
///    resolved [`WeaponBundle`](crate::weapon::WeaponBundle)) land in the world, the
///    [`BattleInProgress`] witness is inserted (the battle-active tag the
///    [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate) band gates on),
///    the [`PlayerFaction`] is inserted seeded from
///    [`Situation::player_faction`](crate::situation::Situation), the
///    [`BattleRoster`] is captured from the situation's fielded gangers' factions, the
///    [`ActiveFaction`] turn-cycle resource is seeded to the same player faction (the
///    player acts first; GTW-309) — all sharing [`BattleInProgress`]'s lifetime — and a
///    [`BattleReady`] is written; on `Err` the typed
///    [`BattleSetupError`](crate::situation::BattleSetupError) (an invalid vertical link,
///    an unresolved weapon key, OR an unresolved armor key) is surfaced via [`error!`] and NEITHER
///    [`BattleInProgress`] / [`PlayerFaction`] / [`BattleRoster`] / [`ActiveFaction`] NOR
///    [`BattleReady`] is written — the app never advances on a bad battle, and the gate
///    never opens. NO `unwrap`/`expect`/`panic`.
///
/// The [`WeaponRegistry`] and [`ArmorRegistry`] are each read as `Option<Res<_>>`
/// (PERSISTENT `Load` state like [`CombatTuning`](crate::tuning::CombatTuning)); a setup
/// requested before EITHER loads fails closed (logged, no [`BattleReady`]).
/// [`CombatTuning`](crate::tuning::CombatTuning) is NOT inserted here: it is E10.4's
/// PERSISTENT `Load` resource, present throughout the battle for the acts to read.
pub fn setup_battle_on_request(
    mut requests: MessageReader<SetupBattleRequested>,
    mut ready: MessageWriter<BattleReady>,
    weapons: Option<Res<WeaponRegistry>>,
    armor: Option<Res<ArmorRegistry>>,
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

        // 1. Seed the battle-lifetime RNG from the trigger's seed.
        commands.insert_resource(SimRng::from_seed(request.seed));

        // 2. Pour the situation into the world via the authoritative setup. A bad
        //    vertical link, a missing weapon key, OR a missing armor key returns the
        //    typed error — log it (NEVER panic / unwrap) and write NO BattleReady, so
        //    the app's gate never fires (fail-closed).
        match setup_battle(&request.situation, weapons, armor, &mut commands) {
            Ok(_setup) => {
                // The battle is live: insert the gate witness (alongside the four
                // setup_battle grids + the seeded SimRng) so the Simulate band's bundled
                // runtime turns on, seed the PlayerFaction from the situation, capture the
                // BattleRoster from the fielded gangers' factions (both share the
                // BattleInProgress lifetime — same Ok path, removed together on teardown),
                // then signal BattleReady. All happen ONLY on Ok.
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
                ready.write(BattleReady);
            }
            Err(error) => {
                error!(
                    "battle setup failed: {error:?} (an invalid vertical link, an unresolved \
                     weapon key, or an unresolved armor key); no BattleReady will be signalled"
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
/// [`SimRng`], the four [`setup_battle`]-inserted resources ([`CoverLedger`] /
/// [`SurfaceGrid`] / [`OccupancyGrid`] / [`VerticalLinkGraph`]), the
/// [`BattleInProgress`] witness (closing the
/// [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate) gate so the
/// bundled runtime goes inert again), the [`PlayerFaction`], the [`BattleRoster`], and the
/// [`ActiveFaction`] turn-cycle resource (all lifetimes track [`BattleInProgress`], so they
/// are removed in the same teardown). These resources are BATTLE-lifetime: the app
/// sends this trigger only at the battle boundary (its `OnExit(GameState::BattleScape)`),
/// so they survive the whole battle for the E10.6 acts before being cleaned
/// (`bevy-traps.md` #1 at the correct state level).
///
/// [`CombatTuning`](crate::tuning::CombatTuning) is deliberately NOT removed: it is
/// E10.4's persistent `Load` resource, untouched by this plugin. A
/// [`remove_resource`](Commands::remove_resource) on an absent resource is a no-op,
/// so a spurious / double teardown is harmless.
pub fn teardown_battle_on_request(
    mut requests: MessageReader<TeardownBattleRequested>,
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
        commands.remove_resource::<OccupancyGrid>();
        commands.remove_resource::<VerticalLinkGraph>();
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
    }
}
