use gdtf_game::test_support::BattlescapeButton;
use gdtf_ui::{MenuScreen, theme::GdtfTheme};

use super::socket_support::{TestResult, game_app_listening};

#[test]
fn the_socket_fixture_rests_on_a_menu_that_really_spawned() -> TestResult {
    let (mut app, _port) = game_app_listening()?;

    assert!(
        app.world().get_resource::<GdtfTheme>().is_some(),
        "the real Load state inserts the theme, which is what lets the menu spawn at all",
    );

    let screens = app
        .world_mut()
        .query::<&MenuScreen>()
        .iter(app.world())
        .count();
    assert!(
        screens >= 1,
        "the menu screen entity must exist, or `spawn_menu` returned early",
    );

    let buttons = app
        .world_mut()
        .query::<&BattlescapeButton>()
        .iter(app.world())
        .count();
    assert!(
        buttons >= 1,
        "the menu's own buttons must exist, so a command that reads a widget sees real UI",
    );
    Ok(())
}
