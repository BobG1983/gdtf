use bevy::{
    MinimalPlugins,
    asset::AssetPlugin,
    ecs::system::{RunSystemOnce, SystemState},
    input_focus::directional_navigation::DirectionalNavigationMap,
    prelude::*,
    scene::ScenePlugin,
    ui::{BackgroundColor, BorderColor as UiBorderColor, Checked, Node, Val},
    ui_widgets::Checkbox,
};
use gdtf_ui::theme::default_theme;

use super::{paint_sound_toggle, spawn_options_screen, theming::toggle_colors};
use crate::states::running::options::{
    components::{OptionsScreenRoot, OptionsTitle, SoundToggle, SoundValueLabel},
    settings::{GameSettings, SoundEnabled},
};

type SpawnParams<'w, 's> = SystemState<(
    Commands<'w, 's>,
    Option<Res<'w, gdtf_ui::theme::GdtfTheme>>,
    Res<'w, GameSettings>,
    ResMut<'w, DirectionalNavigationMap>,
)>;

fn spawn_screen(sound_on: bool) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.insert_resource(default_theme());
    app.insert_resource(GameSettings::default().with_sound(SoundEnabled::new(sound_on)));
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
    assert!(
        world.get::<Checked>(entity).is_none(),
        "sound Off in settings must seed the Checkbox without the Checked component",
    );
    assert_eq!(
        track,
        toggle_colors(&default_theme()).track(SoundEnabled::new(false)),
        "the toggle track must be painted from the button sub-theme's off fill",
    );
}

#[test]
fn sound_value_label_has_a_fixed_min_width() {
    for sound_on in [false, true] {
        let mut app = spawn_screen(sound_on);
        let world = app.world_mut();
        let mut labels = world.query_filtered::<&Node, With<SoundValueLabel>>();
        let widths: Vec<Val> = labels.iter(world).map(|node| node.min_width).collect();
        assert_eq!(widths.len(), 1, "exactly one SoundValueLabel");
        assert!(
            matches!(widths[0], Val::Vw(w) if w > 0.0),
            "the value readout must carry a positive Vw min_width floor (sound_on={sound_on}); \
             got {:?}",
            widths[0],
        );
    }
}

#[test]
fn off_sound_toggle_track_has_a_visible_themed_border() {
    let mut app = spawn_screen(false);
    let world = app.world_mut();
    let mut toggles = world.query_filtered::<(&UiBorderColor, &Node), With<SoundToggle>>();
    let found: Vec<(UiBorderColor, Node)> =
        toggles.iter(world).map(|(b, n)| (*b, n.clone())).collect();
    assert_eq!(found.len(), 1, "exactly one sound toggle");
    let (border, node) = &found[0];
    for edge in [
        node.border.left,
        node.border.right,
        node.border.top,
        node.border.bottom,
    ] {
        assert!(
            matches!(edge, Val::Vw(w) if w > 0.0),
            "the OFF toggle track must have a positive border width on every edge; got {edge:?}",
        );
    }
    let expected = *default_theme().button.border_color;
    assert_eq!(
        *border,
        UiBorderColor::all(expected),
        "the OFF toggle track border must be painted from the button sub-theme's border color",
    );
}

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

#[test]
fn theme_change_repaints_the_toggle_in_place() {
    let mut app = spawn_screen(true);

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

    let mut swapped = default_theme();
    let new_on = Color::srgb(0.9, 0.1, 0.1);
    swapped.button.active = gdtf_ui::theme::ActiveColor::new(new_on);
    app.insert_resource(swapped);

    let ran = app.world_mut().run_system_once(paint_sound_toggle);
    assert!(ran.is_ok(), "paint_sound_toggle must run");

    let world = app.world_mut();
    let mut q = world.query_filtered::<Entity, With<SoundToggle>>();
    assert_eq!(
        q.iter(world).next(),
        Some(toggle),
        "the toggle must be MUTATED in place, not replaced",
    );
    let track = world.get::<BackgroundColor>(toggle).map(|bg| bg.0);
    assert_eq!(
        track,
        Some(new_on),
        "the ON toggle's track must repaint to the new theme's active fill",
    );
}
