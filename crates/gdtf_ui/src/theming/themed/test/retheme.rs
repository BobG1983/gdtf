use bevy::{
    prelude::*,
    text::{FontSize, TextFont},
    ui::{BackgroundColor, BorderColor as UiBorderColor, Node, Val},
};

use super::{
    super::{ThemeRole, Themed},
    support::{app_with_apply_theme, theme},
};
use crate::theme::GdtfTheme;

#[test]
fn re_theme_after_resource_mutation_repaints_with_new_palette()
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

    let new_button = Color::srgb(0.50, 0.10, 0.30);
    let new_border = Color::srgb(0.99, 0.40, 0.00);
    app.insert_resource(theme(
        [0.50, 0.10, 0.30],
        [0.99, 0.40, 0.00],
        4.0,
        9.0,
        30.0,
    )?);

    app.update();

    let world = app.world();
    assert!(
        world
            .get::<TextFont>(text)
            .is_some_and(|f| f.font_size == FontSize::Px(30.0)),
        "button-text size must reflect the NEW theme after re-run",
    );
    assert_eq!(
        world.get::<BackgroundColor>(button).map(|c| c.0),
        Some(new_button),
        "button fill must reflect the NEW theme after re-run",
    );
    assert_eq!(
        world.get::<UiBorderColor>(button).map(|b| b.top),
        Some(new_border),
        "border color must reflect the NEW theme after re-run",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.border.left),
        Some(Val::Vw(4.0)),
        "border width must reflect the NEW theme after re-run (Vw)",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.border_radius.top_left),
        Some(Val::Vw(9.0)),
        "corner radius must reflect the NEW theme after re-run (Vw)",
    );

    Ok(())
}

#[test]
fn absent_theme_does_not_panic() {
    let mut app = app_with_apply_theme();
    app.world_mut()
        .spawn((Themed::new(ThemeRole::Panel), Node::default()));

    app.update();

    assert!(
        app.world().get_resource::<GdtfTheme>().is_none(),
        "test precondition: GdtfTheme must be absent for this guard check",
    );
}
