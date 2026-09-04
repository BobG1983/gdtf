//! Menu navigation over a real socket: focus a button, step the focus, and activate what it hits.

use bevy::{
    app::App,
    ecs::{component::Component, entity::Entity},
    input_focus::{InputFocus, directional_navigation::DirectionalNavigationMap},
};
use cobalt_mcp_protocol::{
    command::{RunOptions, UnavailableCode},
    ports::McpPort,
};
use gdtf_game::{
    qa_wire::token::FocusTargetNet,
    test_support::{BattlescapeButton, MenuTitle, OptionsButton},
};

use super::{
    act_support::{decode, next},
    command_exchange::{
        APP_PHASE, INPUT_ACTIVATE, INPUT_FOCUS_STEP, INPUT_SET_FOCUS, UI_FOCUS, exchange,
        exchange_expected, ran_body, run,
    },
    input_support::{FocusBody, UiFocusBody, focus_argument, refusal},
    socket_support::{TestError, TestResult, game_app_listening},
};

/// The two menu buttons one navigation case walks between.
struct MenuWalk {
    from: FocusTargetNet,
    to:   FocusTargetNet,
}

/// The one entity carrying `M`, when the live world holds it.
fn widget<M: Component>(app: &App) -> Option<Entity> {
    app.world()
        .iter_entities()
        .find(bevy::ecs::world::EntityRef::contains::<M>)
        .map(|entity| entity.id())
}

/// The menu app plus the tokens of the Battlescape and Options buttons it registered.
fn menu_walk() -> Result<(App, McpPort, MenuWalk), TestError> {
    let (app, port) = game_app_listening()?;
    let (Some(from), Some(to)) = (
        widget::<BattlescapeButton>(&app),
        widget::<OptionsButton>(&app),
    ) else {
        return Err(
            "the menu must spawn a Battlescape and an Options button to walk between".into(),
        );
    };
    let Some(map) = app.world().get_resource::<DirectionalNavigationMap>() else {
        return Err("the app carries a directional navigation map from build".into());
    };
    if !map.neighbors.contains_key(&from) || !map.neighbors.contains_key(&to) {
        return Err(
            "both menu buttons must be registered focusable for a step to reach one from \
                    the other"
                .into(),
        );
    }
    Ok((
        app,
        port,
        MenuWalk {
            from: FocusTargetNet::new(from.to_bits()),
            to:   FocusTargetNet::new(to.to_bits()),
        },
    ))
}

/// The menu app plus the token of a widget the screen never registered as focusable.
fn menu_naming_an_unregistered_widget() -> Result<(App, McpPort, FocusTargetNet), TestError> {
    let (app, port) = game_app_listening()?;
    let Some(title) = widget::<MenuTitle>(&app) else {
        return Err("the menu must spawn its title, which is a live entity nothing focuses".into());
    };
    Ok((app, port, FocusTargetNet::new(title.to_bits())))
}

/// The menu app with keyboard focus dropped.
fn focus_cleared_app() -> Result<(App, McpPort), TestError> {
    let (mut app, port) = game_app_listening()?;
    app.world_mut().resource_mut::<InputFocus>().clear();
    Ok((app, port))
}

#[test]
fn focus_step_and_activate_walk_the_menu_the_way_the_keyboard_does() -> TestResult {
    let (replies, walk) = exchange_expected(menu_walk, |walk| {
        vec![
            run(UI_FOCUS, "()", RunOptions::default()),
            run(
                INPUT_SET_FOCUS,
                &focus_argument(walk.to),
                RunOptions::default(),
            ),
            run(INPUT_FOCUS_STEP, "(step:Prev)", RunOptions::default()),
            run(INPUT_FOCUS_STEP, "(step:Next)", RunOptions::default()),
            run(INPUT_ACTIVATE, "()", RunOptions::default()),
            run(APP_PHASE, "()", RunOptions::default()),
        ]
    })?;
    let mut replies = replies.into_iter();
    let listed = decode::<UiFocusBody>(UI_FOCUS, next(UI_FOCUS, &mut replies)?)?;
    let set = decode::<FocusBody>(INPUT_SET_FOCUS, next(INPUT_SET_FOCUS, &mut replies)?)?;
    let back = decode::<FocusBody>(INPUT_FOCUS_STEP, next(INPUT_FOCUS_STEP, &mut replies)?)?;
    let stepped = decode::<FocusBody>(INPUT_FOCUS_STEP, next(INPUT_FOCUS_STEP, &mut replies)?)?;
    let activated = decode::<FocusBody>(INPUT_ACTIVATE, next(INPUT_ACTIVATE, &mut replies)?)?;
    let phase = ran_body(APP_PHASE, next(APP_PHASE, &mut replies)?)?;

    assert!(
        listed.focusable.contains(&walk.from) && listed.focusable.contains(&walk.to),
        "the case names both targets the way `ui.focus` reports them, so both must be on the \
         list it published: {listed:?}",
    );
    assert_ne!(
        listed.focused,
        Some(walk.to),
        "the menu must start focused on some other widget, or setting focus on the Options \
         button proves nothing about `input.set_focus`: {listed:?}",
    );
    assert_eq!(
        set.focused,
        Some(walk.to),
        "setting focus on a registered widget must move focus onto that widget, whatever held \
         it before: {set:?}",
    );
    assert_eq!(
        back.focused,
        Some(walk.from),
        "one Prev step follows the menu's south chain the other way, from the Options button \
         back to the Battlescape one: {back:?}",
    );
    assert_eq!(
        stepped.focused,
        Some(walk.to),
        "one Next step follows the menu's own south edge from the Battlescape button to the \
         Options one — the reply reports the focus after the step because a step with no \
         neighbour is swallowed: {stepped:?}",
    );
    assert!(
        activated.focused.is_some(),
        "activating does not drop focus in the frame it lands: {activated:?}",
    );
    assert!(
        phase.contains("running:Some(Options)"),
        "activating the focused Options button must take the same path its press does and put \
         the app on the Options screen: {phase}",
    );
    Ok(())
}

#[test]
fn a_focus_token_the_screen_never_registered_is_refused() -> TestResult {
    let (replies, title) = exchange_expected(menu_naming_an_unregistered_widget, |title| {
        vec![run(
            INPUT_SET_FOCUS,
            &focus_argument(*title),
            RunOptions::default(),
        )]
    })?;
    let mut replies = replies.into_iter();
    let (code, note) = refusal(INPUT_SET_FOCUS, next(INPUT_SET_FOCUS, &mut replies)?)?;

    assert_eq!(
        code,
        UnavailableCode::WrongState,
        "the title is a live entity the navigation map never took, so focusing it is a wrong \
         host state rather than an unknown token — {note}",
    );
    assert!(
        !note.is_empty(),
        "the refusal names what a caller should have passed instead: {title:?}",
    );
    Ok(())
}

#[test]
fn activating_with_nothing_focused_is_refused_rather_than_guessing_a_target() -> TestResult {
    let reply = exchange(
        focus_cleared_app,
        run(INPUT_ACTIVATE, "()", RunOptions::default()),
    )?;
    let (code, note) = refusal(INPUT_ACTIVATE, reply)?;

    assert_eq!(
        code,
        UnavailableCode::WrongState,
        "activation always lands on the focused widget, so with none it must refuse rather \
         than pick one — {note}",
    );
    assert!(
        note.contains("input.set_focus"),
        "the refusal names the command that fixes it: {note}",
    );
    Ok(())
}
