use bevy::prelude::*;

use super::super::{
    newtypes::{BorderWidthVw, CornerRadiusVw, MarginSpec, MarginVh, MarginVw, Srgba4},
    spec::{
        BackgroundThemeSpec, ButtonThemeSpec, GdtfThemeSpec, PanelThemeSpec, TextThemeSpec,
        TitleThemeSpec,
    },
};

pub(super) fn distinctive_spec() -> GdtfThemeSpec {
    GdtfThemeSpec {
        default_font: String::from("fonts/default.ttf"),
        background:   BackgroundThemeSpec {
            color: Srgba4::new([0.01, 0.02, 0.03, 0.04]),
        },
        panel:        PanelThemeSpec {
            color:         Srgba4::new([0.10, 0.11, 0.12, 0.13]),
            border_color:  Srgba4::new([0.20, 0.21, 0.22, 0.23]),
            border_width:  1.5,
            corner_radius: 2.5,
            margin:        MarginSpec {
                left:   3.5,
                right:  4.5,
                top:    5.5,
                bottom: 6.5,
            },
        },
        button:       ButtonThemeSpec {
            color:         Srgba4::new([0.30, 0.31, 0.32, 0.33]),
            disabled:      Srgba4::new([0.40, 0.41, 0.42, 0.43]),
            active:        Srgba4::new([0.44, 0.45, 0.46, 0.47]),
            hover:         Srgba4::new([0.50, 0.51, 0.52, 0.53]),
            pressed:       Srgba4::new([0.60, 0.61, 0.62, 0.63]),
            text_color:    Srgba4::new([0.70, 0.71, 0.72, 0.73]),
            font_size_pt:  7.5,
            border_color:  Srgba4::new([0.80, 0.81, 0.82, 0.83]),
            border_width:  8.5,
            corner_radius: 9.5,
            margin:        MarginSpec {
                left:   10.5,
                right:  11.5,
                top:    12.5,
                bottom: 13.5,
            },
            font:          None,
        },
        title:        TitleThemeSpec {
            text_color:   Srgba4::new([0.15, 0.16, 0.17, 0.18]),
            font_size_pt: 14.5,
            font:         None,
        },
        text:         TextThemeSpec {
            text_color:   Srgba4::new([0.25, 0.26, 0.27, 0.28]),
            font_size_pt: 15.5,
            font:         None,
        },
    }
}

#[test]
fn resolve_maps_background_and_panel_fields() {
    let theme = distinctive_spec().resolve(|_| Handle::<Font>::default());

    assert_eq!(
        *theme.background.color,
        Color::srgba(0.01, 0.02, 0.03, 0.04),
        "background.color"
    );
    assert_eq!(
        *theme.panel.color,
        Color::srgba(0.10, 0.11, 0.12, 0.13),
        "panel.color"
    );
    assert_eq!(
        *theme.panel.border_color,
        Color::srgba(0.20, 0.21, 0.22, 0.23),
        "panel.border_color",
    );
    let _border: BorderWidthVw = theme.panel.border_width;
    assert_eq!(
        theme.panel.border_width.to_bits(),
        1.5_f32.to_bits(),
        "panel.border_width (Vw)"
    );
    let _radius: CornerRadiusVw = theme.panel.corner_radius;
    assert_eq!(
        theme.panel.corner_radius.to_bits(),
        2.5_f32.to_bits(),
        "panel.corner_radius (Vw)"
    );
    let _ml: MarginVw = theme.panel.margin.l;
    let _mt: MarginVh = theme.panel.margin.t;
    assert_eq!(
        theme.panel.margin.l.to_bits(),
        3.5_f32.to_bits(),
        "panel margin l (Vw)"
    );
    assert_eq!(
        theme.panel.margin.r.to_bits(),
        4.5_f32.to_bits(),
        "panel margin r (Vw)"
    );
    assert_eq!(
        theme.panel.margin.t.to_bits(),
        5.5_f32.to_bits(),
        "panel margin t (Vh)"
    );
    assert_eq!(
        theme.panel.margin.b.to_bits(),
        6.5_f32.to_bits(),
        "panel margin b (Vh)"
    );
    assert_eq!(&**theme.default_font, "fonts/default.ttf", "default_font");
}

#[test]
fn resolve_maps_button_title_and_text_fields() {
    let theme = distinctive_spec().resolve(|_| Handle::<Font>::default());

    assert_eq!(
        *theme.button.color,
        Color::srgba(0.30, 0.31, 0.32, 0.33),
        "button.color"
    );
    assert_eq!(
        *theme.button.disabled,
        Color::srgba(0.40, 0.41, 0.42, 0.43),
        "button.disabled"
    );
    assert_eq!(
        *theme.button.active,
        Color::srgba(0.44, 0.45, 0.46, 0.47),
        "button.active"
    );
    assert_eq!(
        *theme.button.hover,
        Color::srgba(0.50, 0.51, 0.52, 0.53),
        "button.hover"
    );
    assert_eq!(
        *theme.button.pressed,
        Color::srgba(0.60, 0.61, 0.62, 0.63),
        "button.pressed"
    );
    assert_eq!(
        *theme.button.text_color,
        Color::srgba(0.70, 0.71, 0.72, 0.73),
        "button.text_color",
    );
    assert_eq!(
        theme.button.font_size_pt.to_bits(),
        7.5_f32.to_bits(),
        "button.font_size"
    );
    assert_eq!(
        *theme.button.border_color,
        Color::srgba(0.80, 0.81, 0.82, 0.83),
        "button.border_color",
    );
    let _bw: BorderWidthVw = theme.button.border_width;
    let _cr: CornerRadiusVw = theme.button.corner_radius;
    let _bml: MarginVw = theme.button.margin.l;
    let _bmt: MarginVh = theme.button.margin.t;
    assert_eq!(
        theme.button.border_width.to_bits(),
        8.5_f32.to_bits(),
        "button.border_width (Vw)"
    );
    assert_eq!(
        theme.button.corner_radius.to_bits(),
        9.5_f32.to_bits(),
        "button.corner_radius (Vw)"
    );
    assert_eq!(
        theme.button.margin.l.to_bits(),
        10.5_f32.to_bits(),
        "button margin l (Vw)"
    );
    assert_eq!(
        theme.button.margin.r.to_bits(),
        11.5_f32.to_bits(),
        "button margin r (Vw)"
    );
    assert_eq!(
        theme.button.margin.t.to_bits(),
        12.5_f32.to_bits(),
        "button margin t (Vh)"
    );
    assert_eq!(
        theme.button.margin.b.to_bits(),
        13.5_f32.to_bits(),
        "button margin b (Vh)"
    );

    assert_eq!(
        *theme.title.text_color,
        Color::srgba(0.15, 0.16, 0.17, 0.18),
        "title.text_color"
    );
    assert_eq!(
        theme.title.font_size_pt.to_bits(),
        14.5_f32.to_bits(),
        "title.font_size"
    );

    assert_eq!(
        *theme.text.text_color,
        Color::srgba(0.25, 0.26, 0.27, 0.28),
        "text.text_color"
    );
    assert_eq!(
        theme.text.font_size_pt.to_bits(),
        15.5_f32.to_bits(),
        "text.font_size"
    );
}
