use bevy::{
    MinimalPlugins,
    asset::{AssetApp, AssetEvent, AssetPlugin, AssetServer, Assets, Handle},
    input::InputPlugin,
    prelude::*,
    text::Font,
    ui::{BackgroundColor, Interaction, Node, widget::Button},
};
use cobalt_ron_assets::{HotRonHandle, RonAsset, RonAssetAppExt};

use crate::{
    UiPlugin,
    theme::{GdtfTheme, GdtfThemeSpec},
    themed::{ThemeRole, Themed},
};

fn spec(
    text: [f32; 3],
    panel: [f32; 3],
    hover: [f32; 3],
) -> Result<GdtfThemeSpec, ron::error::SpannedError> {
    let [tr, tg, tb] = text;
    let [pr, pg, pb] = panel;
    let [hr, hg, hb] = hover;
    let ron = format!(
        "(\
         default_font: \"fonts/test.ttf\", \
         background: ( color: (0.05, 0.05, 0.06, 1.0) ), \
         panel: ( color: ({pr}, {pg}, {pb}, 1.0), border_color: (0.20, 0.20, 0.24, 1.0), \
                  border_width: 1.0, corner_radius: 2.0, \
                  margin: (left: 8.0, right: 8.0, top: 6.0, bottom: 6.0) ), \
         button: ( color: ({pr}, {pg}, {pb}, 1.0), disabled: (0.16, 0.16, 0.18, 0.55), \
                   active: (0.45, 0.62, 0.30, 0.96), \
                   hover: ({hr}, {hg}, {hb}, 1.0), pressed: ({pr}, {pg}, {pb}, 1.0), \
                   text_color: ({tr}, {tg}, {tb}, 1.0), font_size_pt: 18.0, \
                   border_color: (0.20, 0.20, 0.24, 1.0), \
                   border_width: 1.0, corner_radius: 2.0, \
                   margin: (left: 8.0, right: 8.0, top: 6.0, bottom: 6.0) ), \
         title: ( text_color: ({tr}, {tg}, {tb}, 1.0), font_size_pt: 36.0 ), \
         text:  ( text_color: ({tr}, {tg}, {tb}, 1.0), font_size_pt: 18.0 ))",
    );
    ron::from_str(&ron)
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_ron_asset::<GdtfThemeSpec>()
        .init_asset::<Font>()
        .add_plugins(UiPlugin);
    app
}

fn add_theme_asset(app: &mut App, spec: GdtfThemeSpec) -> Handle<RonAsset<GdtfThemeSpec>> {
    app.world_mut()
        .resource_mut::<Assets<RonAsset<GdtfThemeSpec>>>()
        .add(RonAsset::new(spec))
}

fn hot_edit(app: &mut App, handle: &Handle<RonAsset<GdtfThemeSpec>>, spec: GdtfThemeSpec) {
    let mut assets = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<GdtfThemeSpec>>>();
    if let Some(mut asset) = assets.get_mut(handle) {
        **asset = spec;
    }
}

fn inject_modified(app: &mut App, handle: &Handle<RonAsset<GdtfThemeSpec>>) {
    app.world_mut()
        .write_message(AssetEvent::Modified { id: handle.id() });
}

#[test]
fn modified_event_rederives_theme_and_repaints_widgets() -> Result<(), ron::error::SpannedError> {
    let mut app = app();

    let handle = add_theme_asset(
        &mut app,
        spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?,
    );
    app.world_mut().insert_resource(
        spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?
            .resolve(|_| Handle::<Font>::default()),
    );
    app.world_mut()
        .insert_resource(HotRonHandle::new(handle.clone()));

    let text = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Text), Text::new("x")))
        .id();
    let panel = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Panel), Node::default()))
        .id();

    app.update();

    let new_text = Color::srgb(0.10, 0.90, 0.20);
    let new_panel = Color::srgb(0.50, 0.10, 0.30);
    hot_edit(
        &mut app,
        &handle,
        spec([0.10, 0.90, 0.20], [0.50, 0.10, 0.30], [0.70, 0.10, 0.40])?,
    );
    inject_modified(&mut app, &handle);

    app.update();

    let world = app.world();
    assert_eq!(
        world
            .get_resource::<GdtfTheme>()
            .map(|t| *t.text.text_color),
        Some(new_text),
        "GdtfTheme.text.text_color must be re-derived to the NEW palette",
    );
    assert_eq!(
        world.get::<bevy::text::TextColor>(text).map(|c| c.0),
        Some(new_text),
        "Themed(Text) must repaint to the NEW text color the same frame",
    );
    assert_eq!(
        world.get::<BackgroundColor>(panel).map(|c| c.0),
        Some(new_panel),
        "Themed(Panel) must repaint to the NEW panel bg the same frame",
    );

    Ok(())
}

#[test]
fn rederive_reresolves_fonts_through_the_asset_server() -> Result<(), ron::error::SpannedError> {
    let mut app = app();

    let handle = add_theme_asset(
        &mut app,
        spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?,
    );
    let expected: Handle<Font> = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>("fonts/test.ttf");
    app.world_mut().insert_resource(
        spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?
            .resolve(|_| Handle::<Font>::default()),
    );
    app.world_mut()
        .insert_resource(HotRonHandle::new(handle.clone()));

    app.update();

    hot_edit(
        &mut app,
        &handle,
        spec([0.10, 0.90, 0.20], [0.50, 0.10, 0.30], [0.70, 0.10, 0.40])?,
    );
    inject_modified(&mut app, &handle);
    app.update();

    assert_eq!(
        app.world()
            .get_resource::<GdtfTheme>()
            .map(|t| t.button.font.clone()),
        Some(expected),
        "the re-derive must resolve the button font to the server's handle for its key",
    );
    assert_ne!(
        app.world()
            .get_resource::<GdtfTheme>()
            .map(|t| t.button.font.clone()),
        Some(Handle::<Font>::default()),
        "the re-resolved font must not be the default handle",
    );

    Ok(())
}

#[test]
fn change_driven_apply_theme_does_not_clobber_hover() -> Result<(), ron::error::SpannedError> {
    let mut app = app();

    let theme = spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?
        .resolve(|_| Handle::<Font>::default());
    let hover_bg = *theme.button.hover;
    let panel_bg = *theme.button.color;
    app.world_mut().insert_resource(theme);

    let button = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Button), Button, Node::default()))
        .id();

    app.update();

    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = Interaction::Hovered;
    }
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(hover_bg),
        "precondition: the hover swap must land HoverBg",
    );

    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(hover_bg),
        "steady-state apply_theme must NOT clobber HoverBg back to PanelBg ",
    );
    assert_ne!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(panel_bg),
        "the hovered button must not have reverted to the resting PanelBg",
    );

    Ok(())
}

#[test]
fn newly_added_themed_entity_gets_base_look() -> Result<(), ron::error::SpannedError> {
    let mut app = app();
    app.world_mut().insert_resource(
        spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?
            .resolve(|_| Handle::<Font>::default()),
    );

    app.update();
    app.update();

    let panel = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Panel), Node::default()))
        .id();
    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(panel).map(|c| c.0),
        Some(Color::srgb(0.08, 0.08, 0.10)),
        "a Themed entity spawned on a steady-state frame must still get its base look",
    );

    Ok(())
}

#[test]
fn theme_change_repaints_all_themed() -> Result<(), ron::error::SpannedError> {
    let mut app = app();
    app.world_mut().insert_resource(
        spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?
            .resolve(|_| Handle::<Font>::default()),
    );

    let panel = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Panel), Node::default()))
        .id();

    app.update();
    app.update();

    let new_panel = Color::srgb(0.50, 0.10, 0.30);
    app.world_mut().insert_resource(
        spec([0.10, 0.90, 0.20], [0.50, 0.10, 0.30], [0.70, 0.10, 0.40])?
            .resolve(|_| Handle::<Font>::default()),
    );
    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(panel).map(|c| c.0),
        Some(new_panel),
        "a GdtfTheme change must repaint an already-settled Themed entity",
    );

    Ok(())
}
