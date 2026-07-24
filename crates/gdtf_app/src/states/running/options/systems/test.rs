//! Unit tests for the Options screen's spawn structure and widget theming
//! (GTW-637), driving the real `spawn_options_screen` / `paint_sound_toggle`
//! systems over a scene-support world (the `bsn!` builders need `AssetPlugin` +
//! `ScenePlugin`; see `inspect_panel::test`).

use bevy::{
    MinimalPlugins,
    asset::AssetPlugin,
    ecs::system::{RunSystemOnce, SystemState},
    input_focus::directional_navigation::DirectionalNavigationMap,
    prelude::*,
    scene::ScenePlugin,
    ui::{BackgroundColor, Checked},
    ui_widgets::Checkbox,
};
use gdtf_ui::theme::default_theme;

use super::{paint_sound_toggle, spawn_options_screen, theming::sound_toggle_colors};
use crate::states::running::options::{
    components::{OptionsScreenRoot, OptionsTitle, SoundToggle, SoundValueLabel},
    settings::{GameSettings, SoundEnabled},
};

/// The params `spawn_options_screen` reads, aliased out of the `type_complexity`
/// deny lint.
type SpawnParams<'w, 's> = SystemState<(
    Commands<'w, 's>,
    Option<Res<'w, gdtf_ui::theme::GdtfTheme>>,
    Res<'w, GameSettings>,
    ResMut<'w, DirectionalNavigationMap>,
)>;

/// Builds a scene-support world, inserts the fallback theme + `GameSettings` + the
/// (otherwise `UiPlugin`-owned) nav map, and runs `spawn_options_screen` once.
fn spawn_screen(sound_on: bool) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.insert_resource(default_theme());
    app.insert_resource(GameSettings {
        sound: SoundEnabled::new(sound_on),
    });
    // Under MinimalPlugins the `DirectionalNavigationPlugin` (normally pulled in by
    // `gdtf_ui::UiPlugin`) is absent, so seed the map the spawn system writes into.
    app.init_resource::<DirectionalNavigationMap>();
    let world = app.world_mut();
    let mut state: SpawnParams = SystemState::new(world);
    if let Ok((commands, theme, settings, nav_map)) = state.get_mut(world) {
        spawn_options_screen(commands, theme, settings, nav_map);
        state.apply(world);
    }
    app
}

#[test]
fn spawns_the_titled_screen_root() {
    let mut app = spawn_screen(true);
    let world = app.world_mut();
    let mut roots = world.query_filtered::<(), With<OptionsScreenRoot>>();
    assert_eq!(
        roots.iter(world).count(),
        1,
        "spawn_options_screen must spawn exactly one OptionsScreenRoot",
    );
    let mut titles = world.query_filtered::<(), With<OptionsTitle>>();
    assert_eq!(
        titles.iter(world).count(),
        1,
        "and exactly one OptionsTitle heading",
    );
    let mut values = world.query_filtered::<(), With<SoundValueLabel>>();
    assert_eq!(
        values.iter(world).count(),
        1,
        "and exactly one SoundValueLabel readout",
    );
}

/// The sound toggle is a REAL first-party `bevy_ui_widgets::Checkbox`, seeded from the
/// setting (its `Checked` state) and painted from the theme (its track color).
#[test]
fn sound_toggle_is_a_real_checkbox_seeded_from_settings_and_themed() {
    let mut app = spawn_screen(false);
    let world = app.world_mut();
    let mut toggles =
        world.query_filtered::<(Entity, &BackgroundColor), (With<SoundToggle>, With<Checkbox>)>();
    let found: Vec<(Entity, Color)> = toggles.iter(world).map(|(e, c)| (e, c.0)).collect();
    assert_eq!(
        found.len(),
        1,
        "exactly one sound toggle carrying BOTH SoundToggle and the first-party Checkbox",
    );
    let (entity, track) = found[0];
    // Sound Off in settings must leave the checkbox UNchecked (the checkbox's own state).
    assert!(
        world.get::<Checked>(entity).is_none(),
        "sound Off in settings must seed the Checkbox without the Checked component",
    );
    // The track color comes from the theme's off fill, not a literal (THEMING clause).
    assert_eq!(
        track,
        sound_toggle_colors(&default_theme()).track(SoundEnabled::new(false)),
        "the toggle track must be painted from the button sub-theme's off fill",
    );
}

/// Sound On seeds the checkbox WITH the first-party `Checked` component.
#[test]
fn sound_on_seeds_the_checkbox_checked() {
    let mut app = spawn_screen(true);
    let world = app.world_mut();
    let mut toggles = world.query_filtered::<Entity, (With<SoundToggle>, With<Checkbox>)>();
    let entity = toggles.iter(world).next();
    assert!(entity.is_some(), "a sound toggle Checkbox must exist");
    let Some(entity) = entity else {
        return;
    };
    assert!(
        world.get::<Checked>(entity).is_some(),
        "sound On in settings must seed the Checkbox with the Checked component",
    );
}

/// Hot-reload repaint in place: after the theme changes, `paint_sound_toggle`
/// re-derives the toggle's colors and repaints its track fill WITHOUT despawn/respawn
/// — the same checkbox entity is mutated (mutate-not-respawn).
#[test]
fn theme_change_repaints_the_toggle_in_place() {
    let mut app = spawn_screen(true);

    // The toggle entity id before the retheme — the same entity must survive it.
    let before = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<SoundToggle>>();
        q.iter(world).next()
    };
    assert!(
        before.is_some(),
        "precondition: a sound toggle Checkbox must exist"
    );
    let Some(toggle) = before else {
        return;
    };

    // Swap in a theme whose active fill differs, then run the paint system.
    let mut swapped = default_theme();
    let new_on = Color::srgb(0.9, 0.1, 0.1);
    swapped.button.active = gdtf_ui::theme::ActiveColor::new(new_on);
    app.insert_resource(swapped);

    let ran = app.world_mut().run_system_once(paint_sound_toggle);
    assert!(ran.is_ok(), "paint_sound_toggle must run");

    let world = app.world_mut();
    // Same entity — not despawned/respawned.
    let mut q = world.query_filtered::<Entity, With<SoundToggle>>();
    assert_eq!(
        q.iter(world).next(),
        Some(toggle),
        "the toggle must be MUTATED in place, not replaced",
    );
    // Its ON-track fill now reflects the new theme (the toggle was spawned ON).
    let track = world.get::<BackgroundColor>(toggle).map(|bg| bg.0);
    assert_eq!(
        track,
        Some(new_on),
        "the ON toggle's track must repaint to the new theme's active fill",
    );
}
