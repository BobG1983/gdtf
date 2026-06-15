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

use bevy::prelude::{
    App, Commands, IntoScheduleConfigs, Message, MessageReader, MessageWriter, Plugin, Resource,
    Update, error, resource_exists,
};

use crate::{
    acts::SimActsPlugin,
    cover::CoverLedger,
    occupancy::OccupancyGrid,
    occupancy_sync::{OccupancyMaintenancePlugin, SimSystems},
    rng::{BattleSeed, SimRng},
    situation::{Situation, setup_battle},
    surface::SurfaceGrid,
    vertical::VerticalLinkGraph,
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

/// **Setup** the battle on [`SetupBattleRequested`] — seed the [`SimRng`] and run
/// [`setup_battle`], signalling [`BattleReady`] on success (E10.5).
///
/// Drains [`MessageReader<SetupBattleRequested>`] and per message:
///
/// 1. Inserts the battle-lifetime [`SimRng`] seeded from the message's [`BattleSeed`]
///    (via [`SimRng::from_seed`] — the deterministic stream the acts draw from).
/// 2. Calls [`setup_battle`] on the REAL [`Commands`] path. On `Ok` the four sim
///    resources ([`CoverLedger`] / [`SurfaceGrid`] / [`OccupancyGrid`] /
///    [`VerticalLinkGraph`]) and the spawned ganger entities land in the world, the
///    [`BattleInProgress`] witness is inserted (the battle-active tag the
///    [`SimSystems::Simulate`] band gates on), and a [`BattleReady`] is written; on
///    `Err` the typed [`InvalidVerticalLink`](crate::vertical::InvalidVerticalLink) is
///    surfaced via [`error!`] and NEITHER [`BattleInProgress`] NOR [`BattleReady`] is
///    written — the app never advances on a bad battle, and the gate never opens. NO
///    `unwrap`/`expect`/`panic`.
///
/// [`CombatTuning`](crate::tuning::CombatTuning) is NOT inserted here: it is E10.4's
/// PERSISTENT `Load` resource, present throughout the battle for the acts to read.
pub fn setup_battle_on_request(
    mut requests: MessageReader<SetupBattleRequested>,
    mut ready: MessageWriter<BattleReady>,
    mut commands: Commands,
) {
    for request in requests.read() {
        // 1. Seed the battle-lifetime RNG from the trigger's seed.
        commands.insert_resource(SimRng::from_seed(request.seed));

        // 2. Pour the situation into the world via the authoritative setup. A bad
        //    vertical link returns the typed error — log it (NEVER panic / unwrap) and
        //    write NO BattleReady, so the app's gate never fires (fail-closed).
        match setup_battle(&request.situation, &mut commands) {
            Ok(_setup) => {
                // The battle is live: insert the gate witness (alongside the four
                // setup_battle grids + the seeded SimRng) so the Simulate band's bundled
                // runtime turns on, then signal BattleReady. Both happen ONLY on Ok.
                commands.insert_resource(BattleInProgress);
                ready.write(BattleReady);
            }
            Err(invalid) => {
                error!(
                    "battle setup failed on an invalid vertical link: {invalid:?}; no BattleReady \
                     will be signalled"
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
/// [`SurfaceGrid`] / [`OccupancyGrid`] / [`VerticalLinkGraph`]), and the
/// [`BattleInProgress`] witness (closing the [`SimSystems::Simulate`] gate so the
/// bundled runtime goes inert again). These resources are BATTLE-lifetime: the app
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
///   ([`SetupBattleRequested`] / [`TeardownBattleRequested`] / [`BattleReady`])
///   exactly once each (`bevy-traps.md` #5 — an unregistered buffer fails a
///   [`MessageReader`]'s param validation); and
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
            // Gate the whole Simulate band (the bundled dispatch + occupancy systems)
            // on the setup-inserted BattleInProgress witness — the purpose-built
            // "a battle is active" tag (GTW-212), replacing the incidental OccupancyGrid
            // proxy — so the bundled runtime is inert + panic-free outside a live battle
            // (bevy-traps.md #1; conditions still accumulate across configure_sets #5).
            .configure_sets(
                Update,
                SimSystems::Simulate.run_if(resource_exists::<BattleInProgress>),
            )
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
        situation::GangerSpawn,
        tuning::CombatTuning,
        vertical::{InvalidVerticalLink, LinkKind, VerticalLink},
        weapon::{FireModeSpec, ModeConeMult, ModeShots, ModeTuPercent},
    };

    /// An arbitrary (NOT shipped tuning) seed for a test battle's RNG stream.
    const SEED: u64 = 0x5A1C_AC75;

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
        }
    }

    /// A valid two-ganger fixture situation (no cover / slabs / links — link-free
    /// validates trivially).
    fn two_ganger_situation() -> Situation {
        Situation {
            gangers: vec![ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)],
            ..Situation::new()
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
    fn headless_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(BattleSimPlugin);
        app.insert_resource(CombatTuning::default());
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
        // A BattleReady was emitted on success.
        assert_eq!(
            drain_battle_ready(&mut app),
            1,
            "a successful setup must emit exactly one BattleReady",
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
        let mut world = World::new();
        let result = world
            .run_system_once(move |mut commands: Commands| setup_battle(&situation, &mut commands));
        assert!(result.is_ok(), "the one-shot system must run");
        let Ok(setup_result) = result else {
            return;
        };
        assert_eq!(
            setup_result.err(),
            Some(InvalidVerticalLink::DanglingCell { link }),
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
}
