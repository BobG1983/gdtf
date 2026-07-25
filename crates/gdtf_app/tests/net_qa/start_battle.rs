//! GTW-742 — the T9 start-battle navigation path, driven through the REAL
//! `drive_start_battle` consumer + the menu's `apply_start_battle` on a headless app
//! resting at the main menu (never a shadow copy).
//!
//! A `StartBattle` request enters through the same inbox the loopback listener feeds;
//! the app then descends the real state machine (`Menu → Game → … → BattleRunning`)
//! exactly as a Battlescape-button press would, because the network path produces the
//! SAME `StartBattleRequested` message the button produces. Two facts are pinned:
//!
//! - a `StartBattle` sent from the menu with a pinned seed reaches
//!   `BattleScapeState::BattleRunning`, and the REQUESTED seed drove the battle — the
//!   resolved `ShotRng` stream matches the one `ShotRng::from_root(that seed)` produces
//!   exactly (the project's seed-determinism convention, as in the sim's `seed_logging`
//!   suite), proving it was the network seed, not `resolve_root_seed`'s wall-clock;
//! - an unknown situation ref is answered with a typed `QaError::BadRequest` — never a
//!   panic, never a silent no-op — and leaves the app at the menu.

use std::sync::mpsc;

use bevy::{
    app::App,
    state::state::{NextState, State},
};
use gdtf_app::test_support::{
    AppState, BattleScapeState, LoadedSituation, NetQaPlugin, RunningState, SHIPPED_SITUATION,
};
use gdtf_battle_sim::{
    effects::fields::FieldDefRegistry,
    rng::{BattleSeed, ShotRng},
    test_support::{
        fixtures, test_armor_registry, test_gang_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::{CombatTuning, GangerStatTuning},
};
use gdtf_net_qa_transport::{IncomingRequest, Responder};
use gdtf_qa_protocol::{
    envelope::{QaError, QaRequest, QaResponse},
    ids::{SeedNet, SituationRef},
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// The per-milestone frame budget for the deep menu → battle descent (mirrors the
/// `BattleAppBuilder` drive budget, with headroom for the extra `Menu → Game` hop the
/// button path folds into an auto-advance).
const DRIVE_BUDGET: u32 = 128;

/// The fixed seed the determinism test requests over the wire — a distinctive value
/// `resolve_root_seed`'s wall-clock would never coincidentally produce.
const REQUESTED_SEED: u64 = 0x00C0_FFEE_D15E_A5E5;

/// Build a headless app resting at [`RunningState::Menu`] with the REAL `net_qa` router
/// and start-battle consumer wired to an injected inbox, and every persistent `Load`
/// resource the deep descent needs pre-seeded (a `MinimalPlugins` app has no
/// `AssetServer` to resolve them). Returns the app and the sender the test pushes
/// requests on (exactly as the listener thread would). Shared with the affordance-parity
/// suite, which reuses it as its menu-state fixture.
pub(crate) fn menu_app_with_net_qa() -> (App, mpsc::Sender<IncomingRequest>) {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    // The persistent `Load` resources the machine traverses `Load` with, mirroring
    // `BattleAppBuilder` (the canonical menu → battle harness): theme, tunings, and the
    // weapon / armor / gang / field registries the Generation setup pours the situation
    // into, plus the authored `LoadedSituation` itself.
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(test_weapon_registry());
    app.world_mut()
        .insert_resource(test_melee_weapon_registry());
    app.world_mut().insert_resource(test_armor_registry());
    app.world_mut().insert_resource(FieldDefRegistry::default());
    app.world_mut().insert_resource(test_gang_registry());
    app.world_mut()
        .insert_resource(LoadedSituation::new(fixtures::two_ganger()));

    let (tx, rx) = mpsc::channel();
    app.add_plugins(NetQaPlugin::with_channels(rx));

    // Settle into the menu (and run one empty router pass).
    let rested = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        DRIVE_BUDGET,
    );
    assert!(rested, "the harness must rest at RunningState::Menu");
    (app, tx)
}

/// Push a request onto the router's inbox and return the channel its reply arrives on.
fn send(tx: &mpsc::Sender<IncomingRequest>, request: QaRequest) -> mpsc::Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    let sent = tx.send(IncomingRequest::new(request, responder));
    assert!(sent.is_ok(), "the router inbox must be open");
    reply_rx
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// A `StartBattle` sent from the menu naming a shipped situation with a pinned seed
/// descends the real machine to `BattleScapeState::BattleRunning`, and the REQUESTED
/// seed drove the battle — the resolved `ShotRng` stream matches
/// `ShotRng::from_root(that seed)` (the seed-determinism fingerprint), proving the
/// network seed, not wall-clock, seeded the per-subsystem RNG streams end-to-end.
///
/// Pin: drop `drive_start_battle` (or its `StartBattleRequested` write, or the menu's
/// `apply_start_battle` seed install) and the app never reaches `BattleRunning` — or
/// reaches it with a wall-clock seed whose stream does not match `from_root`.
#[test]
fn start_battle_from_menu_reaches_battle_running_with_the_requested_seed() {
    let (mut app, tx) = menu_app_with_net_qa();

    let reply = send(
        &tx,
        QaRequest::StartBattle {
            situation: SituationRef::new(SHIPPED_SITUATION.to_owned()),
            seed:      Some(SeedNet::new(REQUESTED_SEED)),
        },
    );
    // The consumer claims + answers the request THIS frame and writes the shared
    // start-battle message; the menu's `apply_start_battle` then requests `Menu → Game`.
    app.update();
    assert!(
        matches!(reply.try_recv(), Ok(QaResponse::AppFlow(_))),
        "an accepted StartBattle must answer with the app-flow snapshot",
    );

    // Descend the rest of the way to a live battle.
    let reached = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        DRIVE_BUDGET,
    );
    assert!(
        reached,
        "StartBattle from the menu must descend to BattleRunning; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // The requested seed reached the battle: it is the resident `BattleSeed` override.
    assert_eq!(
        app.world().get_resource::<BattleSeed>().copied(),
        Some(BattleSeed::new(REQUESTED_SEED)),
        "the battle's BattleSeed override must be the seed the network requested",
    );

    // …and it drove the streams: the resolved (untouched — no act has fired) `ShotRng`
    // first draw equals a fresh stream derived from the requested seed. Same seed →
    // same stream is the project's seed-determinism fingerprint.
    let world_first = app.world_mut().resource_mut::<ShotRng>().next_u64();
    let mut expected = ShotRng::from_root(BattleSeed::new(REQUESTED_SEED));
    assert_eq!(
        world_first,
        expected.next_u64(),
        "the resolved ShotRng's first draw must equal ShotRng::from_root(requested seed) — \
         proving the network-requested seed drove the streams end-to-end (not wall-clock)",
    );
}

/// A `StartBattle` naming an UNKNOWN situation is rejected with a typed
/// [`QaError::BadRequest`] — not a panic, not a silent no-op — and the app stays at the
/// menu (no descent, no seed override installed).
///
/// Pin: drop the situation-ref validation and the request would either start a battle
/// or leave the client hanging to the deadline sweep instead of a prompt typed error.
#[test]
fn start_battle_with_an_unknown_situation_is_rejected_bad_request() {
    let (mut app, tx) = menu_app_with_net_qa();

    let reply = send(
        &tx,
        QaRequest::StartBattle {
            situation: SituationRef::new("no-such-situation".to_owned()),
            seed:      None,
        },
    );
    app.update();

    assert!(
        matches!(reply.try_recv(), Ok(QaResponse::Error(QaError::BadRequest))),
        "an unknown situation ref must be rejected with a typed BadRequest",
    );
    // No transition was requested and none installed a seed: the app is still at the menu.
    assert_eq!(
        running_state(&app),
        Some(RunningState::Menu),
        "a rejected StartBattle must leave the app at the menu",
    );
    assert!(
        !matches!(
            app.world().resource::<NextState<RunningState>>(),
            NextState::Pending(_) | NextState::PendingIfNeq(_)
        ),
        "a rejected StartBattle must request no RunningState transition",
    );
    assert!(
        app.world().get_resource::<BattleSeed>().is_none(),
        "a rejected StartBattle must install no BattleSeed override",
    );
}
