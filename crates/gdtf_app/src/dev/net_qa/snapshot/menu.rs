//! The generic menu-enumeration handout (GTW-787).
//!
//! [`menu_view`] reads the game-side menu model — the [`MenuScreen`] identity marker on a
//! scene's menu root and the [`MenuItem`] markers on its buttons — and projects it into
//! the bevy-free wire [`MenuView`] that rides in
//! [`AppFlowView::menu`](gdtf_qa_protocol::view::AppFlowView::menu). It is scene-agnostic:
//! any scene that tags its menu with those two markers is enumerated with no change here,
//! so a new menu gets QA nav for free.
//!
//! Each item's token is its `Entity` minted into a [`FocusTargetNet`] (the token
//! [`ActivateMenuItem`](gdtf_qa_protocol::envelope::QaRequest::ActivateMenuItem) echoes
//! back), its label is read off the caption `Text` CHILD (exactly as the GTW-789 panel
//! button label read), and its enabled flag is the ABSENCE of
//! [`DisabledButton`](gdtf_ui::DisabledButton) — a disabled button is still listed, marked
//! `enabled == false`.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_qa_protocol::{
    ids::FocusTargetNet,
    view::{MenuIdNet, MenuItemEnabledNet, MenuItemLabelNet, MenuItemView, MenuView},
};
use gdtf_ui::{DisabledButton, MenuItem, MenuScreen};

/// One menu item's queried data — the entity (minted into a focus token), its child nodes
/// (the caption `Text` the label reads through), and whether it is disabled.
///
/// Factored into a named alias to keep [`MenuReadWorld`]'s `items` field legible (clippy
/// `type_complexity`).
type MenuItemRow = (Entity, Option<&'static Children>, Has<DisabledButton>);

/// The read-only world surface [`menu_view`] projects the wire [`MenuView`] from.
///
/// A `SystemParam` so a consumer (the router's `GetAppFlow` answer) declares one param.
/// Every borrow is SHARED — no field aliases another (`B0001`-safe) — and every query is
/// empty-safe: on a scene with no menu model the `screens` query is empty and
/// [`menu_view`] returns `None`, so this reads harmlessly in any state (including a live
/// battle, where the menu is despawned).
#[derive(SystemParam)]
pub(in crate::dev::net_qa) struct MenuReadWorld<'w, 's> {
    /// The active menu's identity marker(s) — at most one live in a well-formed scene; the
    /// first is the menu identity, and its presence is what makes the wire menu `Some`.
    pub(super) screens:    Query<'w, 's, &'static MenuScreen>,
    /// Every enumerable menu item — its entity (minted into a focus token), its child
    /// nodes (the caption `Text` the label reads through), and whether it is disabled.
    pub(super) items:      Query<'w, 's, MenuItemRow, With<MenuItem>>,
    /// The caption text of any UI node — the item label read keys a button's child entity
    /// through it (`spawn_button` puts the caption on a `Text` child, not the button root).
    pub(super) item_texts: Query<'w, 's, &'static Text>,
}

/// Project the current menu into its wire [`MenuView`], or `None` when the app is not on
/// an enumerable menu (no live [`MenuScreen`]).
///
/// Items are sorted by their token bits — a stable, deterministic wire order that reflects
/// the buttons' spawn/declaration order (menu buttons are spawned top-to-bottom, so earlier
/// buttons carry lower entity indices).
pub(in crate::dev::net_qa) fn menu_view(world: &MenuReadWorld) -> Option<MenuView> {
    let screen = world.screens.iter().next()?;
    let mut items: Vec<MenuItemView> = world
        .items
        .iter()
        .map(|(entity, children, disabled)| {
            MenuItemView::new(
                FocusTargetNet::new(entity.to_bits()),
                item_label(world, children),
                MenuItemEnabledNet::new(!disabled),
            )
        })
        .collect();
    items.sort_by_key(|item| *item.token);
    Some(MenuView::new(
        MenuIdNet::new(screen.name().to_string()),
        items,
    ))
}

/// Read a menu button's on-screen caption into a [`MenuItemLabelNet`] — the first child
/// entity that carries a [`Text`] node ([`spawn_button`](gdtf_ui::spawn_button) spawns
/// exactly one caption child). A button with no readable caption child yields an empty
/// label rather than being dropped (fail-open — the token + enabled flag are still useful).
fn item_label(world: &MenuReadWorld, children: Option<&Children>) -> MenuItemLabelNet {
    let caption = children
        .and_then(|children| {
            children
                .iter()
                .find_map(|child| world.item_texts.get(child).ok())
        })
        .map(|text| text.0.clone())
        .unwrap_or_default();
    MenuItemLabelNet::new(caption)
}
