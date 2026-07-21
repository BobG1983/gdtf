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
    // Spawn the button COLLAPSED (`Display::None`) as well as `Visibility::Hidden`, so a
    // not-yet-offered button reserves NO layout row — the panel column sizes to only its
    // offered buttons and never grows the box up over the map (GTW-726). The per-act
    // toggle flips both fields in place when the act becomes offered.
    commands
        .entity(button)
        .entry::<Node>()
        .and_modify(|mut node| node.display = Display::None);
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
/// Reads [`ContextualOffer<A>`] and shows the `A::Marker` button iff a target is offered,
/// else hides it — IN PLACE, never despawning (`ui-mutate-not-respawn`), writing only on a
/// real change (change-detection hygiene). Runs in the
/// [`ContextualPanelSystems::Toggle`](super::super::registrar::ContextualPanelSystems)
/// set, after the act's offer scan. ONE query per system — no disjointness filters needed
/// (see the module doc).
///
/// "Show" drives BOTH the button's [`Visibility`] (the offer signal the AC tests /
/// [`sync_panel_root_visibility`] / the `net_qa` inject gate read) AND its
/// [`Node::display`] (the layout collapse) — see [`set_button_shown`].
pub(in crate::states::running::game::battlescape) fn sync_contextual_button_visibility<
    A: ContextualPanelAct,
>(
    offer: Res<ContextualOffer<A>>,
    mut buttons: Query<(&mut Visibility, &mut Node), With<A::Marker>>,
) {
    set_button_shown(&mut buttons, offer.is_offered());
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
/// spawned) is a silent no-op. Used for the panel ROOT, which is absolute-positioned and
/// so contributes nothing to the bar's layout — its display never needs collapsing, only
/// its rendered visibility.
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

/// Shows/hides every matched contextual BUTTON in place — sets BOTH its [`Visibility`]
/// (the offer signal the AC tests / [`sync_panel_root_visibility`] / the `net_qa` inject
/// gate read) AND its [`Node::display`]: a hidden button is [`Display::None`] so it
/// COLLAPSES its layout row, a shown one is [`Display::Flex`].
///
/// Collapsing the hidden buttons is the GTW-726 fix: a `Visibility::Hidden` node still
/// RESERVES its layout box, so the eight-button column stayed eight rows tall no matter
/// how few acts were offered — taller than the bottom bar, which pushed the panel up over
/// the map. With the hidden rows collapsed the column sizes to only its offered buttons,
/// so the box stays within the bar. A mutate-in-place toggle — it never despawns/respawns
/// (`ui-mutate-not-respawn`) — writing each field only on a real change (change-detection
/// hygiene). A query that matches no node is a silent no-op.
fn set_button_shown<F: bevy::ecs::query::QueryFilter>(
    query: &mut Query<(&mut Visibility, &mut Node), F>,
    show: bool,
) {
    let (want_visibility, want_display) = if show {
        (Visibility::Visible, Display::Flex)
    } else {
        (Visibility::Hidden, Display::None)
    };
    for (mut visibility, mut node) in query {
        if *visibility != want_visibility {
            *visibility = want_visibility;
        }
        if node.display != want_display {
            node.display = want_display;
        }
    }
}
