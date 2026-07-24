//! The menu-item activation consumer (GTW-787).
//!
//! [`drive_activate_menu_item`] drains the routed [`ActivateMenuPayload`] queue the T3
//! router fills and, for each request, activates the named menu item through the game's
//! REAL focus-activation path: it resolves the token to a live
//! [`MenuItem`](gdtf_ui::MenuItem) entity and raises the SAME
//! [`FocusActivated`](gdtf_ui::focus_nav::FocusActivated) message an `Enter` keypress
//! raises while that item holds focus — so a scene that already consumes that message (the
//! menu's `focus_activated_actions`) reacts identically whether the activation came from a
//! keyboard or the wire. It never writes `NextState` itself and never reaches into a
//! scene's internals.
//!
//! Fail-closed (GTW-787 parity clause): a token that does not resolve to a live, listed
//! [`MenuItem`] — a malformed bit pattern, a despawned entity, or a live entity that is not
//! a menu item — is answered
//! [`Rejected`](gdtf_qa_protocol::envelope::MenuActivationReceipt::Rejected)`(`[`StaleToken`](gdtf_qa_protocol::envelope::RejectReason::StaleToken)`)`,
//! never a panic (via [`Entity::try_from_bits`], never `from_bits`) and never a silent
//! no-op.

use bevy::{ecs::message::Messages, prelude::*};
use gdtf_qa_protocol::{
    envelope::{MenuActivationReceipt, QaResponse, RejectReason},
    ids::FocusTargetNet,
};
use gdtf_ui::{MenuItem, focus_nav::FocusActivated};

use super::pending::{ActivateMenuPayload, PendingQueue};

/// Drain the routed [`ActivateMenuPayload`] queue and activate each named menu item,
/// answering every request THIS frame (GTW-787).
///
/// Registered in [`InputSystems::Gather`](gdtf_battle_input::InputSystems)
/// `.after(route_requests)` by [`super::plugin`] — so it sees the same frame's routed
/// pushes. Always runs (so it can reject a stale token rather than leave it to the deadline
/// sweep). Per request:
///
/// - the token resolves to a live [`MenuItem`] entity → raise
///   [`FocusActivated`] for it (the real activation path) and answer
///   [`Activated`](MenuActivationReceipt::Activated);
/// - otherwise → answer
///   [`Rejected`](MenuActivationReceipt::Rejected)`(`[`StaleToken`](RejectReason::StaleToken)`)`.
///
/// The [`FocusActivated`] message stream is read as `Option<ResMut<Messages<_>>>` so the
/// system stays inert-safe when the focus-nav framework is absent (`MinimalPlugins`, no
/// [`FocusNavPlugin`](gdtf_ui::focus_nav::FocusNavPlugin)); the real app always has it. When
/// the stream is absent a resolved item still answers `Activated` (the token was valid) — the
/// activation message simply has nowhere to go, an outcome concern, not a wire rejection.
pub(super) fn drive_activate_menu_item(
    mut queue: ResMut<PendingQueue<ActivateMenuPayload>>,
    menu_items: Query<(), With<MenuItem>>,
    mut activations: Option<ResMut<Messages<FocusActivated>>>,
) {
    for (payload, responder) in queue.drain_ready() {
        let receipt = match resolve_menu_item(payload.token(), &menu_items) {
            Some(entity) => {
                if let Some(activations) = activations.as_deref_mut() {
                    activations.write(FocusActivated::new(entity));
                }
                MenuActivationReceipt::Activated
            }
            None => MenuActivationReceipt::Rejected(RejectReason::StaleToken),
        };
        responder.reply(QaResponse::MenuItemActivated(receipt));
    }
}

/// Resolve a [`FocusTargetNet`] to a live, listed [`MenuItem`] entity FAIL-CLOSED: a
/// malformed bit pattern (via [`Entity::try_from_bits`], never `from_bits`, which panics) or
/// an entity that is not a live menu item yields [`None`] (answered
/// [`StaleToken`](RejectReason::StaleToken) by the caller).
fn resolve_menu_item(
    token: FocusTargetNet,
    menu_items: &Query<(), With<MenuItem>>,
) -> Option<Entity> {
    let entity = Entity::try_from_bits(*token)?;
    menu_items.get(entity).ok().map(|()| entity)
}
