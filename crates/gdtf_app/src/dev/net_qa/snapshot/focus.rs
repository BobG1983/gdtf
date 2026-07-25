//! The generic focus-navigable control enumeration handout (GTW-802).
//!
//! [`focus_view`] reads the game's OWN focus model — the
//! [`DirectionalNavigationMap`] graph a screen populates to be keyboard-navigable at all,
//! plus the [`InputFocus`] resource that says where focus currently sits — and projects it
//! into the bevy-free wire [`FocusView`] that rides in
//! [`AppFlowView::focus`](gdtf_qa_protocol::view::AppFlowView::focus).
//!
//! ## Why the navigation map, and not a marker
//!
//! The sibling [`menu`](super::menu) handout enumerates entities a scene TAGGED with the
//! menu markers, so a screen has to opt in. This one reads the graph the screen already
//! declares by calling `add_edges` — the Options screen, the main menu, and the
//! battlescape HUD panels all do — so "focusable" means exactly what it means to the
//! player, and the wire list cannot drift from what the arrow keys can actually reach. A
//! screen that adds a control to its navigation chain is enumerated here with no change
//! anywhere in the QA stack, and no screen is asked to adopt a second tagging scheme.
//!
//! Each row's token is its `Entity` minted into a
//! [`FocusTargetNet`] (the token
//! [`FocusControl`](gdtf_qa_protocol::envelope::QaRequest::FocusControl) echoes back), its
//! label is read off the caption `Text` CHILD (exactly as the menu / panel button label
//! reads — a control with no caption child, such as the Options sound toggle, lists an
//! empty label and is identified by its kind instead), its kind comes from the components
//! the entity already carries, and its enabled flag is the ABSENCE of a disabled marker.
//!
//! The navigation map is a GLOBAL resource, not state-scoped, so a key can outlive the
//! screen that added it; a key that no longer names a live entity is SKIPPED rather than
//! listed as a token the consumer would then reject.

use bevy::{
    ecs::system::SystemParam,
    input_focus::{InputFocus, directional_navigation::DirectionalNavigationMap},
    prelude::*,
    ui::{Checked, InteractionDisabled},
    ui_widgets::Checkbox,
};
use gdtf_qa_protocol::{
    ids::FocusTargetNet,
    view::{
        FocusView, FocusableCheckedNet, FocusableEnabledNet, FocusableKindNet, FocusableLabelNet,
        FocusableView, FocusedNet,
    },
};
use gdtf_ui::DisabledButton;

/// One focusable control's queried data — its child nodes (the caption `Text` the label
/// reads through), whether it is a first-party checkbox and currently checked, whether it
/// is a `bevy_ui` button, and the two disabled markers (the first-party
/// [`InteractionDisabled`] and the project's own [`DisabledButton`]).
///
/// Factored into a named alias to keep [`FocusReadWorld`]'s `rows` field legible (clippy
/// `type_complexity`).
type FocusableRow = (
    Option<&'static Children>,
    Has<Checkbox>,
    Has<Checked>,
    Has<Button>,
    Has<InteractionDisabled>,
    Has<DisabledButton>,
);

/// The read-only world surface [`focus_view`] projects the wire [`FocusView`] from.
///
/// A `SystemParam` so a consumer (the router's `GetAppFlow` answer) declares one param.
/// Every borrow is SHARED — no field aliases another (`B0001`-safe) — and every source is
/// empty-safe: the two resources are `Option<Res<…>>` (a `MinimalPlugins` app without the
/// focus framework simply reports no focus view, `bevy-traps.md` #1) and both queries are
/// empty-safe, so this reads harmlessly in any state.
#[derive(SystemParam)]
pub(in crate::dev::net_qa) struct FocusReadWorld<'w, 's> {
    /// The game's focus graph — the SET of focusable entities this handout enumerates.
    pub(super) nav_map:   Option<Res<'w, DirectionalNavigationMap>>,
    /// Where input focus currently sits.
    pub(super) focus:     Option<Res<'w, InputFocus>>,
    /// The per-control components a row's kind / enabled / checked flags read.
    pub(super) rows:      Query<'w, 's, FocusableRow>,
    /// The caption text of any UI node — a control's label read keys its child entity
    /// through it (the caption lives on a `Text` child, not the control root).
    pub(super) row_texts: Query<'w, 's, &'static Text>,
}

/// Project the current screen's focus-navigable controls into their wire [`FocusView`], or
/// `None` when the app is on a screen with no focus graph at all (no navigation map, or one
/// whose keys are all dead).
///
/// Rows are sorted by their token bits — a stable, deterministic wire order that reflects
/// the controls' spawn/declaration order (a screen spawns its controls top-to-bottom, so
/// earlier controls carry lower entity indices), matching the menu handout.
pub(in crate::dev::net_qa) fn focus_view(world: &FocusReadWorld) -> Option<FocusView> {
    let nav_map = world.nav_map.as_ref()?;
    let focused = world.focus.as_ref().and_then(|focus| focus.get());
    let mut focusables: Vec<FocusableView> = nav_map
        .neighbors
        .keys()
        .filter_map(|entity| focusable_row(world, *entity, focused))
        .collect();
    if focusables.is_empty() {
        return None;
    }
    focusables.sort_by_key(|focusable| *focusable.token);
    Some(FocusView::new(
        focused.map(|entity| FocusTargetNet::new(entity.to_bits())),
        focusables,
    ))
}

/// Project ONE navigation-map key into its wire row, or [`None`] when the key no longer
/// names a live entity (the map is global, so a key can outlive its screen).
fn focusable_row(
    world: &FocusReadWorld,
    entity: Entity,
    focused: Option<Entity>,
) -> Option<FocusableView> {
    let (children, is_checkbox, is_checked, is_button, interaction_disabled, disabled_button) =
        world.rows.get(entity).ok()?;
    let kind = if is_checkbox {
        FocusableKindNet::Checkbox
    } else if is_button {
        FocusableKindNet::Button
    } else {
        FocusableKindNet::Other
    };
    // A checkbox reports its value so an agent can confirm a toggle landed from a follow-up
    // `GetAppFlow` alone; every other kind has no value to read.
    let checked = is_checkbox.then(|| FocusableCheckedNet::new(is_checked));
    Some(FocusableView::new(
        FocusTargetNet::new(entity.to_bits()),
        row_label(world, children),
        kind,
        FocusableEnabledNet::new(!interaction_disabled && !disabled_button),
        checked,
        FocusedNet::new(focused == Some(entity)),
    ))
}

/// Read a control's on-screen caption into a [`FocusableLabelNet`] — the first child entity
/// that carries a [`Text`] node ([`spawn_button`](gdtf_ui::spawn_button) spawns exactly one
/// caption child). A control with no readable caption child (the Options sound toggle, whose
/// only child is its knob) yields an empty label rather than being dropped — the token, kind,
/// and checked value still name it.
fn row_label(world: &FocusReadWorld, children: Option<&Children>) -> FocusableLabelNet {
    let caption = children
        .and_then(|children| {
            children
                .iter()
                .find_map(|child| world.row_texts.get(child).ok())
        })
        .map(|text| text.0.clone())
        .unwrap_or_default();
    FocusableLabelNet::new(caption)
}
