//! The GENERIC (act-agnostic) contextual-button systems (GTW-571 C2) — one spawn / one
//! visibility toggle / one press router stamped per act by the
//! [`registrar`](super::super::registrar), plus the two act-count-independent panel
//! passes (child ordering + root visibility).
//!
//! This file is what dissolved the per-marker lattice: the old nine `*VisFilter`
//! aliases, the `PanelVisibility` bundle, the eight per-marker press queries, and the
//! per-marker spawn stanzas are GONE. Each generic system holds exactly ONE
//! `Visibility` / `Interaction` query for its own act's marker, so NO per-act
//! disjointness `Without<>` filters exist anywhere — two `&mut Visibility` queries in
//! different systems merely serialize, they never alias (the N-squared filter wall was
//! an artifact of one system holding nine queries). Visibility stays mutate-in-place
//! (never despawn/respawn — the `ui-mutate-not-respawn` ruling).

use bevy::{prelude::*, ui::Interaction};
use gdtf_battle_input::contextual::PendingContextualIntents;
use gdtf_ui::{DisabledButton, spawn_button, theme::GdtfTheme};

use crate::states::running::game::battlescape::contextual_panel::{
    components::ContextualPanelRoot,
    seam::{ContextualActButton, ContextualOffer, ContextualPanelAct},
};

/// Query filter selecting the ENABLED button carrying marker `M` whose [`Interaction`]
/// changed this frame — the act-agnostic press filter (the action-bar `PressedButton`
/// mechanism, stamped per act by [`press_contextual_button`]).
///
/// A named alias to keep the generic press system's signature legible (clippy
/// `type_complexity`). `Without<DisabledButton>` skips a disabled button (none of the
/// contextual buttons disable today, but the filter stays faithful to the press
/// mechanism), and `Changed<Interaction>` limits the query to the frame a press lands.
type PressedButton<M> = (Changed<Interaction>, With<M>, Without<DisabledButton>);

/// Whether an [`Interaction`] is a fresh press to act on — only [`Interaction::Pressed`].
///
/// The action-bar `is_press` precedent (its copy is `pub(in …action_bar)`, so this
/// sibling panel mirrors the one-liner rather than reaching across the action-bar
/// subtree). Takes [`Interaction`] by value (a one-byte `Copy` enum).
const fn is_press(interaction: Interaction) -> bool {
    matches!(interaction, Interaction::Pressed)
}

/// Spawns act `A`'s themed contextual button under the panel root — the ONE generic
/// spawn stamped per act (GTW-571; replaces the per-marker spawn stanzas).
///
/// Runs `OnEnter(BattleScapeState::BattleRunning)` in the
/// [`ContextualPanelSpawnSystems::Buttons`](super::super::registrar::ContextualPanelSpawnSystems)
/// set — AFTER the root box spawned (the explicit set chain forces the `Commands` sync
/// point, so the root query resolves) and BEFORE the deterministic
/// [`order_contextual_buttons`] pass. The button carries `A::Marker` (the per-act press
/// / toggle key), [`ContextualActButton`] with `A::SLOT` (the act-agnostic ordering +
/// root-aggregation tag), and spawns
/// [`Visibility::Hidden`](bevy::camera::visibility::Visibility) — the act's offer
/// toggle reveals it in place. No theme / no root (a pre-theme frame) spawns nothing —
/// fail-closed (`bevy-traps.md` #1).
pub(in crate::states::running::game::battlescape) fn spawn_contextual_button<
    A: ContextualPanelAct,
>(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    roots: Query<Entity, With<ContextualPanelRoot>>,
) {
    let Some(theme) = theme else {
        return;
    };
    let Ok(root) = roots.single() else {
        return;
    };
    let button = spawn_button(
        &mut commands,
        &theme,
        A::label(),
        (
            A::Marker::default(),
            ContextualActButton::new(A::SLOT),
            Visibility::Hidden,
        ),
    );
    commands.entity(root).add_child(button);
}

/// Re-parents the panel's buttons in [`PanelSlot`](super::super::seam::PanelSlot) order
/// — the act-count-independent ordering pass (GTW-571).
///
/// The per-act [`spawn_contextual_button`] systems run in nondeterministic order inside
/// their set, so raw child order would shuffle between runs; this pass (in the
/// [`ContextualPanelSpawnSystems::Order`](super::super::registrar::ContextualPanelSpawnSystems)
/// set, AFTER every button spawn) sorts every [`ContextualActButton`] by its carried
/// slot (entity id as the tie-break — slots are unique by the recipe's contract) and
/// `replace_children`s the root once. Runs once per battle entry; the buttons keep
/// their identity (a re-parent, never a respawn).
pub(in crate::states::running::game::battlescape) fn order_contextual_buttons(
    mut commands: Commands,
    buttons: Query<(Entity, &ContextualActButton)>,
    roots: Query<Entity, With<ContextualPanelRoot>>,
) {
    let Ok(root) = roots.single() else {
        return;
    };
    let mut ordered: Vec<(Entity, ContextualActButton)> = buttons
        .iter()
        .map(|(entity, slot)| (entity, *slot))
        .collect();
    ordered.sort_by_key(|(entity, slot)| (**slot, *entity));
    let children: Vec<Entity> = ordered.into_iter().map(|(entity, _)| entity).collect();
    commands.entity(root).replace_children(&children);
}

/// Toggles act `A`'s button [`Visibility`] to match its current offer — the ONE generic
/// toggle stamped per act (GTW-571; replaces the `PanelVisibility` bundle + the
/// per-marker `*VisFilter` wall).
///
/// Reads [`ContextualOffer<A>`] and sets the `A::Marker` button `Visible` iff a target
/// is offered, else `Hidden` — IN PLACE, never despawning (`ui-mutate-not-respawn`),
/// writing only on a real change (change-detection hygiene). Runs in the
/// [`ContextualPanelSystems::Toggle`](super::super::registrar::ContextualPanelSystems)
/// set, after the act's offer scan. ONE `&mut Visibility` query per system — no
/// disjointness filters needed (see the module doc).
pub(in crate::states::running::game::battlescape) fn sync_contextual_button_visibility<
    A: ContextualPanelAct,
>(
    offer: Res<ContextualOffer<A>>,
    mut buttons: Query<&mut Visibility, With<A::Marker>>,
) {
    set_visibility(&mut buttons, offer.is_offered());
}

/// Routes act `A`'s button press to its buffered per-act intent queue — the ONE generic
/// press router stamped per act (GTW-571; replaces the `ContextualButtonPresses` bundle
/// + the per-marker route arms).
///
/// On a fresh [`Interaction::Pressed`] with a target offered, pushes the target onto
/// [`PendingContextualIntents<A>`] — the act's single write-point; the input layer's
/// generic drain emits `A::request(actor, target)` the SAME update (the press set is
/// ordered `.before` the drain set — the Q5 same-frame guarantee). The `Some`-guard
/// keeps a stale press (the panel just hid) safe: with no offer nothing is queued. The
/// button writes NO `*Requested` directly (P8 — dispatch is sim-side; this layer only
/// buffers intent).
pub(in crate::states::running::game::battlescape) fn press_contextual_button<
    A: ContextualPanelAct,
>(
    offer: Res<ContextualOffer<A>>,
    presses: Query<&Interaction, PressedButton<A::Marker>>,
    mut pending: ResMut<PendingContextualIntents<A>>,
) {
    if presses.iter().copied().any(is_press)
        && let Some(target) = offer.target()
    {
        pending.push(target);
    }
}

/// Shows the panel ROOT iff ANY contextual-act button is visible — the
/// act-count-independent root aggregation (GTW-571; replaces the old detect brain's
/// nine-way `||`).
///
/// Reads every [`ContextualActButton`]'s [`Visibility`] (written by the per-act toggles
/// — ordered `.after(ContextualPanelSystems::Toggle)`) and mutates the root's in place.
/// The two queries are disjoint by ONE static filter pair (`With<ContextualActButton>`
/// vs `Without<ContextualActButton>`) — act-count-independent, not a per-act wall.
pub(in crate::states::running::game::battlescape) fn sync_panel_root_visibility(
    buttons: Query<&Visibility, With<ContextualActButton>>,
    mut roots: Query<&mut Visibility, (With<ContextualPanelRoot>, Without<ContextualActButton>)>,
) {
    let any_offered = buttons
        .iter()
        .any(|visibility| *visibility == Visibility::Visible);
    set_visibility(&mut roots, any_offered);
}

/// Sets every matched [`Visibility`] to `Visible` (when `show`) or `Hidden`, in place.
///
/// The shared toggle helper (the pre-GTW-571 detect brain's helper, kept verbatim):
/// mutates the existing component — it never spawns or despawns (the
/// `ui-mutate-not-respawn` ruling) — and only writes on a real change
/// (change-detection hygiene). A query that matches no node (the panel not yet
/// spawned) is a silent no-op.
fn set_visibility<F: bevy::ecs::query::QueryFilter>(
    query: &mut Query<&mut Visibility, F>,
    show: bool,
) {
    let want = if show {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut visibility in query {
        if *visibility != want {
            *visibility = want;
        }
    }
}
