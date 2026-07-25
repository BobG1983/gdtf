//! BEHAVIOURAL PARITY across a swap, on an app where BOTH stacks are genuinely wired.
//!
//! One identical intent script is driven over the REAL `net_qa` wire into a REAL battle,
//! once with each stack live, and the two runs must leave identical act-bus emissions and
//! identical sim state — swapping which stack draws changes the DRAWING and nothing else.
//!
//! # Why this runs on the `DefaultPlugins` Load app
//!
//! The egui half of the harness only registers where Bevy's render stack is (egui reads
//! `Assets<Shader>`; see the harness plugin's doc), so on a `MinimalPlugins` app the "egui
//! run" would have no egui pass at all and the comparison would degenerate into a
//! determinism re-assert. Here `EguiPlugin` is really added (asserted below), the egui
//! panel is really drawn, and the egui run ends by CLICKING that panel — which only swaps
//! if the egui pass ran and hit-tested for real during the run.
//!
//! # Why the comparison is token-free
//!
//! Wire tokens are `Entity::to_bits`, and the two runs allocate different entities by
//! construction (the `bevy_ui` run has the swap panel on screen, the egui run does not).
//! The sibling `token_free` module therefore scrubs every token to a constant and sorts the
//! token-keyed lists, so the comparison is about CONTENT — gangers, terrain, fog, turn, HUD
//! buttons — and the selection is compared by the selected ganger's NAME, not its token.

use std::{sync::mpsc, time::Duration};

use bevy::{app::App, prelude::*, state::state::State, time::TimeUpdateStrategy};
use bevy_egui::EguiPlugin;
use gdtf_app::test_support::{
    AppState, BattleScapeState, IncomingRequest, NetQaPlugin, Responder, RunningState,
    SHIPPED_SITUATION, UiStackId, UiSwapHarnessPlugin,
};
use gdtf_battle_input::dispatch_act_intents;
use gdtf_battle_sim::acts::{MoveRequested, SetFacingRequested};
use gdtf_qa_protocol::{
    envelope::{QaRequest, QaResponse},
    ids::{CellLevelNet, CellNet, CellXNet, SeedNet, SituationRef},
    intent::{FacingNet, NetIntent, UiStackNet},
    view::BattleView,
};
use gdtf_test_utils::{
    GdtfLoadTestAppBuilder, MessageProbe, advance_until, drain_message_probe, probed,
};

use super::{
    harness::{BUDGET, click_egui_swap_button, inject_egui_pointer_click, live_stack},
    token_free::{selected_name, token_free},
};

/// The seed both runs request over the wire, so the two battles are the same battle.
const PARITY_SEED: u64 = 0x00C0_FFEE_5EED_0816;

/// The fixed virtual-clock step every frame of a run advances by. Pinning the clock makes
/// the playback / AI cadence a function of the FRAME COUNT alone, so two runs that take the
/// same frames see the same time pass — the thing a wall clock would not guarantee.
const FRAME: Duration = Duration::from_millis(16);

/// Frames settled after the battle opens, and between the script's intents — generous
/// enough for a turn / step to animate out and re-open the sim's input gate, and FIXED so
/// both runs spend exactly the same frames there.
const STEP_FRAMES: usize = 40;

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

/// Push a request onto the router's inbox and return the channel its reply arrives on.
fn send(tx: &mpsc::Sender<IncomingRequest>, request: QaRequest) -> mpsc::Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    let sent = tx.send(IncomingRequest::new(request, responder));
    assert!(sent.is_ok(), "the router inbox must be open");
    reply_rx
}

/// Register a `MessageProbe<M>` draining AFTER the classic intent drain, so a probe read
/// counts exactly what the injected intents emitted (the inject suite's shape).
fn add_probe<M: Message + Clone>(app: &mut App) {
    app.init_resource::<MessageProbe<M>>();
    app.add_systems(Update, drain_message_probe::<M>.after(dispatch_act_intents));
}

/// Advance `frames` frames of the pinned clock.
fn settle(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

/// Build the REAL Load-flow app with the REAL swap harness AND the REAL `net_qa` channel,
/// then drive it to a live battle on the pinned seed.
///
/// Returns `None` if the descent never rests, so the caller asserts the `Some`.
fn parity_battle_app() -> Option<(App, mpsc::Sender<IncomingRequest>)> {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    app.add_plugins(UiSwapHarnessPlugin);
    let (tx, rx) = mpsc::channel();
    app.add_plugins(NetQaPlugin::with_channels(rx));
    app.add_systems(Update, inject_egui_pointer_click);
    add_probe::<SetFacingRequested>(&mut app);
    add_probe::<MoveRequested>(&mut app);
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));

    assert!(
        app.is_plugin_added::<EguiPlugin>(),
        "the premise of this suite: on the render-stack app the harness really adds egui, \
         so the egui run below is a run WITH an egui pass",
    );

    if !advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    ) {
        return None;
    }
    drop(send(
        &tx,
        QaRequest::StartBattle {
            situation: SituationRef::new(SHIPPED_SITUATION.to_owned()),
            seed:      Some(SeedNet::new(PARITY_SEED)),
        },
    ));
    if !advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    ) {
        return None;
    }
    settle(&mut app, STEP_FRAMES);
    Some((app, tx))
}

/// Ask for a fresh battle snapshot.
fn snapshot(app: &mut App, tx: &mpsc::Sender<IncomingRequest>) -> Option<BattleView> {
    let reply = send(tx, QaRequest::GetBattleState);
    app.update();
    match reply.try_recv() {
        Ok(QaResponse::Battle(view)) => Some(view),
        _ => None,
    }
}

/// What one run of the script produced.
struct RunResult {
    /// How many turn acts the act bus emitted.
    turns:    usize,
    /// How many move acts the act bus emitted.
    moves:    usize,
    /// The token-free resulting snapshot.
    battle:   BattleView,
    /// Which ganger was selected at the end, by name.
    selected: Option<String>,
    /// The stack that was live for the run.
    live:     UiStackId,
}

/// Run the identical script with `stack` live, returning the app (still alive, for a
/// follow-up assertion) and what the run produced.
fn run_script_under(stack: UiStackNet) -> Option<(App, RunResult)> {
    let (mut app, tx) = parity_battle_app()?;
    drop(send(
        &tx,
        QaRequest::Inject(NetIntent::SwapUiStack { stack }),
    ));
    settle(&mut app, 2);

    // Where the battle's own auto-selected ganger stands — the script steps one cell east
    // of it, so the script is the same script on the same battle in both runs.
    let opening = snapshot(&mut app, &tx)?;
    let start = opening
        .gangers
        .iter()
        .find(|card| Some(card.token) == opening.selection.selected)?
        .position;
    let east = CellLevelNet::new(
        CellNet::new(CellXNet::new(*start.cell.x + 1), start.cell.y),
        start.level,
    );

    for message in [
        NetIntent::SetFacing {
            facing: FacingNet::East,
        },
        NetIntent::Move { dest: east },
        NetIntent::SetFacing {
            facing: FacingNet::South,
        },
        NetIntent::SelectNext,
    ] {
        drop(send(&tx, QaRequest::Inject(message)));
        settle(&mut app, STEP_FRAMES);
    }

    let battle = snapshot(&mut app, &tx)?;
    let result = RunResult {
        turns:    probed::<SetFacingRequested>(&app).len(),
        moves:    probed::<MoveRequested>(&app).len(),
        selected: selected_name(&battle),
        battle:   token_free(&battle),
        live:     live_stack(&app)?,
    };
    Some((app, result))
}

/// BEHAVIOURAL PARITY: one identical intent script, run once with each stack live, leaves
/// identical act-bus emissions and identical sim state.
///
/// The egui run is a run with a REAL egui pass — asserted twice over: `EguiPlugin` is added
/// on this app, and the run ends with a synthetic pointer click on the egui panel that only
/// swaps the stack if that panel was really drawn and really hit-tested. So a build where
/// the egui half swallowed, duplicated, or re-ordered the acts a script drives fails here.
#[test]
fn one_script_under_both_stacks_reaches_identical_state() -> Result<(), &'static str> {
    let (_, bevy_ui) =
        run_script_under(UiStackNet::BevyUi).ok_or("the bevy_ui run must complete")?;
    let (mut egui_app, egui) =
        run_script_under(UiStackNet::Egui).ok_or("the egui run must complete")?;

    assert_eq!(bevy_ui.live, UiStackId::BevyUi, "run 1 ran under bevy_ui");
    assert_eq!(egui.live, UiStackId::Egui, "run 2 ran under egui");

    assert!(bevy_ui.turns > 0, "the script must actually drive acts");
    assert_eq!(
        bevy_ui.turns, egui.turns,
        "the same script must drive the same number of turn acts under either stack",
    );
    assert!(bevy_ui.moves > 0, "…including the step");
    assert_eq!(bevy_ui.moves, egui.moves, "…and the same move acts");
    assert_eq!(
        bevy_ui.selected, egui.selected,
        "the selection must land on the same ganger under either stack",
    );
    assert!(
        bevy_ui.selected.is_some()
            && !bevy_ui.battle.gangers.is_empty()
            && !bevy_ui.battle.buttons.is_empty(),
        "non-vacuity: the compared snapshot must carry a real battle — gangers on the field, \
         one of them selected, and a HUD offering buttons",
    );
    assert_eq!(
        bevy_ui.battle, egui.battle,
        "the whole battle snapshot must be identical under either stack — gangers, terrain, \
         fog, selection, turn and HUD buttons alike",
    );

    // The egui run really had an egui pass: its panel answers a pointer click.
    click_egui_swap_button(&mut egui_app);
    assert_eq!(
        live_stack(&egui_app),
        Some(UiStackId::BevyUi),
        "the egui run must have been drawing a real, hit-testable egui panel throughout — a \
         click on it here is what proves the egui half was live while the script ran",
    );
    Ok(())
}
