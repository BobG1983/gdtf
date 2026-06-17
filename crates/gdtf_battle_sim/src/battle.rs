//! The sim-owned **battle-lifecycle integration**: the public [`BattleSimPlugin`]
//! that wires the whole render-free combat runtime into a Bevy [`App`], plus the
//! three message-driven lifecycle types the app drives it with (E10.5 / GTW-207).
//!
//! This module is the model's OWN integration seam
//! (`docs/decisions/0001-rust-bevy-rewrite.md`: the model is the authoritative
//! render-free sim, consumed ONE-WAY by the app). Because the dependency edge is
//! `gdtf_app -> gdtf_battle_sim` (E10.1), the sim CANNOT and MUST NOT name any
//! `gdtf_app` type — no `AppState` / `BattleScapeState` / `GenerationComplete` /
//! `LoadedSituation`. So the battle lifecycle is decoupled from the app's state
//! machine via buffered [`Message`]s: the app SENDS triggers (naming its own app
//! states, app-side) and the sim ACTS on them (naming only sim types). This matches
//! the message-driven canon E10.2 established (the app emits a `*Requested`, the sim
//! listens — `bevy-traps.md` #4: buffered [`Message`], NOT the observer `Event`).
//!
//! ## The three lifecycle messages
//!
//! - [`SetupBattleRequested`] — the SETUP trigger: an OWNED [`Situation`] (it is
//!   [`Clone`]) + a [`BattleSeed`] (a [`Copy`] newtype). No lifetime, no app type.
//! - [`TeardownBattleRequested`] — the TEARDOWN trigger.
//! - [`BattleReady`] — the setup-complete SIGNAL the app gates its state advance on.
//!
//! ## [`BattleSimPlugin`]
//!
//! Adding this ONE plugin wires the whole sim runtime: it bundles
//! [`OccupancyMaintenancePlugin`] (the E1.7 maintenance layer + the
//! [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed) buffer) and the E10.2
//! [`SimActsPlugin`] (the per-act dispatch + the six `*Requested` buffers), registers
//! the three lifecycle messages once each, and adds a setup + a teardown system in
//! [`Update`], ordered around the [`SimSystems::Simulate`] band (E10.0's set).
//!
//! Because the bundled dispatch / occupancy systems read the battle-lifetime resources
//! `unconditionally` (a Bevy `Res<T>` whose resource is absent fails param validation —
//! `bevy-traps.md` #1), and those resources exist only DURING a battle, the plugin
//! gates the whole `Simulate` band on the purpose-built [`BattleInProgress`] witness so
//! the bundled runtime stays inert (and panic-free) outside a live battle; the setup /
//! teardown lifecycle systems run unconditionally `around` the band (setup creates the
//! witness, teardown removes it) — see [`BattleSimPlugin`]. [`BattleInProgress`] is an
//! explicit, intentional "a battle is active" tag — it replaced the incidental
//! [`OccupancyGrid`]-as-gate proxy (GTW-212), but spans the exact same battle-active
//! window ([`OccupancyGrid`] remains a [`setup_battle`] resource, just no longer the
//! gate witness), so the gated span — and behavior — is unchanged.
//!
//! The setup system drains [`SetupBattleRequested`] and per message inserts a
//! [`SimRng`] seeded from the message's [`BattleSeed`] and runs [`setup_battle`],
//! emitting [`BattleReady`] (and inserting the [`BattleInProgress`] gate witness) only
//! on success (`Err` is `error!`-logged with NO [`BattleReady`] / NO witness —
//! fail-closed, no `unwrap`/`expect`/`panic`). The teardown system drains
//! [`TeardownBattleRequested`] and removes the battle-lifetime resources ([`SimRng`] +
//! the four [`setup_battle`]-inserted grids + the [`BattleInProgress`] witness); it
//! NEVER touches [`CombatTuning`](crate::tuning::CombatTuning) — that is E10.4's
//! persistent `Load` resource. Render-free: no renderer, window, presenter, or pixel.

use bevy::{
    platform::collections::HashSet,
    prelude::{
        App, Commands, Deref, IntoScheduleConfigs, Local, Message, MessageReader, MessageWriter,
        Plugin, Query, Res, Resource, Update, error, resource_exists,
    },
};

use crate::{
    acts::SimActsPlugin,
    cover::CoverLedger,
    ganger::{Faction, LifeState},
    occupancy::OccupancyGrid,
    occupancy_sync::{OccupancyMaintenancePlugin, SimSystems},
    rng::{BattleSeed, SimRng},
    situation::{Situation, setup_battle},
    surface::SurfaceGrid,
    vertical::VerticalLinkGraph,
    weapon::WeaponRegistry,
};

/// A **setup-battle** trigger — build the battle from `situation`, seeding the
/// model RNG from `seed`.
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), carrying
/// the act's OWNED payload: the authored [`Situation`] (it is [`Clone`], so the app
/// clones its `LoadedSituation` into the trigger by value) plus the [`BattleSeed`]
/// (a [`Copy`] newtype). A [`Message`] cannot hold a borrow, so both are owned —
/// the type has **no lifetime parameter** and names NO `gdtf_app` type, keeping the
/// sim free of the one-way app dependency.
#[derive(Message, Debug, Clone)]
pub struct SetupBattleRequested {
    /// The authored battlefield to pour into the ECS world — owned by value (the app
    /// clones its `LoadedSituation`, or sends [`Situation::default`] when absent).
    pub situation: Situation,
    /// The seed the battle [`SimRng`] is built from — threaded through
    /// [`SimRng::from_seed`], so the same seed reproduces the same draw stream.
    pub seed:      BattleSeed,
}

impl SetupBattleRequested {
    /// Build a setup-battle trigger for `situation`, seeding the RNG from `seed`.
    #[must_use]
    pub const fn new(situation: Situation, seed: BattleSeed) -> Self {
        Self { situation, seed }
    }
}

/// A **teardown-battle** trigger — remove the battle-lifetime sim resources.
///
/// A buffered [`Message`] the app sends at the battle boundary (its
/// `OnExit(GameState::BattleScape)`). Carries no payload: the teardown removes a
/// fixed set of resources ([`SimRng`] + the four [`setup_battle`]-inserted grids). A
/// unit-payload struct (not an enum / not a field) — the trigger's identity IS the
/// signal.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TeardownBattleRequested;

/// The **battle-ready** signal — [`setup_battle`] succeeded and the battle-lifetime
/// resources are in the world.
///
/// A buffered [`Message`] the SETUP system writes on `Ok` (and NEVER on `Err` —
/// fail-closed). The app reads it to gate its state advance: it advances out of its
/// generation phase strictly AFTER the sim reports ready, and stays put on a failed
/// or absent signal. A unit-payload struct — the signal's identity IS the message.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BattleReady;

/// The **battle-won** signal — every enemy gang fielded at setup is out of the fight
/// (all `Dead`/`Downed`) while a player ganger still stands (GTW-237). The win condition
/// is design canon — see the "Battle outcome (win / loss)" beat in `docs/combat/combat.md`.
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) the
/// [`check_outcome`] census writes the first time the win condition holds (then latches,
/// so it fires at most once per battle). A zero-field unit struct — its identity IS the
/// signal, so (like [`BattleReady`]/[`BattleInProgress`]) the no-bare-types rule, which
/// wraps domain *values*, does not apply. This is a SIM SIGNAL only: the sim OWNS the
/// buffer; it does NOT drive any `gdtf_app` state, despawn anything, end the battle, or
/// touch the presenter — the app-side consumer that ends the battle is GTW-239.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BattleWon;

/// The **battle-lost** signal — every player ganger fielded at setup is out of the fight
/// (all `Dead`/`Downed`), so the player gang is wiped (GTW-237). The loss condition (and
/// the mutual-wipe = loss resolution below) is design canon — see the "Battle outcome
/// (win / loss)" beat in `docs/combat/combat.md`.
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) the
/// [`check_outcome`] census writes the first time the loss condition holds (then latches,
/// so it fires at most once per battle). A zero-field unit struct — its identity IS the
/// signal, so (like [`BattleReady`]/[`BattleInProgress`]) the no-bare-types rule, which
/// wraps domain *values*, does not apply. This is a SIM SIGNAL only: the sim OWNS the
/// buffer; it does NOT drive any `gdtf_app` state, despawn anything, end the battle, or
/// touch the presenter — the app-side consumer that ends the battle is GTW-239. A mutual
/// wipe (last enemy and last player fall together) resolves HERE, not as a win:
/// [`check_outcome`]'s win arm requires a surviving player, so a mutual wipe is a LOSS.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BattleLost;

/// The **battle-active** witness — present exactly while a battle is in progress, the
/// gate the [`SimSystems::Simulate`] band's bundled runtime keys off (GTW-212).
///
/// A zero-sized [`Resource`] marker (a unit struct): it carries NO domain value, so the
/// no-bare-types rule — which wraps *values* in named newtypes — does not apply; the
/// tag's identity IS the signal. The setup system inserts it on a successful
/// [`setup_battle`] (the same `Ok` path that emits [`BattleReady`], NEVER on `Err`) and
/// the teardown system removes it on [`TeardownBattleRequested`], so it spans the exact
/// battle-active window.
///
/// It is the EXPLICIT replacement for the incidental "gate on
/// [`resource_exists`]`::<`[`OccupancyGrid`]`>`" proxy E10.5 used: keying the
/// [`SimSystems::Simulate`] `run_if` on a purpose-built "in progress" tag decouples the
/// gate's INTENT from any one battle resource. [`OccupancyGrid`] is still a
/// [`setup_battle`] resource (inserted on setup, removed on teardown) — it is simply no
/// longer the gate witness. Because the tag is inserted/removed at the same points
/// [`OccupancyGrid`] effectively was, the gated span — and the runtime's behavior — is
/// unchanged; only the witness is now intentional.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BattleInProgress;

/// The **player faction** — which gang the human player controls; every other
/// [`Faction`] is the enemy.
///
/// A named newtype [`Resource`] over the [`Faction`] gang index (no-bare-types:
/// it carries a domain value — which gang the player runs — so it is a real named
/// newtype over [`Faction`], NOT the bare [`Faction`] and NOT a zero-sized marker
/// like [`BattleInProgress`]). The derived [`Deref`] reads the inner [`Faction`]
/// back (never a hand-written `impl Deref`).
///
/// It is the single source of truth the later manual-play slices read: control-gating
/// lets the player act only on this gang's gangers, and the victory census polls every
/// gang that is NOT this one. Seeded data-driven from
/// [`Situation::player_faction`](crate::situation::Situation) on setup.
///
/// **Lifetime tracks [`BattleInProgress`]:** the setup system inserts it on the same
/// successful-setup `Ok` path that inserts [`BattleInProgress`] (NEVER on `Err` —
/// fail-closed) and the teardown system removes it alongside, so it is present for
/// exactly the battle-active window. Any later `Res<PlayerFaction>` reader must
/// therefore be gated on [`resource_exists`]`::<`[`BattleInProgress`]`>` (the identical
/// window) or take `Option<Res<_>>`, or it panics when the resource is absent
/// (`bevy-traps.md` #1).
#[derive(Resource, Deref, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerFaction(Faction);

impl PlayerFaction {
    /// Build the player-faction resource from the gang the player controls.
    ///
    /// The public constructor (house style) so the setup can build a `PlayerFaction`
    /// from an authored [`Situation::player_faction`](crate::situation::Situation)
    /// without reaching the private field.
    #[must_use]
    pub const fn new(faction: Faction) -> Self {
        Self(faction)
    }
}

/// The **battle roster** — the set of [`Faction`]s that fielded ≥1 ganger at setup
/// (GTW-237).
///
/// A named newtype [`Resource`] (no-bare-types: it carries a domain value — which gangs
/// took the field — so it wraps the fielded-factions set in a real named type, NOT a bare
/// [`HashSet`]). The collection it owns is a [`HashSet`]`<`[`Faction`]`>` ([`Faction`] is
/// [`Copy`]/[`Eq`]/[`Hash`]). Private inner; the small accessors the census needs read it
/// back (never a hand-written `impl Deref`, because the census asks roster QUESTIONS, not
/// for the raw set).
///
/// It is the source of the "are there enemy gangers?" / "was the player fielded?"
/// EXISTENCE facts the [`check_outcome`] census splits on — grounded in the gangs FIELDED
/// at setup, NOT in a per-frame entity scan. The earlier victory-only design fired on
/// "≥1 non-player entity is in the query this frame", which couples the win to dead
/// gangers PERSISTING as entities (true today — the sim has no despawn-on-death — but it
/// would silently break if despawn is ever added). Grounding existence in the roster makes
/// the win/loss robust regardless of whether dead gangers are ever despawned: the live
/// [`LifeState`] scan answers only "is any of them still up?".
///
/// **Lifetime tracks [`BattleInProgress`]:** captured on the same successful-setup `Ok`
/// path that inserts [`BattleInProgress`] + [`PlayerFaction`] (NEVER on `Err` —
/// fail-closed) and removed alongside on teardown, so it is present for exactly the
/// battle-active window. The [`SimSystems::Simulate`]-band census therefore reads its
/// `Res<BattleRoster>` panic-free (the band is gated on
/// [`resource_exists`]`::<`[`BattleInProgress`]`>` — the identical window;
/// `bevy-traps.md` #1).
///
/// A faction SET, not per-faction counts — sufficient for the binary win/loss existence
/// checks. If a later procgen-roster slice (GTW-243) needs counts, it can enrich.
///
/// Grounding existence in the fielded roster (not a live scan) is part of the battle-outcome
/// canon — see the "Battle outcome (win / loss)" beat in `docs/combat/combat.md`.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct BattleRoster(HashSet<Faction>);

impl BattleRoster {
    /// Build the battle roster from the [`Faction`]s that fielded a ganger.
    ///
    /// The public constructor (house style) so the setup can build a `BattleRoster` from
    /// the situation's gangers (each [`GangerSpawn::faction`](crate::situation::GangerSpawn))
    /// without reaching the private field.
    #[must_use]
    pub fn new(factions: impl IntoIterator<Item = Faction>) -> Self {
        Self(factions.into_iter().collect())
    }

    /// Whether the `player` gang was fielded at setup — the players-fielded existence
    /// fact the loss census splits on.
    #[must_use]
    pub fn has_player(&self, player: Faction) -> bool {
        self.0.contains(&player)
    }

    /// Whether ≥1 fielded faction is NOT the `player` gang — the enemies-fielded existence
    /// fact the win census splits on. An empty enemy roster (player-only / no fielded
    /// faction) is `false`, so the census never wins on a degenerate roster.
    #[must_use]
    pub fn has_enemy_of(&self, player: Faction) -> bool {
        self.0.iter().any(|&faction| faction != player)
    }
}

/// **Setup** the battle on [`SetupBattleRequested`] — seed the [`SimRng`] and run
/// [`setup_battle`], signalling [`BattleReady`] on success (E10.5).
///
/// Drains [`MessageReader<SetupBattleRequested>`] and per message:
///
/// 1. Inserts the battle-lifetime [`SimRng`] seeded from the message's [`BattleSeed`]
///    (via [`SimRng::from_seed`] — the deterministic stream the acts draw from).
/// 2. Calls [`setup_battle`] on the REAL [`Commands`] path, resolving each ganger's
///    weapon key against the [`WeaponRegistry`] (GTW-257). On `Ok` the four sim
///    resources ([`CoverLedger`] / [`SurfaceGrid`] / [`OccupancyGrid`] /
///    [`VerticalLinkGraph`]) and the spawned ganger entities (each armed with its
///    resolved [`WeaponBundle`](crate::weapon::WeaponBundle)) land in the world, the
///    [`BattleInProgress`] witness is inserted (the battle-active tag the
///    [`SimSystems::Simulate`] band gates on), the [`PlayerFaction`] is inserted seeded
///    from [`Situation::player_faction`](crate::situation::Situation), the
///    [`BattleRoster`] is captured from the situation's fielded gangers' factions (both
///    sharing [`BattleInProgress`]'s lifetime), and a [`BattleReady`] is written; on `Err`
///    the typed [`BattleSetupError`](crate::situation::BattleSetupError) (an invalid
///    vertical link OR an unresolved weapon key) is surfaced via [`error!`] and NEITHER
///    [`BattleInProgress`] / [`PlayerFaction`] / [`BattleRoster`] NOR [`BattleReady`] is
///    written — the app never advances on a bad battle, and the gate never opens. NO
///    `unwrap`/`expect`/`panic`.
///
/// The [`WeaponRegistry`] is read as `Option<Res<_>>` (PERSISTENT `Load` state like
/// [`CombatTuning`](crate::tuning::CombatTuning)); a setup requested before it loads
/// fails closed (logged, no [`BattleReady`]). [`CombatTuning`](crate::tuning::CombatTuning)
/// is NOT inserted here: it is E10.4's PERSISTENT `Load` resource, present throughout the
/// battle for the acts to read.
pub fn setup_battle_on_request(
    mut requests: MessageReader<SetupBattleRequested>,
    mut ready: MessageWriter<BattleReady>,
    weapons: Option<Res<WeaponRegistry>>,
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

        // 1. Seed the battle-lifetime RNG from the trigger's seed.
        commands.insert_resource(SimRng::from_seed(request.seed));

        // 2. Pour the situation into the world via the authoritative setup. A bad
        //    vertical link OR a missing weapon key returns the typed error — log it
        //    (NEVER panic / unwrap) and write NO BattleReady, so the app's gate never
        //    fires (fail-closed).
        match setup_battle(&request.situation, weapons, &mut commands) {
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
                ready.write(BattleReady);
            }
            Err(error) => {
                error!(
                    "battle setup failed: {error:?} (an invalid vertical link or an unresolved \
                     weapon key); no BattleReady will be signalled"
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
/// [`BattleInProgress`] witness (closing the [`SimSystems::Simulate`] gate so the
/// bundled runtime goes inert again), the [`PlayerFaction`], and the [`BattleRoster`]
/// (both lifetimes track [`BattleInProgress`], so they are removed in the same teardown).
/// These resources are BATTLE-lifetime: the app
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
    }
}

/// **Check the battle outcome** — emit [`BattleWon`] when every fielded enemy ganger is
/// out of the fight (a player still up), and [`BattleLost`] when every player ganger is
/// out (GTW-237).
///
/// The win/loss conditions, the "out of the fight" = `Downed`/`Dead` rule, the mutual-wipe
/// = loss resolution, and the roster-grounded (not scan-grounded) existence test are
/// design canon — see the "Battle outcome (win / loss)" beat in `docs/combat/combat.md`
/// and the "out of the fight" glossary entry. This system is that canon's sim signal.
///
/// The sim's roster-grounded outcome census. A query/resource/local system (NO `&mut
/// World` — `bevy-traps.md` #7). It runs `.in_set(`[`SimSystems::Simulate`]`)`, the band
/// gated on [`resource_exists`]`::<`[`BattleInProgress`]`>`, so it is INERT outside a live
/// battle and its [`Res<PlayerFaction>`] + [`Res<BattleRoster>`] reads (both
/// battle-lifetime) are panic-free (`bevy-traps.md` #1).
///
/// Two sources, kept separate by design:
///
/// - **Existence** comes from the [`BattleRoster`] (the gangs fielded at setup), NOT a
///   per-frame entity scan: `enemies_fielded = roster.has_enemy_of(*player)`,
///   `players_fielded = roster.has_player(*player)`. This is the bug-2 fix — the win is
///   robust regardless of whether dead gangers are ever despawned.
/// - **Liveness** comes from the live [`Query`]`<(&`[`Faction`]`, &`[`LifeState`]`)>`: a
///   ganger is "up" iff [`LifeState::Alive`] ([`LifeState::Downed`] AND [`LifeState::Dead`]
///   both count OUT, per the incapacitated semantics). `any_enemy_up` =
///   ∃ a non-player Alive ganger; `any_player_up` = ∃ a player Alive ganger.
///
/// Emits [`BattleWon`] iff `enemies_fielded && !any_enemy_up && any_player_up` and not
/// already emitted (then latches its [`Local<bool>`]); emits [`BattleLost`] iff
/// `players_fielded && !any_player_up` and not already emitted (then latches). The two
/// [`Local<bool>`] latches keep each outcome to at most ONE emit per battle (no per-frame
/// spam). Requiring a surviving player for the WIN makes the two outcomes mutually
/// exclusive — a MUTUAL WIPE (last enemy and last player fall together) resolves to
/// [`BattleLost`], not [`BattleWon`]. Player gangers are never inspected for the win; enemy
/// gangers never for the loss.
pub fn check_outcome(
    mut won: MessageWriter<BattleWon>,
    mut lost: MessageWriter<BattleLost>,
    gangers: Query<(&Faction, &LifeState)>,
    player: Res<PlayerFaction>,
    roster: Res<BattleRoster>,
    mut won_emitted: Local<bool>,
    mut lost_emitted: Local<bool>,
) {
    let player = **player;

    // Existence from the ROSTER (the gangs fielded at setup), not the live scan.
    let enemies_fielded = roster.has_enemy_of(player);
    let players_fielded = roster.has_player(player);

    // Liveness from the live scan: "up" iff Alive (Downed AND Dead both count OUT).
    let mut any_enemy_up = false;
    let mut any_player_up = false;
    for (&faction, &life) in &gangers {
        if life != LifeState::Alive {
            continue;
        }
        if faction == player {
            any_player_up = true;
        } else {
            any_enemy_up = true;
        }
    }

    // WIN: every fielded enemy is out AND a player still stands (so a mutual wipe is a
    // LOSS, not a win) — at most once per battle.
    if enemies_fielded && !any_enemy_up && any_player_up && !*won_emitted {
        won.write(BattleWon);
        *won_emitted = true;
    }
    // LOSS: every player ganger is out — at most once per battle. Enemy liveness is
    // irrelevant: the player losing is a loss whether or not enemies remain.
    if players_fielded && !any_player_up && !*lost_emitted {
        lost.write(BattleLost);
        *lost_emitted = true;
    }
}

/// The sim's **battle-lifecycle registration unit** — adding this ONE plugin wires
/// the whole render-free combat runtime into a Bevy [`App`] (E10.5 / GTW-207).
///
/// In `build()` the plugin:
///
/// - adds [`OccupancyMaintenancePlugin`] (the E1.7 maintenance layer + the
///   [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed) buffer; it OWNS the
///   [`SimSystems::Simulate`] `configure_sets`) and the E10.2 [`SimActsPlugin`] (the
///   six `*Requested` buffers + their per-act dispatch systems) — so the app adds
///   only THIS plugin to get the full sim runtime;
/// - [`add_message`](App::add_message)s the three lifecycle types
///   ([`SetupBattleRequested`] / [`TeardownBattleRequested`] / [`BattleReady`]) plus the
///   two GTW-237 outcome witnesses ([`BattleWon`] / [`BattleLost`]) exactly once each
///   (`bevy-traps.md` #5 — an unregistered buffer fails a [`MessageReader`]'s param
///   validation);
/// - adds the roster-grounded [`check_outcome`] census `.in_set(`[`SimSystems::Simulate`]`)`
///   — so it rides the same [`BattleInProgress`] gate as the bundled runtime (inert and
///   panic-free outside a live battle); and
/// - adds [`setup_battle_on_request`] + [`teardown_battle_on_request`] to [`Update`],
///   ordered around the [`SimSystems::Simulate`] band (setup `.before`, teardown
///   `.after`), so the battle-lifetime resources are created before the bundled
///   in-set systems read them and removed only after they have run.
///
/// **Battle-scoped gating of the bundled runtime.** The bundled [`SimActsPlugin`] +
/// [`OccupancyMaintenancePlugin`] systems (all `.in_set(SimSystems::Simulate)`) read
/// the battle-lifetime resources ([`OccupancyGrid`] / [`SurfaceGrid`] /
/// [`CoverLedger`] / [`SimRng`] / [`CombatTuning`](crate::tuning::CombatTuning))
/// unconditionally — a Bevy `Res<T>` whose resource is absent fails param validation
/// (`bevy-traps.md` #1). Those resources exist only between a setup and a teardown, so
/// `BattleSimPlugin` gates the whole `Simulate` band on
/// [`resource_exists`]`::<`[`BattleInProgress`]`>` (the setup-inserted "a battle is
/// active" witness — GTW-212's purpose-built tag, replacing the incidental
/// [`OccupancyGrid`]-as-gate proxy) — a `run_if` skips a set's member systems entirely
/// (no param validation) when false, so the bundled runtime stays inert (and
/// panic-free) before the first battle and after teardown. The setup inserts
/// [`BattleInProgress`] on the same `Ok` path that inserts [`OccupancyGrid`] and the
/// teardown removes it alongside, so the gated span is identical — only the witness is
/// explicit. The lifecycle systems are deliberately NOT in that gated band: the setup
/// system must run to CREATE the witness (it would otherwise dead-gate itself), and the
/// teardown system must run AFTER the band to remove the witness — so both run
/// unconditionally, ordered around the band.
///
/// The plugin names NO `gdtf_app` type: the battle lifecycle is driven entirely by
/// the three lifecycle messages, so the app sends triggers and reads the signal
/// without the sim ever depending back on the app.
#[derive(Debug, Default, Clone, Copy)]
pub struct BattleSimPlugin;

impl Plugin for BattleSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(OccupancyMaintenancePlugin)
            .add_plugins(SimActsPlugin)
            .add_message::<SetupBattleRequested>()
            .add_message::<TeardownBattleRequested>()
            .add_message::<BattleReady>()
            // The roster-grounded outcome witnesses the Simulate-band census emits (GTW-237;
            // bevy-traps.md #5 — an unregistered buffer fails a MessageReader's validation).
            .add_message::<BattleWon>()
            .add_message::<BattleLost>()
            // Gate the whole Simulate band (the bundled dispatch + occupancy systems)
            // on the setup-inserted BattleInProgress witness — the purpose-built
            // "a battle is active" tag (GTW-212), replacing the incidental OccupancyGrid
            // proxy — so the bundled runtime is inert + panic-free outside a live battle
            // (bevy-traps.md #1; conditions still accumulate across configure_sets #5).
            .configure_sets(
                Update,
                SimSystems::Simulate.run_if(resource_exists::<BattleInProgress>),
            )
            // The roster-grounded outcome census joins the gated Simulate band, so it is
            // INERT (and its Res<PlayerFaction>/Res<BattleRoster> reads panic-free) outside
            // a live battle — same window as BattleInProgress (GTW-237). No configure_sets
            // (the set is owned upstream by OccupancyMaintenancePlugin).
            .add_systems(Update, check_outcome.in_set(SimSystems::Simulate))
            // The lifecycle drivers run UNCONDITIONALLY (outside the gated band): setup
            // before the band (it creates the witness), teardown after (it removes it).
            .add_systems(
                Update,
                (
                    setup_battle_on_request.before(SimSystems::Simulate),
                    teardown_battle_on_request.after(SimSystems::Simulate),
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        ecs::message::Messages,
        prelude::{App, MinimalPlugins, World},
    };

    use super::*;
    use crate::{
        acts::FireRequested,
        armor::{
            ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
            SourceArmor, WornArmor,
        },
        ganger::{
            Aiming, Direction, Facing, Faction, Hp, LifeState, Luck, Shooting, Stance, StanceKind,
            Toughness, Tu, Wounds,
        },
        metric::{Cell, CellLevel, Level},
        occupancy_sync::CoverDestroyed,
        situation::{BattleSetupError, GangerSpawn},
        tuning::CombatTuning,
        vertical::{InvalidVerticalLink, LinkKind, VerticalLink},
        weapon::{
            Accuracy, BaseSpread, DamageType, FatalBias, FireMode, FireModeSpec, Kickback,
            MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Stable, WeaponDamage,
            WeaponName, WeaponPunch, WeaponRegistry, WeaponShred, WeaponSpec,
        },
    };

    /// An arbitrary (NOT shipped tuning) seed for a test battle's RNG stream.
    const SEED: u64 = 0x5A1C_AC75;

    /// The weapon KEY every fixture ganger references — present in the registry the
    /// `headless_app` inserts (so a setup arms each ganger; GTW-257).
    const TEST_WEAPON_KEY: &str = "test-weapon";

    /// An arbitrary [`WeaponSpec`] (NOT shipped magnitudes — mechanism only) for the
    /// one [`TEST_WEAPON_KEY`] the fixture gangers reference.
    fn arbitrary_weapon_spec() -> WeaponSpec {
        WeaponSpec {
            base_spread:   BaseSpread::new(0.25),
            accuracy:      Accuracy::new(1.0),
            kickback:      Kickback::new(0.4),
            fatal_bias:    FatalBias::new(7.0),
            damage:        WeaponDamage::new(12),
            punch:         WeaponPunch::new(5),
            shred:         WeaponShred::new(3),
            damage_type:   DamageType::Kinetic,
            magazine_size: MagazineSize::new(30),
            fire_mode:     FireMode::new(vec![FireModeSpec::new(
                ModeKind::Single,
                ModeConeMult::new(1.0),
                ModeTuPercent::new(0.5),
                ModeShots::new(1),
            )]),
            stable:        Stable::new(false),
        }
    }

    /// The test [`WeaponRegistry`] — the one [`TEST_WEAPON_KEY`] weapon the fixture
    /// gangers reference, standing in for the app's `Load`-built registry (always
    /// present before a battle in the real app).
    fn weapon_registry() -> WeaponRegistry {
        WeaponRegistry::new([(
            WeaponName::new(TEST_WEAPON_KEY.to_owned()),
            arbitrary_weapon_spec(),
        )])
    }

    /// Build a `(cell, level)` key from raw coordinates.
    fn key(x: i32, y: i32, level: u8) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(level))
    }

    /// An arbitrary roster-armor record (distinct per-part magnitudes, NOT shipped
    /// tuning) so a fixture ganger carries a faithful `SourceArmor`.
    fn arbitrary_armor(base: i32) -> SourceArmor {
        SourceArmor::uniform(ArmorPiece::new(
            ArmorFloor::new(base),
            ArmorProtection::new(base + 1),
            ArmorIntegrity::new(base + 2),
            ArmorHardness::new(base + 3),
            ArmorType::DEFAULT,
        ))
    }

    /// Build an authored ganger at `at` with arbitrary-but-valid component values.
    fn ganger_at(at: CellLevel, faction: u8) -> GangerSpawn {
        GangerSpawn {
            at,
            faction: Faction::new(faction),
            facing: Facing::new(Direction::East),
            stance: Stance::new(StanceKind::Crouching),
            aiming: Aiming::new(true),
            hp: Hp::new(40),
            wounds: Wounds::new(3),
            tu: Tu::new(60),
            life_state: LifeState::Alive,
            shooting: Shooting::new(f32::from(faction) + 2.0),
            toughness: Toughness::new(f32::from(faction) + 3.0),
            luck: Luck::new(f32::from(faction) + 1.0),
            armor: arbitrary_armor(i32::from(faction) + 1),
            // Every fixture ganger references the one TEST_WEAPON_KEY in weapon_registry.
            weapon: WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        }
    }

    /// A valid two-ganger fixture situation (no cover / slabs / links — link-free
    /// validates trivially). Omits `player_faction`, so the struct-level
    /// `#[serde(default)]` / [`Default`] supplies [`Faction::default`] = `Faction(0)`
    /// (the AC2 default-seed precondition).
    fn two_ganger_situation() -> Situation {
        Situation {
            gangers: vec![ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)],
            ..Situation::new()
        }
    }

    /// The two-ganger fixture with `player_faction` AUTHORED to gang 1 (overriding the
    /// `Faction(0)` default) — the AC3 fixture proving the seed reads
    /// `situation.player_faction`, not a hardcoded gang 0.
    fn two_ganger_situation_player_faction_one() -> Situation {
        Situation {
            player_faction: Faction::new(1),
            ..two_ganger_situation()
        }
    }

    /// A situation with a DANGLING vertical link (an endpoint at a `(cell, level)` no
    /// authored tile occupies) — `setup_battle` returns `Err(DanglingCell)` and
    /// inserts no resource (the `setup_aborts_on_invalid_vertical_link` precedent).
    fn dangling_link_situation() -> (Situation, VerticalLink) {
        let present = key(4, 4, 0);
        let missing = key(4, 4, 1); // never authored — the link dangles off it
        let link = VerticalLink::new(present, missing, LinkKind::stair());
        let situation = Situation {
            gangers: vec![ganger_at(key(0, 0, 0), 0)],
            slabs: vec![present], // only `present` authored; `missing` dangles
            vertical_links: vec![link],
            ..Situation::new()
        };
        (situation, link)
    }

    /// Build a headless app: [`MinimalPlugins`] (no window / renderer) +
    /// [`BattleSimPlugin`] — the `occupancy_sync` / `acts` headless precedent. The sim
    /// crate alone, proving NO `gdtf_app` coupling.
    ///
    /// Inserts [`CombatTuning::default`] up front, standing in for E10.4's PERSISTENT
    /// `Load` resource (always present in the real app before a battle): once a setup
    /// inserts the [`OccupancyGrid`] witness, the gated `Simulate` band runs the bundled
    /// dispatch systems, which read `CombatTuning` — so it must be present (the
    /// `acts.rs::insert_sim_resources` precedent). It is deliberately NOT one of the
    /// battle-lifetime resources the teardown removes.
    ///
    /// Inserts the test [`WeaponRegistry`] too (GTW-257): like `CombatTuning` it is
    /// PERSISTENT `Load` state present before a battle, and `setup_battle_on_request`
    /// reads it to arm each ganger. The fixture gangers reference [`TEST_WEAPON_KEY`],
    /// which it holds, so a setup succeeds.
    fn headless_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(BattleSimPlugin);
        app.insert_resource(CombatTuning::default());
        app.insert_resource(weapon_registry());
        app
    }

    /// Drain the `BattleReady` buffer and return how many were emitted this run — the
    /// setup-complete signal probe. `drain` empties the buffer, so a follow-up call
    /// sees only messages written since (the test runs one `update()` then probes).
    fn drain_battle_ready(app: &mut App) -> usize {
        app.world_mut()
            .resource_mut::<Messages<BattleReady>>()
            .drain()
            .count()
    }

    /// Drain the `BattleWon` buffer and return how many were emitted since the last drain
    /// — the win-census probe (mirrors `drain_battle_ready`).
    fn drain_battle_won(app: &mut App) -> usize {
        app.world_mut()
            .resource_mut::<Messages<BattleWon>>()
            .drain()
            .count()
    }

    /// Drain the `BattleLost` buffer and return how many were emitted since the last drain
    /// — the loss-census probe (mirrors `drain_battle_ready`).
    fn drain_battle_lost(app: &mut App) -> usize {
        app.world_mut()
            .resource_mut::<Messages<BattleLost>>()
            .drain()
            .count()
    }

    /// A three-ganger fixture: one player ganger (`faction 0`) + two enemy gangers
    /// (`faction 1`) — the AC1/AC2/AC4/AC6(b) win-side fixture (`PlayerFaction(0)` from the
    /// `player_faction` default). Link-free, so `setup_battle` validates trivially.
    fn one_player_two_enemy_situation() -> Situation {
        Situation {
            gangers: vec![
                ganger_at(key(5, 6, 0), 0),
                ganger_at(key(7, 8, 0), 1),
                ganger_at(key(9, 10, 0), 1),
            ],
            ..Situation::new()
        }
    }

    /// A player-only fixture: every ganger is `faction 0` — the AC6(a) degenerate /
    /// empty-enemy-roster fixture (`has_enemy_of` is false, so the census never wins).
    fn player_only_situation() -> Situation {
        Situation {
            gangers: vec![ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 0)],
            ..Situation::new()
        }
    }

    /// Set the [`LifeState`] of every ganger whose [`Faction`] is `faction` to `to`, via a
    /// `world_mut()` query in the test body (bevy-traps #7's headless-test carve-out — NOT
    /// a registered system / helper taking `&mut World`). The accepted way to drive a
    /// ganger out of the fight without re-running the damage pipeline.
    fn set_faction_life_state(app: &mut App, faction: u8, to: LifeState) {
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
    fn set_one_faction_ganger_life_state(app: &mut App, faction: u8, to: LifeState) -> bool {
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

    // === AC1 — BattleSimPlugin bundles the runtime + registers the three lifecycle
    // messages; bundled-plugin buffers are present; no double-add across update(). ===

    #[test]
    fn plugin_bundles_runtime_and_registers_lifecycle_messages() {
        let mut app = headless_app();
        // One update proves no double-add panic across the bundled add_message calls.
        app.update();

        let world = app.world();
        // The three lifecycle buffers BattleSimPlugin registers.
        assert!(
            world
                .get_resource::<Messages<SetupBattleRequested>>()
                .is_some(),
            "BattleSimPlugin must register the SetupBattleRequested buffer",
        );
        assert!(
            world
                .get_resource::<Messages<TeardownBattleRequested>>()
                .is_some(),
            "BattleSimPlugin must register the TeardownBattleRequested buffer",
        );
        assert!(
            world.get_resource::<Messages<BattleReady>>().is_some(),
            "BattleSimPlugin must register the BattleReady buffer",
        );
        // The bundled OccupancyMaintenancePlugin's buffer (proves it is bundled).
        assert!(
            world.get_resource::<Messages<CoverDestroyed>>().is_some(),
            "BattleSimPlugin must bundle OccupancyMaintenancePlugin (CoverDestroyed buffer)",
        );
        // A representative SimActsPlugin buffer (proves it is bundled).
        assert!(
            world.get_resource::<Messages<FireRequested>>().is_some(),
            "BattleSimPlugin must bundle SimActsPlugin (FireRequested buffer)",
        );
    }

    // === AC2 — SetupBattleRequested seeds SimRng (from the message's seed) + runs
    // setup_battle, emitting BattleReady on success; the seed threads through. ===

    #[test]
    fn setup_request_seeds_rng_inserts_resources_and_signals_ready() {
        let mut app = headless_app();
        app.world_mut().write_message(SetupBattleRequested::new(
            two_ganger_situation(),
            BattleSeed::new(SEED),
        ));
        app.update();

        // The battle-lifetime RNG was inserted.
        assert!(
            app.world().get_resource::<SimRng>().is_some(),
            "the setup must insert a SimRng",
        );
        // The four setup_battle resources are present.
        assert!(
            app.world().get_resource::<CoverLedger>().is_some(),
            "the setup must insert a CoverLedger",
        );
        assert!(
            app.world().get_resource::<SurfaceGrid>().is_some(),
            "the setup must insert a SurfaceGrid",
        );
        assert!(
            app.world().get_resource::<OccupancyGrid>().is_some(),
            "the setup must insert an OccupancyGrid",
        );
        assert!(
            app.world().get_resource::<VerticalLinkGraph>().is_some(),
            "the setup must insert a VerticalLinkGraph",
        );
        // GTW-226 AC2 — PlayerFaction is inserted on the SAME Ok path as
        // BattleInProgress (both present together) and, because the fixture omits
        // player_faction, defaults to gang 0.
        assert!(
            app.world().get_resource::<BattleInProgress>().is_some(),
            "the setup must insert BattleInProgress on the Ok path",
        );
        let player = app.world().get_resource::<PlayerFaction>().copied();
        assert_eq!(
            player,
            Some(PlayerFaction::new(Faction::new(0))),
            "the setup must insert PlayerFaction seeded from the situation (default gang 0)",
        );
        // A BattleReady was emitted on success.
        assert_eq!(
            drain_battle_ready(&mut app),
            1,
            "a successful setup must emit exactly one BattleReady",
        );
    }

    // === GTW-226 AC1 — PlayerFaction is a public newtype Resource over Faction with
    // new() + a derived Deref reading the inner Faction back. ===

    #[test]
    fn player_faction_constructs_and_derefs_to_its_inner_faction() {
        let player = PlayerFaction::new(Faction::new(2));
        assert_eq!(
            *player,
            Faction::new(2),
            "PlayerFaction::new(Faction(2)) must Deref back to Faction(2)",
        );
    }

    // === GTW-226 AC3 — an authored player_faction:1 overrides the Faction(0) default,
    // proving the seed reads situation.player_faction, not a hardcoded 0. ===

    #[test]
    fn authored_player_faction_overrides_the_default_seed() {
        let mut app = headless_app();
        app.world_mut().write_message(SetupBattleRequested::new(
            two_ganger_situation_player_faction_one(),
            BattleSeed::new(SEED),
        ));
        app.update();

        let player = app.world().get_resource::<PlayerFaction>().copied();
        assert_eq!(
            player,
            Some(PlayerFaction::new(Faction::new(1))),
            "an authored player_faction:1 must seed PlayerFaction(Faction(1)), not the default 0",
        );
    }

    #[test]
    fn setup_threads_the_message_seed_through_sim_rng() {
        // Two runs with the SAME seed give a SimRng whose first draw matches; a
        // DIFFERENT seed gives a different first draw — the message's seed threads
        // through SimRng::from_seed (a relation, never a pinned magnitude).
        let first_draw = |seed: u64| {
            let mut app = headless_app();
            app.world_mut().write_message(SetupBattleRequested::new(
                two_ganger_situation(),
                BattleSeed::new(seed),
            ));
            app.update();
            app.world_mut()
                .get_resource_mut::<SimRng>()
                .map(|mut rng| rng.next_u64())
        };
        assert_eq!(
            first_draw(SEED),
            first_draw(SEED),
            "the same seed must thread through to an identical SimRng first draw",
        );
        assert_ne!(
            first_draw(SEED),
            first_draw(SEED ^ 0xFFFF),
            "a different seed must yield a different SimRng first draw",
        );
    }

    // === AC3 — setup_battle runs on the REAL Commands path: the spawned WornArmor
    // count equals the authored ganger count, and the four resources are present. ===

    #[test]
    fn setup_runs_setup_battle_on_the_real_commands_path() {
        let situation = two_ganger_situation();
        let authored = situation.gangers.len();
        let mut app = headless_app();
        app.world_mut()
            .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
        app.update();

        let world = app.world_mut();
        let mut query = world.query::<&WornArmor>();
        assert_eq!(
            query.iter(world).count(),
            authored,
            "the spawned WornArmor ganger count must equal the authored ganger count (the real \
             setup_battle ran, not a stub)",
        );
    }

    // === AC5 (sim half) — a FAILED setup emits NO BattleReady, inserts no setup
    // resource, and does not panic. ===

    #[test]
    fn failed_setup_emits_no_ready_and_inserts_no_resource() {
        let (situation, _link) = dangling_link_situation();
        let mut app = headless_app();
        app.world_mut()
            .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
        app.update();

        // No BattleReady signalled (fail-closed).
        assert_eq!(
            drain_battle_ready(&mut app),
            0,
            "a failed setup must emit NO BattleReady",
        );
        // setup_battle aborts before any resource insert (validation first).
        assert!(
            app.world().get_resource::<CoverLedger>().is_none(),
            "a failed setup must insert no CoverLedger (it aborts before any resource insert)",
        );
        assert!(
            app.world().get_resource::<OccupancyGrid>().is_none(),
            "a failed setup must insert no OccupancyGrid",
        );
        assert!(
            app.world().get_resource::<VerticalLinkGraph>().is_none(),
            "a failed setup must insert no VerticalLinkGraph",
        );
        // The SimRng IS inserted before the setup call (seeding precedes setup), but no
        // grid resources land — the app's gate keys off BattleReady, which is absent.
    }

    /// Sanity: the dangling-link fixture really yields the typed `DanglingCell` error
    /// (so the fail-closed test above is exercising the real error path).
    #[test]
    fn dangling_link_fixture_yields_the_typed_error() {
        use bevy::ecs::system::RunSystemOnce as _;

        let (situation, link) = dangling_link_situation();
        let registry = weapon_registry();
        let mut world = World::new();
        let result = world.run_system_once(move |mut commands: Commands| {
            setup_battle(&situation, &registry, &mut commands)
        });
        assert!(result.is_ok(), "the one-shot system must run");
        let Ok(setup_result) = result else {
            return;
        };
        assert_eq!(
            setup_result.err(),
            Some(BattleSetupError::InvalidLink(
                InvalidVerticalLink::DanglingCell { link },
            )),
            "the dangling-link fixture must abort setup with the typed DanglingCell error",
        );
    }

    // === AC6 (sim half) — TeardownBattleRequested after a setup removes the five
    // battle-lifetime resources; an injected CombatTuning is untouched. ===

    #[test]
    fn teardown_removes_battle_resources_and_leaves_tuning() {
        // `headless_app` already inserts CombatTuning (E10.4's persistent Load resource).
        let mut app = headless_app();

        // Set the battle up first.
        app.world_mut().write_message(SetupBattleRequested::new(
            two_ganger_situation(),
            BattleSeed::new(SEED),
        ));
        app.update();
        assert!(
            app.world().get_resource::<SimRng>().is_some(),
            "precondition: setup inserted the SimRng",
        );
        assert!(
            app.world().get_resource::<OccupancyGrid>().is_some(),
            "precondition: setup inserted the OccupancyGrid",
        );
        // GTW-226 AC5 precondition — setup inserted PlayerFaction (present, like
        // BattleInProgress) so the teardown removal is observable.
        assert!(
            app.world().get_resource::<PlayerFaction>().is_some(),
            "precondition: setup inserted the PlayerFaction",
        );

        // Now tear it down.
        app.world_mut().write_message(TeardownBattleRequested);
        app.update();

        assert!(
            app.world().get_resource::<SimRng>().is_none(),
            "teardown must remove the SimRng",
        );
        assert!(
            app.world().get_resource::<CoverLedger>().is_none(),
            "teardown must remove the CoverLedger",
        );
        assert!(
            app.world().get_resource::<SurfaceGrid>().is_none(),
            "teardown must remove the SurfaceGrid",
        );
        assert!(
            app.world().get_resource::<OccupancyGrid>().is_none(),
            "teardown must remove the OccupancyGrid",
        );
        assert!(
            app.world().get_resource::<VerticalLinkGraph>().is_none(),
            "teardown must remove the VerticalLinkGraph",
        );
        // GTW-226 AC5 — teardown removes PlayerFaction; and BattleInProgress is also
        // absent in the same test, proving the identical battle-active lifetime.
        assert!(
            app.world().get_resource::<PlayerFaction>().is_none(),
            "teardown must remove the PlayerFaction",
        );
        assert!(
            app.world().get_resource::<BattleInProgress>().is_none(),
            "teardown must remove the BattleInProgress witness (identical lifetime to \
             PlayerFaction)",
        );
        // The persistent Load resource is untouched.
        assert!(
            app.world().get_resource::<CombatTuning>().is_some(),
            "teardown must NOT remove CombatTuning (E10.4's persistent Load resource)",
        );
    }

    // === GTW-212 AC1 — BattleInProgress is a public marker Resource with the correct
    // lifecycle: ABSENT before setup, PRESENT after a successful setup, ABSENT after
    // teardown, and ABSENT after a FAILED setup (no BattleReady → no witness). ===

    #[test]
    fn battle_in_progress_tracks_the_battle_active_window() {
        let mut app = headless_app();

        // ABSENT before any setup.
        assert!(
            app.world().get_resource::<BattleInProgress>().is_none(),
            "BattleInProgress must be absent before any setup",
        );

        // PRESENT after a successful setup (the small fixture that emits BattleReady).
        app.world_mut().write_message(SetupBattleRequested::new(
            two_ganger_situation(),
            BattleSeed::new(SEED),
        ));
        app.update();
        assert_eq!(
            drain_battle_ready(&mut app),
            1,
            "the fixture setup must succeed (one BattleReady)",
        );
        assert!(
            app.world().get_resource::<BattleInProgress>().is_some(),
            "a successful setup must insert the BattleInProgress witness",
        );

        // ABSENT after a teardown.
        app.world_mut().write_message(TeardownBattleRequested);
        app.update();
        assert!(
            app.world().get_resource::<BattleInProgress>().is_none(),
            "teardown must remove the BattleInProgress witness",
        );
    }

    #[test]
    fn failed_setup_inserts_no_battle_in_progress() {
        let (situation, _link) = dangling_link_situation();
        let mut app = headless_app();
        app.world_mut()
            .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
        app.update();

        // A failed setup emits no BattleReady (fail-closed) and inserts no witness.
        assert_eq!(
            drain_battle_ready(&mut app),
            0,
            "a failed setup must emit NO BattleReady",
        );
        assert!(
            app.world().get_resource::<BattleInProgress>().is_none(),
            "a FAILED setup must NOT insert BattleInProgress (the Ok-only witness)",
        );
        // GTW-226 AC4 — fail-closed: PlayerFaction rides the same Ok-only path, so a
        // failed setup inserts no PlayerFaction (mirrors the BattleInProgress absence).
        assert!(
            app.world().get_resource::<PlayerFaction>().is_none(),
            "a FAILED setup must NOT insert PlayerFaction (the Ok-only seed)",
        );
    }

    // === GTW-212 AC2 — the Simulate band is gated on BattleInProgress, not
    // OccupancyGrid: the bundled runtime does NOT panic pre-battle / post-teardown (a
    // missing non-Option Res would panic), runs only while the witness is present, and
    // the gate's run_if references BattleInProgress (not OccupancyGrid). ===

    #[test]
    fn bundled_runtime_is_inert_pre_battle_and_post_teardown() {
        let mut app = headless_app();

        // Pre-battle: no witness, so the gated band is skipped — updating must not panic
        // even though the bundled in-set systems take non-Option battle-lifetime Res.
        for _ in 0..3 {
            app.update();
        }
        assert!(
            app.world().get_resource::<BattleInProgress>().is_none(),
            "no setup means no BattleInProgress, so the Simulate band is skipped",
        );

        // Set the battle up — the witness now exists, so the band is live (and an
        // emitted FireRequested is consumed without panic).
        app.world_mut().write_message(SetupBattleRequested::new(
            two_ganger_situation(),
            BattleSeed::new(SEED),
        ));
        app.update();
        assert!(
            app.world().get_resource::<BattleInProgress>().is_some(),
            "a successful setup turns the gate on",
        );
        // An arbitrary actor entity + a valid single-shot mode spec — the gated
        // dispatch must consume the message without panic (the unarmed actor simply
        // fails the fire guard; the point is the LIVE band runs and stays panic-free).
        let actor = app.world_mut().spawn_empty().id();
        let mode = FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.2),
            ModeShots::new(1),
        );
        app.world_mut().write_message(FireRequested::new(
            actor,
            mode,
            Cell::new(7, 8),
            Level::new(0),
        ));
        app.update();

        // Tear it down — the witness is gone, the band is skipped again, and further
        // updates do not panic.
        app.world_mut().write_message(TeardownBattleRequested);
        app.update();
        assert!(
            app.world().get_resource::<BattleInProgress>().is_none(),
            "teardown removes the witness, re-closing the gate",
        );
        for _ in 0..3 {
            app.update();
        }
    }

    /// AC2 structure check — the gate's `run_if` references the `BattleInProgress`
    /// witness and NOT `OccupancyGrid`. A source-scan over THIS module's text proves the
    /// witness swap landed in the `configure_sets` call (the externally invisible wiring
    /// the AC2 grep check asks for), keyed on the literal `run_if(resource_exists::<…>)`
    /// fragments so a regression back to the `OccupancyGrid` proxy turns this red.
    #[test]
    fn simulate_gate_run_if_references_battle_in_progress_not_occupancy_grid() {
        let source = include_str!("battle.rs");
        // The witness fragment is assembled so this assertion is not its own self-match.
        let witness = concat!("BattleIn", "Progress");
        let stale = concat!("Occupancy", "Grid");
        let gate_on = |ty: &str| format!("run_if(resource_exists::<{ty}>)");
        assert!(
            source.contains(&gate_on(witness)),
            "the Simulate band's run_if must gate on the BattleInProgress witness",
        );
        assert!(
            !source.contains(&gate_on(stale)),
            "the Simulate band must NOT gate on the incidental OccupancyGrid proxy any more",
        );
    }

    // === A no-trigger run is inert: with no lifecycle message emitted, neither
    // system mutates the world (the empty-input invariant). ===

    #[test]
    fn no_lifecycle_message_means_no_setup_or_teardown() {
        let mut app = headless_app();
        for _ in 0..3 {
            app.update();
        }
        assert!(
            app.world().get_resource::<SimRng>().is_none(),
            "no SetupBattleRequested means no SimRng inserted",
        );
        assert!(
            app.world().get_resource::<OccupancyGrid>().is_none(),
            "no SetupBattleRequested means no OccupancyGrid inserted",
        );
        assert_eq!(
            drain_battle_ready(&mut app),
            0,
            "no SetupBattleRequested means no BattleReady signalled",
        );
    }

    /// The lifecycle messages are constructible and carry their payload (the no-bare
    /// payload contract): `SetupBattleRequested` owns a `Situation` + `BattleSeed`
    /// with no lifetime parameter (that this names `SetupBattleRequested` without
    /// `<'_>` is the proof).
    #[test]
    fn lifecycle_messages_carry_their_payload() {
        let seed = BattleSeed::new(SEED);
        let setup: SetupBattleRequested = SetupBattleRequested::new(two_ganger_situation(), seed);
        assert_eq!(setup.seed, seed, "SetupBattleRequested carries the seed");
        assert_eq!(
            setup.situation.gangers.len(),
            2,
            "SetupBattleRequested carries the owned Situation",
        );
        // The two signals are unit payloads (their identity IS the message — nothing to
        // carry); constructing them and comparing against a fresh value proves they are
        // the zero-field markers the app sends / reads.
        let teardown = TeardownBattleRequested;
        assert_eq!(
            Some(teardown),
            Some(TeardownBattleRequested),
            "TeardownBattleRequested is a unit signal",
        );
        let ready = BattleReady;
        assert_eq!(
            Some(ready),
            Some(BattleReady),
            "BattleReady is a unit signal",
        );
    }

    // === GTW-237 — the roster-grounded WIN/LOSS outcome census (check_outcome). The
    // fixtures go through the REAL setup (SetupBattleRequested seeds PlayerFaction +
    // BattleRoster) and drive LifeState via a world_mut() query (bevy-traps #7 carve-out).
    // ===

    /// AC1 — all enemies down + a live player → exactly ONE `BattleWon`, zero `BattleLost`.
    #[test]
    fn all_enemies_down_with_live_player_emits_one_battle_won_and_no_loss() {
        let mut app = headless_app();
        app.world_mut().write_message(SetupBattleRequested::new(
            one_player_two_enemy_situation(),
            BattleSeed::new(SEED),
        ));
        app.update();
        // Drain the setup-frame census output (no outcome yet — all gangers Alive).
        let _ = drain_battle_won(&mut app);
        let _ = drain_battle_lost(&mut app);

        // One enemy Dead, the other Downed (both count OUT); the faction-0 player stays
        // Alive. The setter only flips Alive gangers, so the two calls down DISTINCT enemies.
        assert!(
            set_one_faction_ganger_life_state(&mut app, 1, LifeState::Dead),
            "precondition: a first enemy ganger to set Dead",
        );
        assert!(
            set_one_faction_ganger_life_state(&mut app, 1, LifeState::Downed),
            "precondition: a second (still-Alive) enemy ganger to set Downed",
        );
        app.update();

        assert_eq!(
            drain_battle_won(&mut app),
            1,
            "all enemies out (Dead + Downed) with a live player must emit exactly one BattleWon",
        );
        assert_eq!(
            drain_battle_lost(&mut app),
            0,
            "a player still standing must emit NO BattleLost",
        );
    }

    /// AC2 — any enemy still `Alive` → NO `BattleWon`.
    #[test]
    fn any_enemy_alive_emits_no_battle_won() {
        let mut app = headless_app();
        app.world_mut().write_message(SetupBattleRequested::new(
            one_player_two_enemy_situation(),
            BattleSeed::new(SEED),
        ));
        app.update();
        let _ = drain_battle_won(&mut app);

        // Only ONE enemy Dead; the other faction-1 ganger stays Alive.
        assert!(
            set_one_faction_ganger_life_state(&mut app, 1, LifeState::Dead),
            "precondition: the fixture fielded ≥1 enemy ganger to set Dead",
        );
        app.update();

        assert_eq!(
            drain_battle_won(&mut app),
            0,
            "with an enemy still Alive the census must NOT win",
        );
    }

    /// AC3 — all players down → exactly ONE `BattleLost`, enemies still Alive → no
    /// `BattleWon`.
    #[test]
    fn all_players_down_emits_one_battle_lost_and_no_win() {
        let mut app = headless_app();
        app.world_mut().write_message(SetupBattleRequested::new(
            two_ganger_situation(),
            BattleSeed::new(SEED),
        ));
        app.update();
        let _ = drain_battle_won(&mut app);
        let _ = drain_battle_lost(&mut app);

        // The single player ganger (faction 0) goes Dead; the faction-1 enemy stays Alive.
        set_faction_life_state(&mut app, 0, LifeState::Dead);
        app.update();

        assert_eq!(
            drain_battle_lost(&mut app),
            1,
            "all players out must emit exactly one BattleLost",
        );
        assert_eq!(
            drain_battle_won(&mut app),
            0,
            "an enemy still Alive (and the player wiped) must emit NO BattleWon",
        );
    }

    /// AC4 — at-most-once per outcome (no per-frame spam): the win latch and the loss
    /// latch each suppress every emit after the first.
    #[test]
    fn outcomes_emit_at_most_once_per_battle() {
        // Win latch: the AC1 all-enemies-down fixture.
        let mut win_app = headless_app();
        win_app.world_mut().write_message(SetupBattleRequested::new(
            one_player_two_enemy_situation(),
            BattleSeed::new(SEED),
        ));
        win_app.update();
        let _ = drain_battle_won(&mut win_app);

        set_faction_life_state(&mut win_app, 1, LifeState::Dead);
        win_app.update();
        assert_eq!(
            drain_battle_won(&mut win_app),
            1,
            "the first all-enemies-down frame emits exactly one BattleWon",
        );
        // Several more frames WITHOUT draining: the latch suppresses every further emit.
        for _ in 0..3 {
            win_app.update();
        }
        assert_eq!(
            drain_battle_won(&mut win_app),
            0,
            "the win latch must suppress all further BattleWon (no per-frame spam)",
        );

        // Loss latch: the AC3 all-players-down fixture.
        let mut loss_app = headless_app();
        loss_app
            .world_mut()
            .write_message(SetupBattleRequested::new(
                two_ganger_situation(),
                BattleSeed::new(SEED),
            ));
        loss_app.update();
        let _ = drain_battle_lost(&mut loss_app);

        set_faction_life_state(&mut loss_app, 0, LifeState::Dead);
        loss_app.update();
        assert_eq!(
            drain_battle_lost(&mut loss_app),
            1,
            "the first all-players-down frame emits exactly one BattleLost",
        );
        for _ in 0..3 {
            loss_app.update();
        }
        assert_eq!(
            drain_battle_lost(&mut loss_app),
            0,
            "the loss latch must suppress all further BattleLost (no per-frame spam)",
        );
    }

    /// AC5 — inert outside a live battle: no setup means no `BattleInProgress` /
    /// `PlayerFaction` / `BattleRoster`, so the Simulate-gated census never runs and both
    /// buffers drain to 0.
    #[test]
    fn census_is_inert_without_a_live_battle() {
        let mut app = headless_app();
        // No SetupBattleRequested — the battle-lifetime resources are all absent.
        assert!(
            app.world().get_resource::<BattleInProgress>().is_none(),
            "precondition: no setup means no BattleInProgress",
        );
        assert!(
            app.world().get_resource::<BattleRoster>().is_none(),
            "precondition: no setup means no BattleRoster",
        );
        assert!(
            app.world().get_resource::<PlayerFaction>().is_none(),
            "precondition: no setup means no PlayerFaction",
        );

        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            drain_battle_won(&mut app),
            0,
            "the Simulate gate keeps check_outcome from running, so NO BattleWon",
        );
        assert_eq!(
            drain_battle_lost(&mut app),
            0,
            "the Simulate gate keeps check_outcome from running, so NO BattleLost",
        );
    }

    /// AC6(a) — roster-grounded, NOT scan-grounded: a player-only situation (empty enemy
    /// roster) never wins, even though every fielded ganger could be "all non-player down"
    /// under the old scan (there are no enemies to be down).
    #[test]
    fn empty_enemy_roster_never_wins() {
        let mut app = headless_app();
        app.world_mut().write_message(SetupBattleRequested::new(
            player_only_situation(),
            BattleSeed::new(SEED),
        ));
        app.update();

        // has_enemy_of(player) is false for a player-only roster — no false win.
        assert_eq!(
            drain_battle_won(&mut app),
            0,
            "a player-only (empty enemy) roster must never emit BattleWon (has_enemy_of false)",
        );
        // And no loss either: the player gang is fully Alive.
        assert_eq!(
            drain_battle_lost(&mut app),
            0,
            "a fully-Alive player gang must not emit BattleLost",
        );
    }

    /// AC6(b) — the converse: with enemies fielded and all of them down (corpses persist as
    /// entities, the sim has no despawn-on-death), the win STILL fires — existence comes
    /// from the roster, so the win is robust regardless of despawn. (Shares the AC1 setup;
    /// asserts the roster, not the scan, is what grounds the win.)
    #[test]
    fn wiped_out_enemy_gang_still_wins_from_the_roster() {
        let mut app = headless_app();
        app.world_mut().write_message(SetupBattleRequested::new(
            one_player_two_enemy_situation(),
            BattleSeed::new(SEED),
        ));
        app.update();
        let _ = drain_battle_won(&mut app);

        // The enemy roster IS fielded (the win cannot come from an empty roster) ...
        let roster = app.world().get_resource::<BattleRoster>().cloned();
        assert_eq!(
            roster,
            Some(BattleRoster::new([Faction::new(0), Faction::new(1)])),
            "the roster records both fielded factions",
        );
        // ... and the enemy corpses persist as entities after they go down.
        set_faction_life_state(&mut app, 1, LifeState::Dead);
        let world = app.world_mut();
        let mut enemy_corpses = world.query::<(&Faction, &LifeState)>();
        let dead_enemies = enemy_corpses
            .iter(world)
            .filter(|&(&fac, &life)| fac == Faction::new(1) && life == LifeState::Dead)
            .count();
        assert_eq!(
            dead_enemies, 2,
            "both enemy gangers persist as Dead entities (no despawn)"
        );

        app.update();
        assert_eq!(
            drain_battle_won(&mut app),
            1,
            "with enemies fielded and all down, the roster-grounded win still fires",
        );
    }

    /// AC7 — `BattleRoster` lifetime tracks `BattleInProgress`: present after a successful
    /// setup (its set == the situation's distinct factions), absent after teardown
    /// (alongside `BattleInProgress` / `PlayerFaction`).
    #[test]
    fn battle_roster_lifetime_tracks_battle_in_progress() {
        let mut app = headless_app();

        // ABSENT before any setup.
        assert!(
            app.world().get_resource::<BattleRoster>().is_none(),
            "BattleRoster must be absent before any setup",
        );

        // PRESENT after a successful setup — the set equals the distinct ganger factions.
        app.world_mut().write_message(SetupBattleRequested::new(
            one_player_two_enemy_situation(),
            BattleSeed::new(SEED),
        ));
        app.update();
        let roster = app.world().get_resource::<BattleRoster>().cloned();
        assert_eq!(
            roster,
            Some(BattleRoster::new([Faction::new(0), Faction::new(1)])),
            "setup must capture a BattleRoster of the situation's distinct ganger factions",
        );

        // ABSENT after a teardown — alongside BattleInProgress + PlayerFaction.
        app.world_mut().write_message(TeardownBattleRequested);
        app.update();
        assert!(
            app.world().get_resource::<BattleRoster>().is_none(),
            "teardown must remove the BattleRoster (lifetime identical to BattleInProgress)",
        );
        assert!(
            app.world().get_resource::<BattleInProgress>().is_none(),
            "teardown must remove BattleInProgress (the shared-lifetime witness)",
        );
        assert!(
            app.world().get_resource::<PlayerFaction>().is_none(),
            "teardown must remove PlayerFaction (the shared-lifetime resource)",
        );
    }

    /// AC8 — mutual wipe → `BattleLost`, not `BattleWon` (the flagged mutual-exclusion
    /// rule): enemies AND players all set OUT in the same update emit one loss, zero wins.
    #[test]
    fn mutual_wipe_resolves_to_battle_lost_not_won() {
        let mut app = headless_app();
        app.world_mut().write_message(SetupBattleRequested::new(
            one_player_two_enemy_situation(),
            BattleSeed::new(SEED),
        ));
        app.update();
        let _ = drain_battle_won(&mut app);
        let _ = drain_battle_lost(&mut app);

        // Everyone falls in the SAME frame: the player gang Dead, the enemy gang Downed.
        set_faction_life_state(&mut app, 0, LifeState::Dead);
        set_faction_life_state(&mut app, 1, LifeState::Downed);
        app.update();

        assert_eq!(
            drain_battle_lost(&mut app),
            1,
            "a mutual wipe must emit exactly one BattleLost",
        );
        assert_eq!(
            drain_battle_won(&mut app),
            0,
            "a mutual wipe must emit NO BattleWon (the win requires a surviving player)",
        );
    }
}
