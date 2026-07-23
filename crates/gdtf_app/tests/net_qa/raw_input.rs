//! GTW-783 — the raw-input inject path (keypress / hover / focus-set), driven through the
//! REAL `apply_injects` pump on a live-battle `BattleAppBuilder` app.
//!
//! Each test injects a raw-input `NetIntent` through the same inbox the loopback listener
//! feeds, drives `app.update()`, and asserts the ACTUAL downstream windowing input the game
//! consumes — a keypress folded into `ButtonInput<KeyCode>` by Bevy's real
//! `keyboard_input_system`, the primary window's cursor position `bevy_ui` hover reads, and
//! the `InputFocus` resource `sync_hover_to_focus` writes — not a green-run-only claim.

use bevy::{
    input::{ButtonInput, keyboard::KeyCode},
    input_focus::InputFocus,
    prelude::*,
    window::PrimaryWindow,
};
use gdtf_battle_input::keybinds::{BoundKey, Keybinds};
use gdtf_qa_protocol::{
    envelope::{InjectReceipt, QaRequest, QaResponse, RejectReason},
    ids::{FocusTargetNet, PointerPosNet, PointerXNet, PointerYNet},
    intent::{KeyNet, KeyPressNet, KeybindActionNet, NetIntent},
};

use crate::inject_support::{inject_battle_app, send};

/// A keybind table with a KNOWN binding under test: `select_clear` is on Escape. The other
/// fields are arbitrary valid keys — only `select_clear` is exercised.
const fn test_keybinds() -> Keybinds {
    Keybinds {
        select_clear:     BoundKey::KeyEscape,
        level_up:         BoundKey::KeyPageUp,
        level_down:       BoundKey::KeyPageDown,
        toggle_full_view: BoundKey::KeyV,
        stance_cycle:     BoundKey::KeyC,
        aim_toggle:       BoundKey::KeyF,
        facing_cycle:     BoundKey::KeyR,
        select_next:      BoundKey::KeyTab,
        select_prev:      BoundKey::KeyTab,
    }
}

/// KEYPRESS (by physical key): an injected `PressKey(Key(Tab))` writes a real `KeyboardInput`
/// press+release pair, and Bevy's own `keyboard_input_system` folds it into
/// `ButtonInput<KeyCode>` — so `just_pressed(Tab)` reads true, exactly as a real Tab press.
/// The receipt is `Queued` the frame the intent enters the input path.
#[test]
fn injected_keypress_folds_into_button_input() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    // The battle app already runs Bevy's real keyboard input path (the scene-support harness
    // adds `InputPlugin`): `keyboard_input_system` reads the `KeyboardInput` message stream
    // in PreUpdate and maintains `ButtonInput<KeyCode>`.
    let reply = send(
        &tx,
        QaRequest::Inject(NetIntent::PressKey {
            key: KeyPressNet::Key(KeyNet::Tab),
        }),
    );
    // Frame A: route + apply_injects writes the KeyboardInput Pressed+Released messages.
    app.update();
    assert!(
        matches!(
            reply.try_recv(),
            Ok(QaResponse::Injected(InjectReceipt::Queued))
        ),
        "a keypress inject is Queued the frame it enters the windowing-input path",
    );
    // Frame B: `keyboard_input_system` folds those messages into `ButtonInput<KeyCode>`.
    app.update();

    let keys = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(
        keys.just_pressed(KeyCode::Tab),
        "the injected Tab keypress reaches ButtonInput<KeyCode> via the real keyboard path",
    );
}

/// KEYPRESS (by named action): an injected `PressKey(Action(SelectClear))` resolves through
/// the live `Keybinds` table (`select_clear` = Escape here) and presses THAT key — proving
/// the action lookup drives the real key, not a fixed one.
#[test]
fn injected_keypress_by_action_resolves_through_keybinds() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    // A live keybind table so the bound-action keypress can resolve its key through it
    // (a `MinimalPlugins` battle app has no `AssetServer` to load the shipped keybinds).
    app.world_mut().insert_resource(test_keybinds());

    let reply = send(
        &tx,
        QaRequest::Inject(NetIntent::PressKey {
            key: KeyPressNet::Action(KeybindActionNet::SelectClear),
        }),
    );
    app.update();
    assert!(
        matches!(
            reply.try_recv(),
            Ok(QaResponse::Injected(InjectReceipt::Queued))
        ),
        "a keypress-by-action inject is Queued the frame it enters the windowing-input path",
    );
    app.update();

    let keys = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(
        keys.just_pressed(KeyCode::Escape),
        "a keypress-by-action taps whatever key the live Keybinds binds it to (select_clear = Escape)",
    );
    assert!(
        !keys.just_pressed(KeyCode::Tab),
        "only the action's bound key is pressed",
    );
}

/// HOVER: an injected `Hover` sets the primary window's cursor position — the state
/// `bevy_ui`'s hover detection (and the hover-follows-focus bridge) reads — to the requested
/// window pixel. The receipt is `Queued`.
#[test]
fn injected_hover_sets_the_primary_window_cursor() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    // A primary window to hover over — `MinimalPlugins` spawns none, so we spawn our own
    // (pure ECS data; the inject path only queries + mutates the `Window` component).
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    app.update();

    let reply = send(
        &tx,
        QaRequest::Inject(NetIntent::Hover {
            at: PointerPosNet::new(PointerXNet::new(100), PointerYNet::new(80)),
        }),
    );
    app.update();
    assert!(
        matches!(
            reply.try_recv(),
            Ok(QaResponse::Injected(InjectReceipt::Queued))
        ),
        "a hover inject is Queued the frame it enters the windowing-input path",
    );

    let Some(window) = app.world().entity(window).get::<Window>() else {
        unreachable!("the spawned primary window still exists");
    };
    assert_eq!(
        window.cursor_position(),
        Some(Vec2::new(100.0, 80.0)),
        "the injected hover moved the primary window's cursor to the requested pixel",
    );
}

/// FOCUS-SET: an injected `SetFocus` at a live entity points the `InputFocus` resource at it
/// — the SAME write `sync_hover_to_focus` performs. The receipt is `Queued`.
#[test]
fn injected_set_focus_points_input_focus_at_the_entity() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    // `MinimalPlugins` has no focus stack; insert the resource the write targets.
    app.world_mut().insert_resource(InputFocus::default());
    let target = app.world_mut().spawn_empty().id();
    app.update();

    let reply = send(
        &tx,
        QaRequest::Inject(NetIntent::SetFocus {
            target: FocusTargetNet::new(target.to_bits()),
        }),
    );
    app.update();
    assert!(
        matches!(
            reply.try_recv(),
            Ok(QaResponse::Injected(InjectReceipt::Queued))
        ),
        "a focus-set at a live entity is Queued",
    );
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(target),
        "the focus-set points InputFocus at the resolved entity",
    );
}

/// FOCUS-SET (fail-closed): a DEAD (despawned) token and a MALFORMED bit pattern each resolve
/// to `Rejected(UnknownEntity)` — never a panic — and never touch `InputFocus`.
#[test]
fn injected_set_focus_dead_or_malformed_token_is_rejected() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    app.world_mut().insert_resource(InputFocus::default());
    let doomed = app.world_mut().spawn_empty().id();
    let dead_bits = doomed.to_bits();
    app.world_mut().despawn(doomed);
    app.update();

    let dead_reply = send(
        &tx,
        QaRequest::Inject(NetIntent::SetFocus {
            target: FocusTargetNet::new(dead_bits),
        }),
    );
    let malformed_reply = send(
        &tx,
        QaRequest::Inject(NetIntent::SetFocus {
            target: FocusTargetNet::new(u64::MAX),
        }),
    );
    app.update();

    assert!(
        matches!(
            dead_reply.try_recv(),
            Ok(QaResponse::Injected(InjectReceipt::Rejected(
                RejectReason::UnknownEntity
            )))
        ),
        "a despawned focus token is Rejected(UnknownEntity), never a panic",
    );
    assert!(
        matches!(
            malformed_reply.try_recv(),
            Ok(QaResponse::Injected(InjectReceipt::Rejected(
                RejectReason::UnknownEntity
            )))
        ),
        "a malformed focus token is Rejected(UnknownEntity), never a panic",
    );
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        None,
        "a rejected focus-set never touches InputFocus",
    );
}
