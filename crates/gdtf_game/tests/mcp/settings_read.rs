use bevy::{
    app::App,
    ecs::{entity::Entity, world::EntityRef},
    prelude::Text,
    state::state::NextState,
    ui_widgets::ValueChange,
};
use cobalt_mcp_protocol::{
    command::{CommandOutcome, RunOptions},
    message::QaResponse,
    ports::McpPort,
};
use gdtf_game::{
    qa_wire::shell::SoundNet,
    test_support::{RunningState, SoundToggle, SoundValueLabel},
};
use gdtf_test_utils::advance_until;
use serde::Deserialize;

use super::{
    command_exchange::{SETTINGS_READ, exchange, run},
    socket_support::{TestError, TestResult, game_app_listening},
};

#[derive(Debug, Deserialize)]
struct SettingsBody {
    sound: SoundNet,
}

fn sound_toggle(app: &App) -> Option<Entity> {
    app.world()
        .iter_entities()
        .find(EntityRef::contains::<SoundToggle>)
        .map(|entity| entity.id())
}

/// What the Options screen shows for sound, which `sync_sound_value_label` writes from settings.
fn sound_readout(app: &App) -> Option<String> {
    app.world()
        .iter_entities()
        .find(EntityRef::contains::<SoundValueLabel>)
        .and_then(|entity| entity.get::<Text>())
        .map(|text| text.0.clone())
}

/// The menu app driven to Options with sound switched off through the real toggle widget.
fn sound_switched_off_app() -> Result<(App, McpPort), TestError> {
    let (mut app, port) = game_app_listening()?;
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Options);
    advance_until(&mut app, |app| sound_toggle(app).is_some());
    let Some(toggle) = sound_toggle(&app) else {
        return Err("the sound toggle vanished between the drive and the read".into());
    };
    let before = sound_readout(&app);
    app.world_mut().trigger(ValueChange {
        source:   toggle,
        value:    false,
        is_final: true,
    });
    advance_until(&mut app, |app| sound_readout(app) != before);
    Ok((app, port))
}

fn ran_body(reply: QaResponse) -> String {
    let QaResponse::Outcome(CommandOutcome::Ran { reply, .. }) = reply else {
        unreachable!("a plain settings.read call must RUN, got {reply:?}");
    };
    reply.as_str().to_owned()
}

fn sound_in(body: &str) -> SoundNet {
    let Ok(settings) = ron::de::from_str::<SettingsBody>(body) else {
        unreachable!("the reply body decodes into the published settings shape: {body}");
    };
    settings.sound
}

fn read_sound(fixture: fn() -> Result<(App, McpPort), TestError>) -> Result<SoundNet, TestError> {
    let reply = exchange(fixture, run(SETTINGS_READ, "()", RunOptions::default()))?;
    Ok(sound_in(&ran_body(reply)))
}

#[test]
fn settings_read_answers_the_sound_setting_of_an_untouched_app() -> TestResult {
    assert_eq!(
        read_sound(game_app_listening)?,
        SoundNet::new(true),
        "an app nobody has touched carries `GameSettings::default()`, whose sound is on",
    );
    Ok(())
}

#[test]
fn settings_read_follows_a_sound_change_made_on_the_options_screen() -> TestResult {
    assert_eq!(
        read_sound(sound_switched_off_app)?,
        SoundNet::new(false),
        "the reply must carry the live settings resource, not a constant: this app had its \
         sound switched off through the real Options toggle before the command ran",
    );
    Ok(())
}

#[test]
fn settings_read_keeps_the_dev_tools_stepper_flag_off_the_wire() -> TestResult {
    let reply = exchange(
        game_app_listening,
        run(SETTINGS_READ, "()", RunOptions::default()),
    )?;
    let body = ran_body(reply);

    assert!(
        !body.contains("procgen_stepper"),
        "this test runs with `dev_tools` on, so a reply that mirrored the settings struct \
         would carry the stepper flag and the published shape would vary by build flag: {body}",
    );
    Ok(())
}
