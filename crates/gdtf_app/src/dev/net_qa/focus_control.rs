//! The focus-drive consumer (GTW-802).
//!
//! [`drive_focus_control`] drains the routed [`FocusControlPayload`] queue the T3 router
//! fills and realises each command through the game's REAL focus / input path, so ANY
//! focus-navigable `bevy_ui` screen — not just one carrying the menu markers — is drivable
//! over the wire, in or out of a battle.
//!
//! ## How each command reaches the real path
//!
//! - [`Step`](FocusStepNet) writes the SAME
//!   [`NavigateRequest`](gdtf_ui::focus_nav::NavigateRequest) message the keyboard /
//!   gamepad bridge writes for an arrow key, and lets `gdtf_ui`'s own
//!   [`apply_navigation`](gdtf_ui::focus_nav::apply_navigation) perform the move.
//! - [`Focus`](FocusCommandNet::Focus) performs the SAME
//!   [`InputFocus::set`](bevy::input_focus::InputFocus::set) with
//!   [`FocusCause::Navigated`] the hover-to-focus bridge (and the GTW-783 `SetFocus`
//!   intent) performs.
//! - [`Activate`](FocusCommandNet::Activate) /
//!   [`ActivateTarget`](FocusCommandNet::ActivateTarget) emit a REAL `Enter` keypress on
//!   the primary window ([`activation_key_tap`]) rather than forging an activation message.
//!
//! ## Why activation is a keypress, not a forged message
//!
//! There are TWO activation consumer families on the real screens, and only one of them
//! reads `gdtf_ui`'s [`FocusActivated`](gdtf_ui::focus_nav::FocusActivated):
//!
//! - the project's own readers — the menu's activation actions and the Options screen's
//!   Continue bridge — read `FocusActivated`;
//! - the first-party `bevy_ui_widgets` widgets — the Options SOUND CHECKBOX among them —
//!   read `On<FocusedInput<KeyboardInput>>` and never look at `FocusActivated` at all.
//!
//! So writing a `FocusActivated` (the GTW-787 mechanism) would leave the Options checkbox
//! untoggleable. Emitting the key instead is one step FURTHER UPSTREAM on the same path,
//! and reaches both families through their own code with nothing forged: Bevy's
//! `keyboard_input_system` folds the key into `ButtonInput<KeyCode>`, from which `gdtf_ui`'s
//! own keyboard bridge raises the very same `FocusActivated` a player's `Enter` raises; and
//! Bevy's `dispatch_focused_input` triggers the `FocusedInput<KeyboardInput>` the checkbox
//! reads. This module writes no `NextState` and reaches into no scene's internals.
//!
//! The cost is honest and mirrors [`InjectReceipt::Queued`](gdtf_qa_protocol::envelope::InjectReceipt::Queued):
//! the emitted messages are consumed in the NEXT frame's `PreUpdate`, so
//! [`Applied`](FocusControlReceipt::Applied) means "dispatched", and the resulting state
//! change is observed with a follow-up
//! [`GetAppFlow`](gdtf_qa_protocol::envelope::QaRequest::GetAppFlow).
//!
//! Fail-closed: a token that does not name a live, currently LISTED focusable — a malformed
//! bit pattern, a despawned entity, or a live entity that is not in the focus graph — is
//! answered [`Rejected`](FocusControlReceipt::Rejected)`(`[`StaleToken`](RejectReason::StaleToken)`)`,
//! never a panic (via [`Entity::try_from_bits`], never `from_bits`) and never a silent
//! no-op. An [`Activate`](FocusCommandNet::Activate) with nothing focused is the same
//! rejection.

use bevy::{
    ecs::{message::Messages, system::SystemParam},
    input::keyboard::KeyboardInput,
    input_focus::{FocusCause, InputFocus, directional_navigation::DirectionalNavigationMap},
    prelude::*,
    window::PrimaryWindow,
};
use gdtf_qa_protocol::{
    envelope::{FocusCommandNet, FocusControlReceipt, FocusStepNet, QaResponse, RejectReason},
    ids::FocusTargetNet,
};
use gdtf_ui::focus_nav::{NavDirection, NavigateRequest};

use super::{
    key_tap::activation_key_tap,
    pending::{FocusControlPayload, PendingQueue},
};

/// The write-side bundle [`drive_focus_control`] drives — the SAME focus / windowing-input
/// path the player's own keyboard drives.
///
/// Every resource is `Option`-wrapped and every query empty-safe, so the consumer is
/// inert-safe when the focus / windowing stack is absent (`bevy-traps.md` #1) rather than
/// failing param validation; the real app always has them.
#[derive(SystemParam)]
pub(super) struct FocusDriveSink<'w, 's> {
    /// The focus graph — the set a token must be LISTED in to resolve.
    nav_map:    Option<Res<'w, DirectionalNavigationMap>>,
    /// Where input focus sits — read by an `Activate`, written by a `Focus`.
    focus:      Option<ResMut<'w, InputFocus>>,
    /// The navigate-request stream a `Step` writes, exactly as the arrow-key bridge does.
    navigate:   Option<ResMut<'w, Messages<NavigateRequest>>>,
    /// The buffered keyboard stream an activation's real `Enter` tap writes.
    key_events: Option<ResMut<'w, Messages<KeyboardInput>>>,
    /// The primary window an emitted key is attributed to.
    windows:    Query<'w, 's, Entity, With<PrimaryWindow>>,
    /// An all-entity liveness probe — a focus token resolves only to an entity that exists.
    entities:   Query<'w, 's, Entity>,
}

/// Drain the routed [`FocusControlPayload`] queue and realise each focus command, answering
/// every request THIS frame (GTW-802).
///
/// Registered in [`InputSystems::Gather`](gdtf_battle_input::InputSystems)
/// `.after(route_requests)` — so it sees the same frame's routed pushes — and
/// `.before(FocusNavSystems::Apply)`, so a `Step` raised this frame moves focus this frame
/// rather than dropping one (`bevy-traps.md` #3). Always runs (so it can reject a stale
/// token rather than leave it to the deadline sweep), and is NEVER battle-gated.
pub(super) fn drive_focus_control(
    mut queue: ResMut<PendingQueue<FocusControlPayload>>,
    mut sink: FocusDriveSink,
) {
    for (payload, responder) in queue.drain_ready() {
        let receipt = apply_command(payload.command(), &mut sink);
        responder.reply(QaResponse::FocusControlled(receipt));
    }
}

/// Realise ONE focus command against the real path, returning its receipt.
fn apply_command(command: FocusCommandNet, sink: &mut FocusDriveSink) -> FocusControlReceipt {
    match command {
        FocusCommandNet::Step(step) => {
            step_focus(step, sink);
            FocusControlReceipt::Applied
        }
        FocusCommandNet::Focus(token) => match resolve_listed(token, sink) {
            Some(entity) => {
                point_focus(entity, sink);
                FocusControlReceipt::Applied
            }
            None => FocusControlReceipt::Rejected(RejectReason::StaleToken),
        },
        FocusCommandNet::ActivateTarget(token) => match resolve_listed(token, sink) {
            Some(entity) => {
                point_focus(entity, sink);
                activate(sink);
                FocusControlReceipt::Applied
            }
            None => FocusControlReceipt::Rejected(RejectReason::StaleToken),
        },
        FocusCommandNet::Activate => {
            if focused_and_listed(sink).is_some() {
                activate(sink);
                FocusControlReceipt::Applied
            } else {
                FocusControlReceipt::Rejected(RejectReason::StaleToken)
            }
        }
    }
}

/// Write the SAME [`NavigateRequest`] the arrow-key / D-pad bridge writes, leaving the move
/// itself to `gdtf_ui`'s own navigation system. Inert if the stream is absent.
fn step_focus(step: FocusStepNet, sink: &mut FocusDriveSink) {
    let direction = match step {
        FocusStepNet::Next => NavDirection::DOWN,
        FocusStepNet::Prev => NavDirection::UP,
        FocusStepNet::Left => NavDirection::WEST,
        FocusStepNet::Right => NavDirection::EAST,
    };
    if let Some(navigate) = sink.navigate.as_deref_mut() {
        navigate.write(NavigateRequest::new(direction));
    }
}

/// Point input focus at `entity` — the SAME write the hover-to-focus bridge performs.
fn point_focus(entity: Entity, sink: &mut FocusDriveSink) {
    if let Some(focus) = sink.focus.as_deref_mut() {
        focus.set(entity, FocusCause::Navigated);
    }
}

/// Emit the real activation keypress on the primary window (see the module doc for why this
/// is a key and not a forged activation message).
fn activate(sink: &mut FocusDriveSink) {
    let window = sink.windows.iter().next().unwrap_or(Entity::PLACEHOLDER);
    activation_key_tap(window, sink.key_events.as_deref_mut());
}

/// Resolve a [`FocusTargetNet`] to a live, LISTED focusable FAIL-CLOSED: a malformed bit
/// pattern (via [`Entity::try_from_bits`], never `from_bits`, which panics), an entity that
/// no longer exists, or a live entity the focus graph does not list all yield [`None`]
/// (answered [`StaleToken`](RejectReason::StaleToken) by the caller).
///
/// Membership in the focus graph — not mere liveness — is the gate, so a client can only
/// ever drive a control the [`FocusView`](gdtf_qa_protocol::view::FocusView) handout listed.
fn resolve_listed(token: FocusTargetNet, sink: &FocusDriveSink) -> Option<Entity> {
    let entity = Entity::try_from_bits(*token)?;
    is_listed(entity, sink).then_some(entity)
}

/// The currently focused entity, if it is still live and listed — an `Activate` with
/// nothing focused (or a focus left pointing at a dead / unlisted entity) resolves to
/// [`None`] and is rejected rather than emitting a key into nowhere.
fn focused_and_listed(sink: &FocusDriveSink) -> Option<Entity> {
    let entity = sink.focus.as_ref()?.get()?;
    is_listed(entity, sink).then_some(entity)
}

/// Whether `entity` is both live and a key of the focus graph.
fn is_listed(entity: Entity, sink: &FocusDriveSink) -> bool {
    sink.nav_map
        .as_ref()
        .is_some_and(|nav_map| nav_map.neighbors.contains_key(&entity))
        && sink.entities.contains(entity)
}
