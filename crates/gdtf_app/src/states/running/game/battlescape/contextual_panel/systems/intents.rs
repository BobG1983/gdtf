//! Routes a contextual-button press to the shared 222a act-intent seam (GTW-294 live slice).
//!
//! This is the contextual panel's INTERACTION → INTENT layer — the exact action-bar pattern: a
//! button press [`push`](gdtf_battle_input::PendingActIntent::push)es an
//! [`ActIntent`](gdtf_battle_input::ActIntent) onto the ONE
//! [`PendingActIntent`](gdtf_battle_input::PendingActIntent) seam that
//! `gdtf_battle_input`'s keyboard surface also feeds, and the SINGLE
//! [`dispatch_act_intents`](gdtf_battle_input::dispatch_act_intents) drain interprets it — so
//! there is exactly ONE emission path, never a divergent marker → `*Requested` mapping
//! (ADR-0001). The button does NOT independently write any `gdtf_battle_sim::acts::*Requested`.
//!
//! ## The press mechanism, routed to the carried target
//!
//! Each per-act query is filtered [`PressedButton<M>`] (`= (Changed<Interaction>, With<M>,
//! Without<DisabledButton>)`) and acts only on [`Interaction::Pressed`] ([`is_press`]) — the
//! action-bar `action_bar_button_intents` mechanism, reused verbatim. Unlike a global act, each
//! contextual press carries the TARGET the detection system named on the
//! [`ContextualTargets`] seam: an Execute press pushes
//! [`ActIntent::Execute(target)`](gdtf_battle_input::ActIntent::Execute), a Stabilize press
//! pushes [`ActIntent::Stabilize(target)`](gdtf_battle_input::ActIntent::Stabilize), a Shove press
//! pushes [`ActIntent::Shove(target)`](gdtf_battle_input::ActIntent::Shove) (GTW-525). The
//! `Some`-guard on the seam keeps a stale press (the panel was just hidden) safe — with no
//! target nothing is queued.

use bevy::{prelude::*, ui::Interaction};
use gdtf_battle_input::{ActIntent, PendingActIntent};
use gdtf_ui::DisabledButton;

use crate::states::running::game::battlescape::contextual_panel::components::{
    ContextualTargets, ExecuteButton, MeleeButton, ShoveButton, StabilizeButton,
};

/// Query filter selecting the ENABLED button carrying marker `M` whose [`Interaction`] became a
/// press this frame — the contextual panel's mirror of the action-bar `PressedButton` filter.
///
/// Factored into a named alias to keep [`contextual_button_intents`]'s signature legible
/// (clippy `type_complexity`). `Without<DisabledButton>` skips a disabled button (none of the
/// contextual buttons disable today, but the filter stays faithful to the press mechanism), and
/// `Changed<Interaction>` limits each query to the frame a press lands. The action-bar
/// `action_bar_button_intents` precedent (its `PressedButton` is `pub(in …action_bar)`, so this
/// sibling panel mirrors it rather than reaching across the action-bar subtree).
type PressedButton<M> = (Changed<Interaction>, With<M>, Without<DisabledButton>);

/// Whether an [`Interaction`] is a fresh press to act on — only [`Interaction::Pressed`].
///
/// The action-bar `is_press` precedent (its copy is `pub(in …action_bar)`, so this sibling
/// panel mirrors the one-liner rather than reaching across the action-bar subtree). Takes
/// [`Interaction`] by value (a one-byte `Copy` enum).
const fn is_press(interaction: Interaction) -> bool {
    matches!(interaction, Interaction::Pressed)
}

/// Routes each contextual-button press to its target-carrying [`ActIntent`] on the shared 222a
/// seam (GTW-294).
///
/// For each contextual act button whose [`Interaction`] changed to
/// [`Pressed`](Interaction::Pressed) this frame AND whose
/// [`ContextualTargets`] field names a target, [`push`](PendingActIntent::push)es the matching
/// intent carrying that downed target:
///
/// - [`ExecuteButton`] + [`ContextualTargets::execute`] `== Some(target)` →
///   [`ActIntent::Execute(target)`](ActIntent::Execute)
/// - [`StabilizeButton`] + [`ContextualTargets::stabilize`] `== Some(target)` →
///   [`ActIntent::Stabilize(target)`](ActIntent::Stabilize)
/// - [`MeleeButton`] + [`ContextualTargets::melee`] `== Some(target)` →
///   [`ActIntent::Melee(target)`](ActIntent::Melee) (GTW-507)
/// - [`ShoveButton`] + [`ContextualTargets::shove`] `== Some(target)` →
///   [`ActIntent::Shove(target)`](ActIntent::Shove) (GTW-525)
///
/// The `Some`-guard is the safety: a press with no offered target (a stale press after the
/// panel hid) queues nothing. The button writes NO `*Requested` directly — the ONE
/// [`dispatch_act_intents`](gdtf_battle_input::dispatch_act_intents) drain emits
/// [`ExecuteDownedRequested`](gdtf_battle_sim::acts::ExecuteDownedRequested) /
/// [`StabilizeDownedRequested`](gdtf_battle_sim::acts::StabilizeDownedRequested) /
/// [`MeleeRequested`](gdtf_battle_sim::acts::MeleeRequested) /
/// [`ShoveRequested`](gdtf_battle_sim::acts::ShoveRequested) for the
/// [`SelectedShooter`](gdtf_battle_input::SelectedShooter) as the actor (a no-op there with no
/// selection), and the sim's faction + reach (+ LOS for melee) gates are the authoritative check.
///
/// Registered `.before(dispatch_act_intents)` so a press queued this update is drained this
/// update — the same-frame guarantee the keyboard writers + the action bar get (`bevy-traps.md`
/// #3). Param-only (`bevy-traps.md` #7): the read-only [`ContextualTargets`] seam, four
/// disjoint per-marker `Query<&Interaction, …>`s, and the `ResMut<PendingActIntent>` write.
pub(in crate::states::running::game::battlescape) fn contextual_button_intents(
    targets: Res<ContextualTargets>,
    mut pending: ResMut<PendingActIntent>,
    execute_btn: Query<&Interaction, PressedButton<ExecuteButton>>,
    stabilize_btn: Query<&Interaction, PressedButton<StabilizeButton>>,
    melee_btn: Query<&Interaction, PressedButton<MeleeButton>>,
    shove_btn: Query<&Interaction, PressedButton<ShoveButton>>,
) {
    if execute_btn.iter().copied().any(is_press)
        && let Some(target) = targets.execute()
    {
        pending.push(ActIntent::Execute(target));
    }
    if stabilize_btn.iter().copied().any(is_press)
        && let Some(target) = targets.stabilize()
    {
        pending.push(ActIntent::Stabilize(target));
    }
    // The dedicated Melee button routes to a GANGER strike (GTW-507) or, when no meleeable
    // ganger is offered, an adjacent-structure SMASH (GTW-508) — the ONE button, two target
    // kinds. The ganger target takes priority (checked first); the `Some`-guards keep a stale
    // press safe.
    if melee_btn.iter().copied().any(is_press) {
        if let Some(target) = targets.melee() {
            pending.push(ActIntent::Melee(target));
        } else if let Some(at) = targets.melee_structure() {
            pending.push(ActIntent::MeleeStructure(at));
        }
    }
    // The dedicated Shove button (GTW-525) — a deliberate, universal knock-back on an 8-adjacent
    // alive opposing ganger. The `Some`-guard keeps a stale press safe.
    if shove_btn.iter().copied().any(is_press)
        && let Some(target) = targets.shove()
    {
        pending.push(ActIntent::Shove(target));
    }
}
