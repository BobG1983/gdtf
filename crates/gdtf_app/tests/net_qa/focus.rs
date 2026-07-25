//! GTW-802 — the generic focus enumeration + focus-movement path, driven through the REAL
//! router (`GetAppFlow` folds in the focus view) and the REAL `drive_focus_control` consumer
//! on a headless app resting on the Options screen (never a shadow copy).
//!
//! Four facts are pinned:
//!
//! - enumerating the Options screen over `GetAppFlow` returns exactly the controls the
//!   screen put in its OWN navigation chain — the sound checkbox (with its `checked` value)
//!   and the Continue button — with the toggle marked focused;
//! - a `Step` command moves `InputFocus` between them through the game's real
//!   `apply_navigation` over the screen's real `add_edges` chain;
//! - a `Focus` command points `InputFocus` at an enumerated control;
//! - a token that is malformed, dead, or LIVE-BUT-UNLISTED is answered a typed
//!   `FocusControlReceipt::Rejected(RejectReason::StaleToken)` — never a panic, never a
//!   silent no-op. The live-but-unlisted case is what proves the enumeration is the gate,
//!   not mere entity liveness.
//!
//! The whole suite runs OFF-BATTLE, which is itself the point: `send_input` is battle-gated,
//! so before GTW-802 none of this was reachable over the wire.

use bevy::{app::App, ecs::entity::Entity, input_focus::InputFocus};
use gdtf_app::test_support::{ContinueButton, OptionsTitle, SoundToggle};
use gdtf_qa_protocol::{
    envelope::{
        FocusCommandNet, FocusControlReceipt, FocusStepNet, QaRequest, QaResponse, RejectReason,
    },
    ids::FocusTargetNet,
    view::{FocusView, FocusableKindNet, FocusableView},
};

use crate::{
    focus_support::{options_app_with_net_qa, read_focus},
    inject_support::send,
};

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

/// The entity input focus currently rests on, if any.
fn focused_entity(app: &App) -> Option<Entity> {
    app.world().get_resource::<InputFocus>()?.get()
}

/// The enumerated row whose token names `entity`, if it was listed.
fn row_for(focus: &FocusView, entity: Entity) -> Option<&FocusableView> {
    focus
        .focusables
        .iter()
        .find(|row| *row.token == entity.to_bits())
}

/// Send one focus command, drive a frame, and read the typed receipt.
fn focus_control(
    app: &mut App,
    tx: &std::sync::mpsc::Sender<gdtf_app::test_support::IncomingRequest>,
    command: FocusCommandNet,
) -> FocusControlReceipt {
    let reply = send(tx, QaRequest::FocusControl(command));
    app.update();
    let Ok(QaResponse::FocusControlled(receipt)) = reply.try_recv() else {
        unreachable!("FocusControl must answer with a FocusControlled receipt");
    };
    receipt
}

/// Enumerating the Options screen over `GetAppFlow` returns exactly the two controls the
/// screen declared focus-navigable, with the checkbox's kind + current value and the
/// button's caption, and the toggle marked focused (its `set_initial_focus`).
///
/// Pin: this reads the LIVE screen through the game's OWN navigation graph — no marker was
/// added to Options for QA's benefit. Dropping a control from the screen's `add_edges` call
/// reddens it, which is exactly the coupling that keeps the wire list honest.
#[test]
fn enumerates_the_options_screen_focusables() {
    let (mut app, tx) = options_app_with_net_qa();
    let Some(toggle) = single_with::<SoundToggle>(&mut app) else {
        unreachable!("the Options screen spawns exactly one sound toggle");
    };
    let Some(continue_button) = single_with::<ContinueButton>(&mut app) else {
        unreachable!("the Options screen spawns exactly one Continue button");
    };

    let Some(focus) = read_focus(&mut app, &tx) else {
        unreachable!("a focus-navigable screen must enumerate a FocusView");
    };
    assert_eq!(
        focus.focusables.len(),
        2,
        "exactly the two controls Options put in its navigation chain are listed; got {:?}",
        focus.focusables,
    );

    let Some(toggle_row) = row_for(&focus, toggle) else {
        unreachable!("the sound toggle is enumerated");
    };
    assert_eq!(
        toggle_row.kind,
        FocusableKindNet::Checkbox,
        "the sound toggle reports its Checkbox kind, so a client knows activating it flips \
         a value",
    );
    assert_eq!(
        toggle_row.checked.map(|checked| *checked),
        Some(true),
        "sound defaults On, so the toggle reports checked == true before any flip",
    );
    assert!(*toggle_row.enabled, "the sound toggle is activatable");
    assert!(
        *toggle_row.focused,
        "the screen's set_initial_focus put focus on the toggle",
    );

    let Some(continue_row) = row_for(&focus, continue_button) else {
        unreachable!("the Continue button is enumerated");
    };
    assert_eq!(
        continue_row.kind,
        FocusableKindNet::Button,
        "Continue reports its Button kind",
    );
    assert_eq!(
        continue_row.label.as_str(),
        "Continue",
        "Continue's on-screen caption is handed out so a client names it by purpose",
    );
    assert_eq!(
        continue_row.checked, None,
        "a button carries no checked value",
    );
    assert!(!*continue_row.focused, "Continue does not hold focus yet");

    assert_eq!(
        focus.focused.map(|token| *token),
        Some(toggle.to_bits()),
        "the screen-level focused field answers 'activate what?' in one read",
    );
}

/// A `Step` command moves focus along the screen's OWN navigation chain, through the game's
/// real `apply_navigation` — `Next` walks toggle → Continue, `Prev` walks back.
///
/// Pin: nothing here writes `InputFocus` directly; the command writes the SAME
/// `NavigateRequest` an arrow key writes, so removing the screen's `add_edges` call (or the
/// consumer's message write) leaves focus parked and reddens the assertion.
#[test]
fn step_moves_focus_along_the_real_navigation_chain() {
    let (mut app, tx) = options_app_with_net_qa();
    let Some(toggle) = single_with::<SoundToggle>(&mut app) else {
        unreachable!("the Options screen spawns exactly one sound toggle");
    };
    let Some(continue_button) = single_with::<ContinueButton>(&mut app) else {
        unreachable!("the Options screen spawns exactly one Continue button");
    };
    assert_eq!(
        focused_entity(&app),
        Some(toggle),
        "precondition: focus starts on the sound toggle",
    );

    let receipt = focus_control(&mut app, &tx, FocusCommandNet::Step(FocusStepNet::Next));
    assert_eq!(receipt, FocusControlReceipt::Applied);
    assert_eq!(
        focused_entity(&app),
        Some(continue_button),
        "a Next step must walk the real navigation edge to Continue",
    );

    let receipt = focus_control(&mut app, &tx, FocusCommandNet::Step(FocusStepNet::Prev));
    assert_eq!(receipt, FocusControlReceipt::Applied);
    assert_eq!(
        focused_entity(&app),
        Some(toggle),
        "a Prev step must walk the reverse edge back to the sound toggle",
    );
}

/// A `Focus` command points input focus at an enumerated control by its token, without
/// activating it.
#[test]
fn focus_points_input_focus_at_an_enumerated_control() {
    let (mut app, tx) = options_app_with_net_qa();
    let Some(continue_button) = single_with::<ContinueButton>(&mut app) else {
        unreachable!("the Options screen spawns exactly one Continue button");
    };
    let Some(focus) = read_focus(&mut app, &tx) else {
        unreachable!("a focus-navigable screen must enumerate a FocusView");
    };
    let Some(row) = row_for(&focus, continue_button) else {
        unreachable!("the Continue button is enumerated");
    };

    let receipt = focus_control(&mut app, &tx, FocusCommandNet::Focus(row.token));
    assert_eq!(receipt, FocusControlReceipt::Applied);
    assert_eq!(
        focused_entity(&app),
        Some(continue_button),
        "a Focus command must point the game's own InputFocus at the named control",
    );
    assert_eq!(
        crate::focus_support::running_state(&app),
        Some(gdtf_app::test_support::RunningState::Options),
        "pointing focus must NOT activate anything — the screen stays put",
    );
}

/// Three shapes of bad token are each answered a typed
/// `Rejected(StaleToken)`: a malformed bit pattern, a despawned entity, and a LIVE entity
/// the focus graph does not list.
///
/// The third case is the load-bearing one: the Options title is alive and well but is not
/// focus-navigable, so accepting it would mean the wire could drive controls the
/// enumeration never handed out. Mere liveness is not the gate — membership in the focus
/// graph is.
#[test]
fn unlisted_and_stale_tokens_are_rejected() {
    let (mut app, tx) = options_app_with_net_qa();

    let malformed = focus_control(
        &mut app,
        &tx,
        FocusCommandNet::Focus(FocusTargetNet::new(0)),
    );
    assert_eq!(
        malformed,
        FocusControlReceipt::Rejected(RejectReason::StaleToken),
        "a malformed token is rejected StaleToken, never a panic",
    );

    let Some(toggle) = single_with::<SoundToggle>(&mut app) else {
        unreachable!("the Options screen spawns exactly one sound toggle");
    };
    let dead = FocusTargetNet::new(toggle.to_bits());
    app.world_mut().despawn(toggle);
    app.update();
    let despawned = focus_control(&mut app, &tx, FocusCommandNet::ActivateTarget(dead));
    assert_eq!(
        despawned,
        FocusControlReceipt::Rejected(RejectReason::StaleToken),
        "a token naming a despawned entity is rejected StaleToken",
    );

    let Some(title) = single_with::<OptionsTitle>(&mut app) else {
        unreachable!("the Options screen spawns exactly one title");
    };
    let unlisted = focus_control(
        &mut app,
        &tx,
        FocusCommandNet::ActivateTarget(FocusTargetNet::new(title.to_bits())),
    );
    assert_eq!(
        unlisted,
        FocusControlReceipt::Rejected(RejectReason::StaleToken),
        "a LIVE but unlisted entity is rejected too — the enumeration is the gate, not \
         liveness",
    );
}
