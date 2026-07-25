//! The DEV swap-harness read the battle snapshot folds in (GTW-816).
//!
//! Lets a QA client that swapped stacks read back WHICH stack is live, so a parity
//! assertion across a swap can name the configuration it is asserting about rather than
//! assuming it.
//!
//! Compiled in TWO forms so a `net_qa` build without `dev_tools` still builds: with
//! `dev_tools` it reads the harness's [`UiStack`](crate::dev::ui_swap::UiStack) resource;
//! without it the harness module does not exist at all and the view is always `None` — the
//! honest answer for a build that has no swap harness to report on.

use bevy::ecs::system::SystemParam;
use gdtf_qa_protocol::view::UiStackView;

/// The swap-harness read, as a [`SystemParam`] — the `dev_tools` body.
///
/// `Option<Res<…>>` (`bevy-traps.md` #1): the snapshot service runs in builds where the
/// harness may not have registered, and reports `None` rather than panicking.
#[cfg(feature = "dev_tools")]
#[derive(SystemParam)]
pub(in crate::dev::net_qa) struct UiStackRead<'w> {
    /// The live swap-harness state, absent where the harness never registered.
    stack: Option<bevy::prelude::Res<'w, crate::dev::ui_swap::UiStack>>,
}

/// Project the harness state into its wire view — the compared pair plus the live stack —
/// or `None` when no harness is present.
///
/// A free function over the read (rather than a method) so the two cfg bodies present the
/// SAME call shape to the snapshot assembly, including the `dev_tools`-less one that has no
/// state to read.
#[cfg(feature = "dev_tools")]
pub(in crate::dev::net_qa) fn ui_stack_view(read: &UiStackRead) -> Option<UiStackView> {
    use gdtf_qa_protocol::{
        intent::UiStackNet,
        view::{UiStackPairView, UiStackView as View},
    };

    use crate::dev::ui_swap::UiStackId;

    /// Map the game's stack identifier onto its wire name.
    const fn stack_net(id: UiStackId) -> UiStackNet {
        match id {
            UiStackId::BevyUi => UiStackNet::BevyUi,
            UiStackId::Egui => UiStackNet::Egui,
        }
    }

    let stack = *read.stack.as_deref()?;
    let pair = stack.comparison();
    Some(View::new(
        UiStackPairView::new(stack_net(pair.baseline()), stack_net(pair.candidate())),
        stack_net(stack.live()),
    ))
}

/// The `dev_tools`-less build's read: there is no swap harness compiled in, so there is
/// nothing to report. Touches NO harness type, so the `net_qa`-without-`dev_tools` build
/// compiles.
#[cfg(not(feature = "dev_tools"))]
#[derive(SystemParam)]
pub(in crate::dev::net_qa) struct UiStackRead;

/// Always `None` — this build has no swap harness to report on.
#[cfg(not(feature = "dev_tools"))]
pub(in crate::dev::net_qa) const fn ui_stack_view(_read: &UiStackRead) -> Option<UiStackView> {
    None
}
