//! Key presses over a real socket: the whole winit-to-ButtonInput chain, and keybind resolution.

use bevy::{
    app::App,
    input::{ButtonInput, keyboard::KeyCode},
};
use gdtf_app::qa_wire::key::{KeyNet, KeyPressNet, KeybindActionNet};
use gdtf_battle_input::{BoundKey, Keybinds};
use gdtf_qa_protocol::{command::RunOptions, ports::NetQaPort};
use gdtf_test_utils::advance_until;

use super::{
    act_support::{decode, next},
    command_exchange::{INPUT_PRESS_KEY, UI_FOCUS, exchange_expected, exchange_inspecting, run},
    input_support::{PressedKeyBody, UiFocusBody, key_argument},
    socket_support::{TestError, TestResult, game_app_listening},
};

/// How many frames the keybind table gets to arrive off the asset server.
const KEYBIND_BUDGET: u32 = 512;

/// The menu app, for a case that reads the live keyboard after the exchange.
fn menu_app() -> Result<(App, NetQaPort, ()), TestError> {
    let (app, port) = game_app_listening()?;
    Ok((app, port, ()))
}

/// Whether the live app still holds `key` down.
fn held(app: &App, key: KeyCode) -> Result<bool, TestError> {
    app.world()
        .get_resource::<ButtonInput<KeyCode>>()
        .map(|keys| keys.pressed(key))
        .ok_or_else(|| "a built app carries the keyboard button input it folds presses into".into())
}

/// The menu app once its keybind table has loaded, plus the key clear-selection is bound to.
fn keybound_app() -> Result<(App, NetQaPort, BoundKey), TestError> {
    let (mut app, port) = game_app_listening()?;
    if !advance_until(
        &mut app,
        |app| app.world().get_resource::<Keybinds>().is_some(),
        KEYBIND_BUDGET,
    ) {
        return Err(format!(
            "the keybind table must load within {KEYBIND_BUDGET} frames for a named action to \
             resolve against it"
        )
        .into());
    }
    let Some(keybinds) = app.world().get_resource::<Keybinds>() else {
        return Err("the wait above already proved the keybind table is loaded".into());
    };
    let bound = keybinds.select_clear;
    Ok((app, port, bound))
}

#[test]
fn pressing_arrow_down_moves_the_menu_focus_the_way_a_real_key_does() -> TestResult {
    let (mut app, replies, ()) = exchange_inspecting(menu_app, |()| {
        vec![
            run(UI_FOCUS, "()", RunOptions::default()),
            run(
                INPUT_PRESS_KEY,
                &key_argument(KeyPressNet::Key(KeyNet::ArrowDown)),
                RunOptions::default(),
            ),
            run(UI_FOCUS, "()", RunOptions::default()),
        ]
    })?;
    let mut replies = replies.into_iter();
    let before = decode::<UiFocusBody>(UI_FOCUS, next(UI_FOCUS, &mut replies)?)?;
    let pressed = decode::<PressedKeyBody>(INPUT_PRESS_KEY, next(INPUT_PRESS_KEY, &mut replies)?)?;
    let after = decode::<UiFocusBody>(UI_FOCUS, next(UI_FOCUS, &mut replies)?)?;

    assert_eq!(
        pressed.key,
        KeyNet::ArrowDown,
        "a physical key is pressed as itself: {pressed:?}",
    );
    assert!(
        before.focused.is_some() && after.focused.is_some(),
        "the menu holds focus on both sides of the press: {before:?} then {after:?}",
    );
    assert_ne!(
        before.focused, after.focused,
        "the press must reach `ButtonInput<KeyCode>` through the same KeyboardInput message \
         winit writes, so the keyboard focus bridge moves the focus — an unchanged focus means \
         the message never became a button press: {before:?} then {after:?}",
    );
    app.update();
    assert!(
        !held(&app, KeyCode::ArrowDown)?,
        "the reply is held back until the release has been written, so the key must not still \
         be down once the client has it — a press with no release leaves the key held forever \
         and every later command reads a keyboard the client never asked for",
    );
    Ok(())
}

#[test]
fn a_named_action_presses_whatever_key_the_keybind_table_holds() -> TestResult {
    let (replies, bound) = exchange_expected(keybound_app, |_bound| {
        vec![run(
            INPUT_PRESS_KEY,
            &key_argument(KeyPressNet::Action(KeybindActionNet::SelectClear)),
            RunOptions::default(),
        )]
    })?;
    let mut replies = replies.into_iter();
    let pressed = decode::<PressedKeyBody>(INPUT_PRESS_KEY, next(INPUT_PRESS_KEY, &mut replies)?)?;

    assert_eq!(
        pressed.key,
        KeyNet::from_bound(bound),
        "a named action is resolved through the live `Keybinds` resource, so the key pressed is \
         whatever that table holds for clear-selection — a hardcoded key would survive a rebind \
         and this assertion would not: {pressed:?}",
    );
    Ok(())
}
