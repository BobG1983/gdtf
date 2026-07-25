//! The ONE swap funnel: a single-slot, idempotent latch every trigger writes, and the one
//! `Update` system that turns a latched request into exactly one change of the live stack.
//!
//! # Why every trigger latches instead of flipping [`UiStack`](super::stack::UiStack)
//!
//! Three surfaces ask for a swap: the keyboard shortcut, the `net_qa`
//! [`SwapUiStack`](gdtf_qa_protocol::intent::NetIntent::SwapUiStack) intent, and each
//! stack's own on-screen swap button. If each flipped the resource itself there would be
//! three implementations of "what a swap means", and the egui one would be WRONG: egui's
//! multipass runs its closure up to TWICE per frame with the same pointer input
//! (`bevy-traps.md` #8b), so a flip inside the closure records one user click as two swaps
//! — a round trip, i.e. a swap key that visibly does nothing.
//!
//! So a trigger only ever WRITES this single-slot latch, which is idempotent: writing it
//! twice in one frame leaves the same world as writing it once, because the second write
//! overwrites the first slot rather than queueing a second swap.
//! [`apply_ui_stack_swap`] then consumes the slot in `Update` and performs exactly one
//! change. One funnel, so the keyboard path and the wire path cannot diverge.
//!
//! The egui pass runs in `PostUpdate`, so a latch written from the egui side is applied on
//! the FOLLOWING frame — the harness is eventually consistent by one frame after an
//! egui-originated swap (the same one-frame lag the GTW-819 spike recorded). The keyboard
//! and wire paths both write during `Update` ahead of this system, so they apply the same
//! frame.

use bevy::prelude::*;

use super::stack::{UiStack, UiStackId};

/// What a latched swap asks for.
///
/// Two shapes, because the two kinds of trigger genuinely differ: a key press (and an
/// on-screen swap button) means "show me the other one" without naming it, while the wire
/// intent names an ABSOLUTE stack so a script can reach a known state without first reading
/// the current one. Module-private: outside callers name a CONSTRUCTOR
/// ([`PendingUiStackSwap::toggle`] / [`PendingUiStackSwap::set_live`]), never this shape.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub(super) enum UiStackSwapRequest {
    /// Flip to the other member of the compared pair.
    Toggle,
    /// Make this specific stack live (a no-op if it already is).
    SetLive(UiStackId),
}

crate::support_item! {
    /// The single-slot swap latch — present iff a swap has been requested and not yet
    /// applied.
    ///
    /// A RESOURCE rather than a message queue on purpose: a resource has exactly one slot,
    /// so N requests in one frame collapse to one swap. That is what makes an egui
    /// multipass re-run harmless (see the module doc).
    #[derive(Resource, Debug, Clone, Copy, Eq, PartialEq, Hash)]
    struct PendingUiStackSwap(UiStackSwapRequest);
}

impl PendingUiStackSwap {
    /// Latch a flip to the other compared stack — what the keyboard shortcut and both
    /// on-screen swap buttons request.
    pub(super) const fn toggle() -> Self {
        Self(UiStackSwapRequest::Toggle)
    }

    crate::support_item! {
        /// Latch an absolute "make this stack live" — what the wire intent requests.
        #[must_use]
        const fn set_live(stack: UiStackId) -> Self {
            Self(UiStackSwapRequest::SetLive(stack))
        }
    }

    /// The latched request.
    const fn request(self) -> UiStackSwapRequest {
        self.0
    }
}

/// `Update`: apply a latched swap to [`UiStack`], exactly once, then clear the latch.
///
/// The ONE writer of the live stack — every trigger reaches the live stack through here,
/// so the keyboard, the wire, and the on-screen buttons cannot drift apart.
///
/// Takes both the latch and the stack as `Option` (`bevy-traps.md` #1) so it is inert
/// rather than panicking in any world where the harness resources are absent. Param-only
/// (`bevy-traps.md` #7): two optional resources plus [`Commands`].
pub(super) fn apply_ui_stack_swap(
    pending: Option<Res<PendingUiStackSwap>>,
    stack: Option<ResMut<UiStack>>,
    mut commands: Commands,
) {
    let Some(pending) = pending else {
        return;
    };
    // Clear the slot whether or not the stack resource is there to change, so a latch
    // written in a harness-less world cannot linger and fire on some later frame.
    commands.remove_resource::<PendingUiStackSwap>();
    let Some(mut stack) = stack else {
        return;
    };
    match pending.request() {
        UiStackSwapRequest::Toggle => stack.toggle(),
        UiStackSwapRequest::SetLive(id) => stack.set_live(id),
    }
    info!("ui-swap: live UI stack is now {:?}", stack.live());
}
