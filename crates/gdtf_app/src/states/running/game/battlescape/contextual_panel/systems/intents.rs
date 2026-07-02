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
//! pushes [`ActIntent::Shove(target)`](gdtf_battle_input::ActIntent::Shove) (GTW-525), an Open-Door
//! press pushes [`ActIntent::OpenDoor(door)`](gdtf_battle_input::ActIntent::OpenDoor) (GTW-315), an
//! Enter press pushes
//! [`ActIntent::EnterEmplacement(emplacement)`](gdtf_battle_input::ActIntent::EnterEmplacement) and
//! an Exit press pushes
//! [`ActIntent::ExitEmplacement(emplacement)`](gdtf_battle_input::ActIntent::ExitEmplacement)
//! (GTW-543). The `Some`-guard on the seam keeps a stale press (the panel was just hidden) safe —
//! with no target nothing is queued.

use bevy::{ecs::system::SystemParam, prelude::*, ui::Interaction};
use gdtf_battle_input::{ActIntent, PendingActIntent};
use gdtf_ui::DisabledButton;

use crate::states::running::game::battlescape::contextual_panel::components::{
    ContextualTargets, EnterEmplacementButton, ExecuteButton, ExitEmplacementButton, MeleeButton,
    OpenDoorButton, ShoveButton, StabilizeButton, ThrowGrenadeButton,
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

/// The eight disjoint per-marker press queries, grouped into ONE
/// [`SystemParam`](bevy::ecs::system::SystemParam) so [`contextual_button_intents`] stays under
/// clippy's argument-count gate (the input seam's `ActWriters` / detect's `LosGrids` precedent).
///
/// Each field is a `Query<&Interaction, `[`PressedButton<M>`]`>` matching exactly the button whose
/// [`Interaction`] became a press this frame; the queries are disjoint by marker (`With<M>`), so
/// grouping them is a pure legibility bundle, not a borrow change. A transparent system-param
/// bundle of read-only queries — not itself a wrapped domain scalar.
#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape) struct ContextualButtonPresses<'w, 's> {
    /// The **Execute** button's this-frame press query (GTW-294).
    execute:           Query<'w, 's, &'static Interaction, PressedButton<ExecuteButton>>,
    /// The **Stabilize** button's this-frame press query (GTW-294).
    stabilize:         Query<'w, 's, &'static Interaction, PressedButton<StabilizeButton>>,
    /// The **Melee** button's this-frame press query (GTW-507).
    melee:             Query<'w, 's, &'static Interaction, PressedButton<MeleeButton>>,
    /// The **Shove** button's this-frame press query (GTW-525).
    shove:             Query<'w, 's, &'static Interaction, PressedButton<ShoveButton>>,
    /// The **Open Door** button's this-frame press query (GTW-315).
    open_door:         Query<'w, 's, &'static Interaction, PressedButton<OpenDoorButton>>,
    /// The **Enter Emplacement** button's this-frame press query (GTW-543).
    enter_emplacement: Query<'w, 's, &'static Interaction, PressedButton<EnterEmplacementButton>>,
    /// The **Exit Emplacement** button's this-frame press query (GTW-543).
    exit_emplacement:  Query<'w, 's, &'static Interaction, PressedButton<ExitEmplacementButton>>,
    /// The **Throw** button's this-frame press query (GTW-546).
    throw_grenade:     Query<'w, 's, &'static Interaction, PressedButton<ThrowGrenadeButton>>,
}

impl ContextualButtonPresses<'_, '_> {
    /// Whether the button carrying marker `M`'s query holds a fresh [`Interaction::Pressed`] this
    /// frame — the shared predicate every arm calls on its bundle field.
    fn pressed<F: bevy::ecs::query::QueryFilter>(query: &Query<&Interaction, F>) -> bool {
        query.iter().copied().any(is_press)
    }
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
/// - [`OpenDoorButton`] + [`ContextualTargets::open_door`] `== Some(door)` →
///   [`ActIntent::OpenDoor(door)`](ActIntent::OpenDoor) (GTW-315)
/// - [`EnterEmplacementButton`] + [`ContextualTargets::enter_emplacement`] `== Some(emplacement)` →
///   [`ActIntent::EnterEmplacement(emplacement)`](ActIntent::EnterEmplacement) (GTW-543)
/// - [`ExitEmplacementButton`] + [`ContextualTargets::exit_emplacement`] `== Some(emplacement)` →
///   [`ActIntent::ExitEmplacement(emplacement)`](ActIntent::ExitEmplacement) (GTW-543)
/// - [`ThrowGrenadeButton`] + [`ContextualTargets::throw_grenade`] `== Some(at)` →
///   [`ActIntent::ThrowGrenade(at)`](ActIntent::ThrowGrenade) (GTW-546)
///
/// The `Some`-guard is the safety: a press with no offered target (a stale press after the
/// panel hid) queues nothing. The button writes NO `*Requested` directly — the ONE
/// [`dispatch_act_intents`](gdtf_battle_input::dispatch_act_intents) drain emits
/// [`ExecuteDownedRequested`](gdtf_battle_sim::acts::ExecuteDownedRequested) /
/// [`StabilizeDownedRequested`](gdtf_battle_sim::acts::StabilizeDownedRequested) /
/// [`MeleeRequested`](gdtf_battle_sim::acts::MeleeRequested) /
/// [`ShoveRequested`](gdtf_battle_sim::acts::ShoveRequested) /
/// [`OpenDoorRequested`](gdtf_battle_sim::acts::OpenDoorRequested) /
/// [`EnterEmplacementRequested`](gdtf_battle_sim::acts::EnterEmplacementRequested) /
/// [`ExitEmplacementRequested`](gdtf_battle_sim::acts::ExitEmplacementRequested) for the
/// [`SelectedShooter`](gdtf_battle_input::SelectedShooter) as the actor (a no-op there with no
/// selection), and the sim's faction + reach (+ LOS for melee; + CLOSED-state for the door;
/// + VACANT-state for enter / occupant-is-actor for exit) gates are the authoritative check.
///
/// Registered `.before(dispatch_act_intents)` so a press queued this update is drained this
/// update — the same-frame guarantee the keyboard writers + the action bar get (`bevy-traps.md`
/// #3). Param-only (`bevy-traps.md` #7): the read-only [`ContextualTargets`] seam, the
/// [`ContextualButtonPresses`] bundle of seven disjoint per-marker `Query<&Interaction, …>`s, and
/// the `ResMut<PendingActIntent>` write. (The [`ContextualButtonPresses`] bundle now holds eight
/// disjoint per-marker `Query<&Interaction, …>`s — the GTW-546 Throw button included.)
pub(in crate::states::running::game::battlescape) fn contextual_button_intents(
    targets: Res<ContextualTargets>,
    mut pending: ResMut<PendingActIntent>,
    presses: ContextualButtonPresses,
) {
    if ContextualButtonPresses::pressed(&presses.execute)
        && let Some(target) = targets.execute()
    {
        pending.push(ActIntent::Execute(target));
    }
    if ContextualButtonPresses::pressed(&presses.stabilize)
        && let Some(target) = targets.stabilize()
    {
        pending.push(ActIntent::Stabilize(target));
    }
    // The dedicated Melee button routes to a GANGER strike (GTW-507) or, when no meleeable
    // ganger is offered, an adjacent-structure SMASH (GTW-508) — the ONE button, two target
    // kinds. The ganger target takes priority (checked first); the `Some`-guards keep a stale
    // press safe.
    if ContextualButtonPresses::pressed(&presses.melee) {
        if let Some(target) = targets.melee() {
            pending.push(ActIntent::Melee(target));
        } else if let Some(at) = targets.melee_structure() {
            pending.push(ActIntent::MeleeStructure(at));
        }
    }
    // The dedicated Shove button (GTW-525) — a deliberate, universal knock-back on an 8-adjacent
    // alive opposing ganger. The `Some`-guard keeps a stale press safe.
    if ContextualButtonPresses::pressed(&presses.shove)
        && let Some(target) = targets.shove()
    {
        pending.push(ActIntent::Shove(target));
    }
    // The dedicated Open Door button (GTW-315) — opens an 8-adjacent CLOSED door. The `Some`-guard
    // keeps a stale press safe (the sim's `dispatch_open_door` re-gate + TU spend are authoritative).
    if ContextualButtonPresses::pressed(&presses.open_door)
        && let Some(door) = targets.open_door()
    {
        pending.push(ActIntent::OpenDoor(door));
    }
    // The dedicated Enter button (GTW-543) — mans an 8-adjacent VACANT emplacement. The `Some`-guard
    // keeps a stale press safe (the sim's `dispatch_enter_emplacement` re-gate + TU spend are
    // authoritative).
    if ContextualButtonPresses::pressed(&presses.enter_emplacement)
        && let Some(emplacement) = targets.enter_emplacement()
    {
        pending.push(ActIntent::EnterEmplacement(emplacement));
    }
    // The dedicated Exit button (GTW-543) — dismounts the emplacement the selection is manning
    // (offered ONLY to the occupant; NO force-eject). The `Some`-guard keeps a stale press safe
    // (the sim's `dispatch_exit_emplacement` re-gate + TU spend are authoritative).
    if ContextualButtonPresses::pressed(&presses.exit_emplacement)
        && let Some(emplacement) = targets.exit_emplacement()
    {
        pending.push(ActIntent::ExitEmplacement(emplacement));
    }
    // The dedicated Throw button (GTW-546) — lobs a grenade at the hovered target cell (a BLIND
    // lob, offered when the selection wields an `Arc` weapon). The `Some`-guard keeps a stale press
    // safe (the sim's `dispatch_throw_grenade` re-gate + TU / magazine spend are authoritative).
    if ContextualButtonPresses::pressed(&presses.throw_grenade)
        && let Some(at) = targets.throw_grenade()
    {
        pending.push(ActIntent::ThrowGrenade(at));
    }
}
