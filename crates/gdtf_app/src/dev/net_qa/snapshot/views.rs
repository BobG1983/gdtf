//! The bundle of read-only enumeration surfaces the router's `GetAppFlow` answer folds in
//! (GTW-802).
//!
//! Two independent handouts ride the app-flow snapshot — the [`menu`](super::menu)
//! enumeration and the [`focus`](super::focus) enumeration — and each needs its own world
//! surface. Nesting them in one `SystemParam` keeps
//! [`route_requests`](super::super::router::route_requests) under the clippy
//! argument-count ceiling and gives a third handout somewhere obvious to land.

use bevy::ecs::system::SystemParam;
use gdtf_qa_protocol::view::{FocusView, MenuView};

use super::{focus::FocusReadWorld, menu::MenuReadWorld};

/// The read-only world surfaces the `GetAppFlow` answer projects its folded enumeration
/// handouts from. Every borrow inside is SHARED, so the bundle is `B0001`-safe.
#[derive(SystemParam)]
pub(in crate::dev::net_qa) struct AppFlowViews<'w, 's> {
    /// The generic menu-enumeration surface (GTW-787).
    menu:  MenuReadWorld<'w, 's>,
    /// The generic focus-navigable control surface (GTW-802).
    focus: FocusReadWorld<'w, 's>,
}

impl AppFlowViews<'_, '_> {
    /// The current menu handout, or `None` off an enumerable menu.
    pub(in crate::dev::net_qa) fn menu(&self) -> Option<MenuView> {
        super::menu::menu_view(&self.menu)
    }

    /// The current screen's focusable-control handout, or `None` on a screen with no focus
    /// graph.
    pub(in crate::dev::net_qa) fn focus(&self) -> Option<FocusView> {
        super::focus::focus_view(&self.focus)
    }
}
