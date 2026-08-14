//! Main menu: the focused button wears a ring, and hover never puts one on.
use bevy::{
    ecs::{component::Component, entity::Entity},
    input_focus::InputFocus,
    prelude::With,
    state::state::State,
    ui::{BackgroundColor, Interaction, Outline},
};
use gdtf_app::test_support::{
    AppState, BattlescapeButton, OptionsButton, QuitButton, RunningState,
};
use gdtf_test_utils::GdtfTestAppBuilder;
use gdtf_ui::theme::default_theme;

fn menu_app() -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.update();
    app
}

fn single_with<M: Component>(app: &mut bevy::app::App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => *one,
        other => unreachable!("expected exactly one marked button, found {}", other.len()),
    }
}

fn has_ring(app: &bevy::app::App, entity: Entity) -> bool {
    app.world().get::<Outline>(entity).is_some()
}

// The three buttons the nav map links; HiveScape is disabled and stays out.
fn enabled_buttons(app: &mut bevy::app::App) -> [(&'static str, Entity); 3] {
    [
        ("Battlescape", single_with::<BattlescapeButton>(app)),
        ("Options", single_with::<OptionsButton>(app)),
        ("Quit", single_with::<QuitButton>(app)),
    ]
}

fn ringed(app: &mut bevy::app::App) -> Vec<&'static str> {
    enabled_buttons(app)
        .into_iter()
        .filter(|(_, entity)| has_ring(app, *entity))
        .map(|(name, _)| name)
        .collect()
}

#[test]
fn exactly_the_focused_menu_button_wears_the_ring() {
    let mut app = menu_app();
    let battlescape = single_with::<BattlescapeButton>(&mut app);
    let options = single_with::<OptionsButton>(&mut app);

    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(battlescape),
        "the menu starts with focus on Battlescape",
    );
    assert_eq!(
        ringed(&mut app),
        vec!["Battlescape"],
        "exactly the focused button wears the ring",
    );

    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(options, bevy::input_focus::FocusCause::Navigated);
    app.update();

    assert_eq!(
        ringed(&mut app),
        vec!["Options"],
        "the ring follows focus, and only one button wears it",
    );
    assert_eq!(
        app.world().resource::<State<RunningState>>().get(),
        &RunningState::Menu,
        "the menu is still the running screen",
    );
}

#[test]
fn focus_wears_a_ring_and_never_borrows_the_hover_fill() {
    let mut app = menu_app();
    let battlescape = single_with::<BattlescapeButton>(&mut app);
    let theme = default_theme();

    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(battlescape, bevy::input_focus::FocusCause::Navigated);
    app.update();

    assert!(
        has_ring(&app, battlescape),
        "the focused button wears the ring",
    );
    assert_eq!(
        app.world().get::<Interaction>(battlescape).copied(),
        Some(Interaction::None),
        "this button is focused and unhovered — the fill check below means nothing otherwise",
    );
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(battlescape)
            .map(|fill| fill.0),
        Some(*theme.button.color),
        "a focused button keeps the resting fill; the hover fill belongs to hover alone",
    );
}
