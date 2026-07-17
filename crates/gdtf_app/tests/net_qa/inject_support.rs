//! Shared harness for the GTW-737 inject-path suite: a live-battle app wired with the
//! REAL `net_qa` inject pump, plus the spawn / request / probe helpers the tests drive.

use std::sync::mpsc;

use bevy::prelude::*;
use gdtf_app::test_support::{IncomingRequest, NetQaPlugin, Responder};
use gdtf_battle_input::{SelectedShooter, contextual::ContextualActSystems, dispatch_act_intents};
use gdtf_battle_sim::{
    effects::bleed::BleedingOut,
    prelude::{Cell, CellLevel, Faction, Level, LifeState, Position},
};
use gdtf_qa_protocol::{
    envelope::{QaRequest, QaResponse},
    ids::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet},
};
use gdtf_test_utils::{BattleAppBuilder, MessageProbe, drain_message_probe};

/// Build a headless app already at a live battle (`BattleAppBuilder`) with the REAL
/// `net_qa` inject pump wired to an injected inbox — returns the app and the sender the
/// test pushes requests on (exactly as the listener thread would). Returns `None` if the
/// battle drive never rests (the caller asserts the `Some`, keeping this harness panic-free).
pub(crate) fn inject_battle_app() -> Option<(App, mpsc::Sender<IncomingRequest>)> {
    let mut app = BattleAppBuilder::new().build()?;
    let (tx, rx) = mpsc::channel();
    app.add_plugins(NetQaPlugin::with_channels(rx));
    // Settle the freshly-added router + pump into the running battle.
    app.update();
    Some((app, tx))
}

/// Register a `MessageProbe<M>` draining AFTER the classic intent drain — pins the SAME
/// frame `dispatch_act_intents` emits `M`, so a probe read proves the injected classic
/// intent was consumed the frame it was injected.
pub(crate) fn add_classic_probe<M: Message + Clone>(app: &mut App) {
    app.init_resource::<MessageProbe<M>>();
    app.add_systems(Update, drain_message_probe::<M>.after(dispatch_act_intents));
}

/// Register a `MessageProbe<M>` draining AFTER the contextual drain set — pins the SAME
/// frame the per-act drain emits `M`.
pub(crate) fn add_contextual_probe<M: Message + Clone>(app: &mut App) {
    app.init_resource::<MessageProbe<M>>();
    app.add_systems(
        Update,
        drain_message_probe::<M>.after(ContextualActSystems::Drain),
    );
}

/// Clear a probe's accumulated messages — call right before the inject update so the
/// post-inject read counts ONLY the injected intent's emission (not battle-setup noise).
pub(crate) fn clear_probe<M: Message + Clone>(app: &mut App) {
    if let Some(mut probe) = app.world_mut().get_resource_mut::<MessageProbe<M>>() {
        probe.clear();
    }
}

/// A ground-level [`Position`] at cell `(x, y)`.
pub(crate) fn at(x: i32, y: i32) -> Position {
    Position::new(CellLevel::new(Cell::new(x, y), Level::new(0)))
}

/// Spawn an ACTOR (a ganger carrying the `(Position, Faction)` the offer scans read) at
/// `(x, y)` in gang `gang`, and SELECT it via the [`SelectedShooter`] resource. Returns
/// its entity. Mirrors the contextual-panel suite's proven spawner.
pub(crate) fn spawn_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app.world_mut().spawn((at(x, y), Faction::new(gang))).id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

/// Spawn a DOWNED, still-[`BleedingOut`] ally at `(x, y)` in gang `gang` — the components
/// the Stabilize offer scan + the inject-path liveness check read. Returns its entity.
pub(crate) fn spawn_downed_ally(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    app.world_mut()
        .spawn((at(x, y), Faction::new(gang), LifeState::Downed, BleedingOut))
        .id()
}

/// Build a wire `(cell, storey)` key.
pub(crate) const fn cell_level_net(x: i32, y: i32, z: u8) -> CellLevelNet {
    CellLevelNet::new(
        CellNet::new(CellXNet::new(x), CellYNet::new(y)),
        LevelNet::new(z),
    )
}

/// Push a request onto the inject pump's inbox, returning the channel its reply arrives
/// on (exactly as the listener thread would hand the request over).
pub(crate) fn send(
    tx: &mpsc::Sender<IncomingRequest>,
    request: QaRequest,
) -> mpsc::Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    let sent = tx.send(IncomingRequest::new(request, responder));
    assert!(sent.is_ok(), "the inject inbox must be open");
    reply_rx
}
