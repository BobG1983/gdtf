//! The wire → DEV-swap-harness bridge for
//! [`SwapUiStack`](gdtf_qa_protocol::intent::NetIntent::SwapUiStack) (GTW-816).
//!
//! The injected intent writes the SAME single-slot swap latch the harness's keyboard
//! shortcut and both on-screen swap buttons write — never a second flip path, so an agent
//! driving the swap over the wire and a developer pressing the key land in exactly one
//! implementation of "what a swap means" (and inherit its multipass double-swap guard for
//! free).
//!
//! Compiled in TWO forms so a `net_qa` build without `dev_tools` still builds: with
//! `dev_tools` it writes the harness latch; without it the harness module does not exist at
//! all and the intent is answered
//! [`Rejected(Unavailable)`](gdtf_qa_protocol::envelope::RejectReason::Unavailable) —
//! fail-closed and honest, rather than a `Queued` for a swap that can never happen.

use bevy::prelude::Commands;
use gdtf_qa_protocol::{envelope::InjectReceipt, intent::UiStackNet};

/// Ask the DEV swap harness to make `stack` live, answering
/// [`Queued`](InjectReceipt::Queued).
///
/// The receipt says the request entered the harness's latch, not that the screen has
/// changed: the latch is applied by the harness's own `Update` system, exactly as it is for
/// a keypress.
#[cfg(feature = "dev_tools")]
pub(super) fn queue_ui_swap(stack: UiStackNet, commands: &mut Commands) -> InjectReceipt {
    use crate::dev::ui_swap::{PendingUiStackSwap, UiStackId};

    let id = match stack {
        UiStackNet::BevyUi => UiStackId::BevyUi,
        UiStackNet::Egui => UiStackId::Egui,
    };
    commands.insert_resource(PendingUiStackSwap::set_live(id));
    InjectReceipt::Queued
}

/// The `dev_tools`-less build's answer: there is no swap harness in this build, so the
/// intent is rejected [`Unavailable`](gdtf_qa_protocol::envelope::RejectReason::Unavailable)
/// rather than silently dropped. Touches NO harness type, so the
/// `net_qa`-without-`dev_tools` build compiles.
#[cfg(not(feature = "dev_tools"))]
pub(super) const fn queue_ui_swap(_stack: UiStackNet, _commands: &mut Commands) -> InjectReceipt {
    InjectReceipt::Rejected(gdtf_qa_protocol::envelope::RejectReason::Unavailable)
}
