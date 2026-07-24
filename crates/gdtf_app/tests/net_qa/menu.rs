//! GTW-787 — the generic menu enumeration + activation path, driven through the REAL
//! router (`GetAppFlow` folds in the menu) and the REAL `drive_activate_menu_item`
//! consumer on a headless app resting at the main menu (never a shadow copy).
//!
//! Three facts are pinned:
//!
//! - enumerating the main menu over `GetAppFlow` returns the curated item set that is
//!   actually spawned/wired — the four buttons, their labels, and their enabled flags
//!   (the disabled `HiveScape` listed but `enabled == false`);
//! - activating the `Battlescape` item's token reaches the resulting state through the
//!   game's REAL focus-activation path (the same `FocusActivated` an `Enter` keypress
//!   raises → the menu's shared `StartBattleRequested` → `apply_start_battle`'s
//!   `Menu → Game` transition), not a bespoke `NextState` write;
//! - a stale / unlisted token is answered a typed
//!   `MenuActivationReceipt::Rejected(RejectReason::StaleToken)` — never a panic, never a
//!   silent no-op.

use std::sync::mpsc;

use bevy::{app::App, state::state::State};
use gdtf_app::test_support::{IncomingRequest, RunningState};
use gdtf_qa_protocol::{
    envelope::{MenuActivationReceipt, QaRequest, QaResponse, RejectReason},
    ids::FocusTargetNet,
    view::{MenuItemView, MenuView},
};
use gdtf_test_utils::advance_until;

use crate::{inject_support::send, start_battle::menu_app_with_net_qa};

/// The frame budget for the menu → game descent an activation kicks off.
const DRIVE_BUDGET: u32 = 128;

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Send `GetAppFlow`, drive one frame, and read the folded menu view off the reply.
fn read_menu(app: &mut App, tx: &mpsc::Sender<IncomingRequest>) -> Option<MenuView> {
    let reply = send(tx, QaRequest::GetAppFlow);
    app.update();
    let Ok(QaResponse::AppFlow(view)) = reply.try_recv() else {
        unreachable!("GetAppFlow must answer with an AppFlow snapshot");
    };
    view.menu
}

/// Find the first enumerated item whose label matches `label`.
fn item_by_label<'a>(menu: &'a MenuView, label: &str) -> Option<&'a MenuItemView> {
    menu.items.iter().find(|item| item.label.as_str() == label)
}

/// Enumerating the main menu over `GetAppFlow` returns the curated item set that is
/// actually spawned/wired: the four buttons with their labels, the disabled `HiveScape`
/// listed but `enabled == false`, under the `MainMenu` identity.
///
/// Pin: this reads the LIVE spawned menu (the real `spawn_menu` scene tagged with the
/// `MenuScreen` / `MenuItem` model), not a fixture echo — dropping the `MenuItem` marker
/// from a button, or the `MenuScreen` from the root, reddens it.
#[test]
fn enumerates_the_main_menu_items() {
    let (mut app, tx) = menu_app_with_net_qa();
    let Some(menu) = read_menu(&mut app, &tx) else {
        unreachable!("the main menu must enumerate a MenuView");
    };

    assert_eq!(
        menu.id.as_str(),
        "MainMenu",
        "the menu identity is the main menu"
    );
    assert_eq!(menu.items.len(), 4, "all four menu buttons are enumerated");

    let labels: Vec<&str> = menu.items.iter().map(|item| item.label.as_str()).collect();
    for expected in ["Battlescape", "Options", "HiveScape", "Quit"] {
        assert!(
            labels.contains(&expected),
            "the menu enumeration lists {expected:?}; got {labels:?}",
        );
    }

    // The enabled buttons are activatable; the disabled `HiveScape` placeholder is listed
    // but marked `enabled == false`.
    for enabled_label in ["Battlescape", "Options", "Quit"] {
        let Some(item) = item_by_label(&menu, enabled_label) else {
            unreachable!("{enabled_label} is enumerated");
        };
        assert!(*item.enabled, "{enabled_label} is enabled");
    }
    let Some(hivescape) = item_by_label(&menu, "HiveScape") else {
        unreachable!("HiveScape is enumerated");
    };
    assert!(
        !*hivescape.enabled,
        "the disabled HiveScape placeholder is listed but not enabled",
    );
}

/// Activating the enumerated `Battlescape` token reaches the resulting state through the
/// game's REAL activation path — the app leaves `RunningState::Menu` (the shared
/// `StartBattleRequested` → `apply_start_battle` `Menu → Game` transition the local button
/// press also drives), driven only by the token a `MenuView` handed out.
///
/// Pin: the app rests at `Menu` and NEVER leaves on its own; only the activation triggers
/// the transition. Dropping the `FocusActivated` write in `drive_activate_menu_item` (or
/// its `MenuItem` resolution) leaves the app parked at `Menu` and reddens the
/// `advance_until` assertion.
#[test]
fn activating_battlescape_leaves_the_menu() {
    let (mut app, tx) = menu_app_with_net_qa();
    let Some(menu) = read_menu(&mut app, &tx) else {
        unreachable!("the main menu must enumerate a MenuView");
    };
    let Some(battlescape) = item_by_label(&menu, "Battlescape") else {
        unreachable!("Battlescape is enumerated");
    };

    // Baseline: the app rests at the menu until something acts.
    assert_eq!(
        running_state(&app),
        Some(RunningState::Menu),
        "the fixture rests at the menu before activation",
    );

    let reply = send(&tx, QaRequest::ActivateMenuItem(battlescape.token));
    app.update();
    let Ok(QaResponse::MenuItemActivated(MenuActivationReceipt::Activated)) = reply.try_recv()
    else {
        unreachable!("a live, listed menu item's activation is Activated");
    };

    // The activation drove the real Menu-leaving transition (the shared start-battle path).
    let left_menu = advance_until(
        &mut app,
        |app| running_state(app).is_some_and(|state| state != RunningState::Menu),
        DRIVE_BUDGET,
    );
    assert!(
        left_menu,
        "activating Battlescape must leave the menu via the real focus-activation path; \
         stuck at {:?}",
        running_state(&app),
    );
}

/// A stale / unlisted token is answered a typed
/// `MenuActivationReceipt::Rejected(RejectReason::StaleToken)` — never a panic, never a
/// silent no-op — and the app stays at the menu.
#[test]
fn stale_token_is_rejected() {
    let (mut app, tx) = menu_app_with_net_qa();

    // A token that names no live entity at all — the fail-closed stale-token case.
    let reply = send(&tx, QaRequest::ActivateMenuItem(FocusTargetNet::new(0)));
    app.update();
    let Ok(QaResponse::MenuItemActivated(receipt)) = reply.try_recv() else {
        unreachable!("ActivateMenuItem answers a MenuItemActivated receipt");
    };
    assert_eq!(
        receipt,
        MenuActivationReceipt::Rejected(RejectReason::StaleToken),
        "an unlisted token is rejected StaleToken",
    );
    assert_eq!(
        running_state(&app),
        Some(RunningState::Menu),
        "a rejected activation leaves the app at the menu",
    );
}
