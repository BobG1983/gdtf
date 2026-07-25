//! GTW-802 — the END-TO-END activation half, driven through the REAL router and the REAL
//! `drive_focus_control` consumer on a `DefaultPlugins` app resting on the Options screen.
//!
//! This is the suite the ticket's second acceptance clause names: an agent toggles the
//! Options sound checkbox and activates its Continue button over the wire. It must run on
//! the `DefaultPlugins` tier — under `MinimalPlugins` there is no `InputPlugin`
//! (`keyboard_input_system`), no `InputDispatchPlugin` (`dispatch_focused_input`), and no
//! `CheckboxPlugin` (`checkbox_on_key_input`), so the keypress → activation half cannot
//! exist there at all.
//!
//! Nothing here forges an activation. The consumer emits a REAL `Enter` keypress at the
//! focused control, and the assertions observe what the game's OWN systems then do with it:
//!
//! - the checkbox path is Bevy's `dispatch_focused_input` → `checkbox_on_key_input` →
//!   `ValueChange<bool>` → the screen's own `sound_activated` → `SoundSettingChanged` →
//!   `apply_sound_setting` → `sync_sound_value_label`, observed as the readout flipping
//!   "On" → "Off" and the first-party `checkbox_self_update` clearing `Checked`;
//! - the Continue path is `keyboard_input_system` → `gdtf_ui`'s own
//!   `bridge_keyboard_navigation` raising the very `FocusActivated` a player's `Enter`
//!   raises → the screen's `bridge_continue_activation` → `continue_activated`, observed as
//!   the screen leaving for the Main Menu.
//!
//! The effect lands a frame after the receipt (the emitted messages are consumed in the next
//! `PreUpdate`), so each assertion polls a small budget — the documented
//! dispatched-not-completed contract of `FocusControlReceipt::Applied`.

use bevy::{app::App, ecs::entity::Entity, ui::Checked};
use gdtf_app::test_support::{ContinueButton, RunningState, SoundToggle};
use gdtf_net_qa_transport::IncomingRequest;
use gdtf_qa_protocol::{
    envelope::{FocusCommandNet, FocusControlReceipt, QaRequest, QaResponse},
    ids::FocusTargetNet,
};
use gdtf_test_utils::advance_until;

use crate::{
    focus_support::{read_focus, running_state, sound_value_text, ui_options_app_with_net_qa},
    inject_support::send,
};

/// How many updates to allow for an emitted keypress to be consumed and its effect to
/// settle through the screen's own observers.
const SETTLE_BUDGET: u32 = 16;

/// Looks up the single entity carrying marker `M`, if exactly one exists.
fn single_with<M: bevy::ecs::component::Component>(app: &mut App) -> Option<Entity> {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, bevy::ecs::prelude::With<M>>();
    let found: Vec<Entity> = query.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// Send an activate-by-token command, drive a frame, and read the typed receipt.
fn activate_target(
    app: &mut App,
    tx: &std::sync::mpsc::Sender<IncomingRequest>,
    token: FocusTargetNet,
) -> FocusControlReceipt {
    let reply = send(
        tx,
        QaRequest::FocusControl(FocusCommandNet::ActivateTarget(token)),
    );
    app.update();
    let Ok(QaResponse::FocusControlled(receipt)) = reply.try_recv() else {
        unreachable!("FocusControl must answer with a FocusControlled receipt");
    };
    receipt
}

/// Acceptance clause 2, half one: activating the enumerated sound-toggle token over the
/// wire TOGGLES the checkbox — the readout flips "On" → "Off" and the first-party
/// `checkbox_self_update` clears `Checked` — and the follow-up `app_flow` reports the new
/// `checked` value, so an agent confirms the flip without a screenshot.
///
/// Pin: forging a `FocusActivated` (the GTW-787 mechanism) would NOT move this — nothing on
/// this screen reads that message on the checkbox's behalf. Only a real key reaches
/// `checkbox_on_key_input`, which is why the consumer emits one.
#[test]
fn activating_the_sound_toggle_flips_the_setting() {
    let (mut app, tx) = ui_options_app_with_net_qa();
    assert_eq!(
        running_state(&app),
        Some(RunningState::Options),
        "precondition: the fixture rests on the Options screen",
    );
    assert_eq!(
        sound_value_text(&mut app).as_deref(),
        Some("On"),
        "precondition: sound defaults On, so the readout starts On",
    );
    let Some(toggle) = single_with::<SoundToggle>(&mut app) else {
        unreachable!("the Options screen spawns exactly one sound toggle");
    };

    let Some(focus) = read_focus(&mut app, &tx) else {
        unreachable!("the Options screen must enumerate a FocusView");
    };
    let Some(row) = focus
        .focusables
        .iter()
        .find(|row| *row.token == toggle.to_bits())
    else {
        unreachable!("the sound toggle is enumerated");
    };
    assert_eq!(
        row.checked.map(|checked| *checked),
        Some(true),
        "the enumeration reports the pre-flip value",
    );

    let receipt = activate_target(&mut app, &tx, row.token);
    assert_eq!(
        receipt,
        FocusControlReceipt::Applied,
        "a listed control's activation is dispatched",
    );

    let flipped = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_entity(toggle)
                .is_ok_and(|entity| entity.get::<Checked>().is_none())
        },
        SETTLE_BUDGET,
    );
    assert!(
        flipped,
        "the emitted Enter must reach the first-party checkbox and clear Checked",
    );
    assert_eq!(
        sound_value_text(&mut app).as_deref(),
        Some("Off"),
        "and the screen's own settings chain must repaint the readout to Off",
    );

    let Some(after) = read_focus(&mut app, &tx) else {
        unreachable!("the Options screen still enumerates a FocusView");
    };
    let Some(after_row) = after
        .focusables
        .iter()
        .find(|row| *row.token == toggle.to_bits())
    else {
        unreachable!("the sound toggle is still enumerated");
    };
    assert_eq!(
        after_row.checked.map(|checked| *checked),
        Some(false),
        "a follow-up app_flow reports the new checked value, so an agent confirms the \
         toggle landed over the wire alone",
    );
}

/// Acceptance clause 2, half two: activating the enumerated Continue token over the wire
/// leaves the Options screen for the Main Menu, through the screen's OWN activation path.
///
/// Pin: the fixture rests on Options and never leaves on its own; only the activation moves
/// it. The path runs through `gdtf_ui`'s real keyboard bridge raising `FocusActivated`, so
/// this stays true if GTW-869 later swaps Continue onto a first-party widget that reads
/// `FocusedInput` instead — the consumer drives the key, not the message.
#[test]
fn activating_continue_leaves_the_options_screen() {
    let (mut app, tx) = ui_options_app_with_net_qa();
    assert_eq!(
        running_state(&app),
        Some(RunningState::Options),
        "precondition: the fixture rests on the Options screen",
    );
    let Some(continue_button) = single_with::<ContinueButton>(&mut app) else {
        unreachable!("the Options screen spawns exactly one Continue button");
    };
    let Some(focus) = read_focus(&mut app, &tx) else {
        unreachable!("the Options screen must enumerate a FocusView");
    };
    let Some(row) = focus
        .focusables
        .iter()
        .find(|row| *row.token == continue_button.to_bits())
    else {
        unreachable!("the Continue button is enumerated");
    };

    let receipt = activate_target(&mut app, &tx, row.token);
    assert_eq!(receipt, FocusControlReceipt::Applied);

    let left = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        SETTLE_BUDGET,
    );
    assert!(
        left,
        "activating Continue over the wire must return to the Main Menu through the \
         screen's real activation path; stuck at {:?}",
        running_state(&app),
    );
}

/// A bare `Activate` clicks whatever currently holds focus — the Options screen focuses its
/// sound toggle on entry, so the bare command flips it with no token at all.
#[test]
fn bare_activate_clicks_the_focused_control() {
    let (mut app, tx) = ui_options_app_with_net_qa();
    let Some(toggle) = single_with::<SoundToggle>(&mut app) else {
        unreachable!("the Options screen spawns exactly one sound toggle");
    };

    let reply = send(&tx, QaRequest::FocusControl(FocusCommandNet::Activate));
    app.update();
    let Ok(QaResponse::FocusControlled(receipt)) = reply.try_recv() else {
        unreachable!("FocusControl must answer with a FocusControlled receipt");
    };
    assert_eq!(receipt, FocusControlReceipt::Applied);

    let flipped = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_entity(toggle)
                .is_ok_and(|entity| entity.get::<Checked>().is_none())
        },
        SETTLE_BUDGET,
    );
    assert!(
        flipped,
        "a bare Activate must click the focused control (the sound toggle)",
    );
}
