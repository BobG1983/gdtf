use bevy::{
    prelude::*,
    text::{FontSize, FontSource, TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, BorderColor as UiBorderColor, Node, Val},
};

use super::{
    super::{ThemeRole, Themed},
    support::{app_with_apply_theme, theme},
};

#[test]
fn apply_theme_paints_role_appropriate_visuals_from_the_resource()
-> Result<(), ron::error::SpannedError> {
    let mut app = app_with_apply_theme();
    app.insert_resource(theme(
        [0.12, 0.12, 0.15],
        [0.20, 0.20, 0.24],
        2.0,
        5.0,
        18.0,
    )?);

    let text = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::ButtonText), Text::new("x")))
        .id();
    let button = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Button), Node::default()))
        .id();

    app.update();

    let world = app.world();

    assert_eq!(
        world.get::<UiTextColor>(text).map(|c| c.0),
        Some(Color::srgb(0.84, 0.80, 0.73)),
        "button-text entity should get the button text color",
    );
    assert_eq!(
        world.get::<TextFont>(text).map(|f| f.font.clone()),
        Some(FontSource::from(Handle::<Font>::default())),
        "button-text entity should get the button font handle",
    );
    assert!(
        world
            .get::<TextFont>(text)
            .is_some_and(|f| f.font_size == FontSize::Px(18.0)),
        "button-text entity font size should be the button size",
    );

    assert_eq!(
        world.get::<BackgroundColor>(button).map(|c| c.0),
        Some(Color::srgb(0.12, 0.12, 0.15)),
        "button entity should get the button fill",
    );
    assert_eq!(
        world.get::<UiBorderColor>(button).map(|b| b.top),
        Some(Color::srgb(0.20, 0.20, 0.24)),
        "button entity should get the button border color on every edge",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.border.left),
        Some(Val::Vw(2.0)),
        "button border width (Vw)",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.padding.left),
        Some(Val::Vw(8.0)),
        "left content padding (Vw)",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.padding.top),
        Some(Val::Vh(6.0)),
        "top content padding (Vh)",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.border_radius.top_left),
        Some(Val::Vw(5.0)),
        "corner radius (Vw)",
    );

    Ok(())
}

#[test]
fn apply_theme_paints_background_and_panel() -> Result<(), ron::error::SpannedError> {
    let mut app = app_with_apply_theme();
    app.insert_resource(theme(
        [0.12, 0.12, 0.15],
        [0.20, 0.20, 0.24],
        2.0,
        5.0,
        18.0,
    )?);

    let background = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Background), Node::default()))
        .id();
    let panel = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Panel), Node::default()))
        .id();

    app.update();

    let world = app.world();
    assert_eq!(
        world.get::<BackgroundColor>(background).map(|c| c.0),
        Some(Color::srgb(0.05, 0.05, 0.06)),
        "background entity should get the backdrop fill",
    );
    assert_eq!(
        world.get::<Node>(background).map(|n| n.border.left),
        Some(Val::Px(0.0)),
        "background node must keep its default border, not get a themed border width",
    );

    assert_eq!(
        world.get::<BackgroundColor>(panel).map(|c| c.0),
        Some(Color::srgba(0.16, 0.16, 0.18, 0.55)),
        "panel entity should get the panel fill",
    );
    assert_eq!(
        world.get::<Node>(panel).map(|n| n.border.left),
        Some(Val::Vw(3.0)),
        "panel border width from the panel sub-theme (Vw)",
    );
    assert_eq!(
        world.get::<Node>(panel).map(|n| n.padding.left),
        Some(Val::Vw(9.0)),
        "panel padding from the panel sub-theme (Vw)",
    );

    Ok(())
}

#[test]
fn apply_theme_uses_title_sub_theme_font_size() -> Result<(), ron::error::SpannedError> {
    let mut app = app_with_apply_theme();
    let theme_res = theme([0.12, 0.12, 0.15], [0.20, 0.20, 0.24], 2.0, 5.0, 18.0)?;
    app.insert_resource(theme_res.clone());

    let title = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Title), Text::new("GRIMDARK TURFWAR")))
        .id();

    app.update();

    let world = app.world();
    assert_eq!(
        world.get::<UiTextColor>(title).map(|c| c.0),
        Some(Color::srgb(0.84, 0.80, 0.73)),
        "title must use the title text color",
    );
    assert_eq!(
        world.get::<TextFont>(title).map(|f| f.font.clone()),
        Some(FontSource::from(Handle::<Font>::default())),
        "title must use the title font handle",
    );
    let title_size = world
        .get::<TextFont>(title)
        .and_then(|f| match f.font_size {
            FontSize::Px(px) => Some(px),
            _ => None,
        });
    assert!(
        title_size.is_some_and(|s| (s - 36.0).abs() < f32::EPSILON),
        "title font size must be the title sub-theme's own font_size_pt (36)",
    );
    let body_size = *theme_res.text.font_size_pt;
    assert!(
        title_size.is_some_and(|s| s > body_size),
        "title must be strictly larger than the body/button text size",
    );

    Ok(())
}
