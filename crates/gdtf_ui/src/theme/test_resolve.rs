//! Tests for the theme schema and spec→runtime resolution.

use bevy::prelude::*;

use super::{
    fallback::SHIPPED_GRIMDARK_RON,
    newtypes::{MarginSpec, Srgba4},
    spec::{
        BackgroundThemeSpec, ButtonThemeSpec, GdtfThemeSpec, PanelThemeSpec, TextThemeSpec,
        TitleThemeSpec,
    },
};

/// **Structure / smoke:** the shipped `grimdark.ron` deserializes into a
/// [`GdtfThemeSpec`] and `resolve()` yields a complete [`GdtfTheme`] — every
/// text-bearing sub-theme carries exactly the handle the resolver returned.
///
/// Deliberately value-agnostic: `grimdark.ron` is the **tunable**,
/// data-driven styling source of truth, so this pins only that the shipped
/// file parses and fully resolves — never a specific color or scalar. The `?`
/// turns a deserialization failure into a test failure without a denied
/// `panic!`.
#[test]
fn shipped_grimdark_ron_parses_and_resolves() -> Result<(), ron::error::SpannedError> {
    let spec: GdtfThemeSpec = ron::from_str(SHIPPED_GRIMDARK_RON)?;

    let font = Handle::<Font>::default();
    let theme = spec.resolve(|_| font.clone());

    // A complete theme: every text-bearing sub-theme carries the handle the
    // resolver returned; every other field exists by construction (the struct
    // cannot resolve partially), so reaching here is the completeness check.
    assert_eq!(
        theme.button.font, font,
        "button sub-theme carries the resolved font"
    );
    assert_eq!(
        theme.title.font, font,
        "title sub-theme carries the resolved font"
    );
    assert_eq!(
        theme.text.font, font,
        "text sub-theme carries the resolved font"
    );

    Ok(())
}

/// An in-test spec with **distinctive** per-field values (no two share a
/// quad/scalar) so a swapped or dropped mapping in `resolve` cannot
/// coincidentally pass — decoupled from the shipped file. Shared by the two
/// mechanism tests below.
fn distinctive_spec() -> GdtfThemeSpec {
    GdtfThemeSpec {
        default_font: String::from("fonts/default.ttf"),
        background:   BackgroundThemeSpec {
            color: Srgba4([0.01, 0.02, 0.03, 0.04]),
        },
        panel:        PanelThemeSpec {
            color:            Srgba4([0.10, 0.11, 0.12, 0.13]),
            border_color:     Srgba4([0.20, 0.21, 0.22, 0.23]),
            border_width_px:  1.5,
            corner_radius_px: 2.5,
            margin:           MarginSpec {
                left:   3.5,
                right:  4.5,
                top:    5.5,
                bottom: 6.5,
            },
        },
        button:       ButtonThemeSpec {
            color:            Srgba4([0.30, 0.31, 0.32, 0.33]),
            disabled:         Srgba4([0.40, 0.41, 0.42, 0.43]),
            active:           Srgba4([0.44, 0.45, 0.46, 0.47]),
            hover:            Srgba4([0.50, 0.51, 0.52, 0.53]),
            pressed:          Srgba4([0.60, 0.61, 0.62, 0.63]),
            text_color:       Srgba4([0.70, 0.71, 0.72, 0.73]),
            font_size_pt:     7.5,
            border_color:     Srgba4([0.80, 0.81, 0.82, 0.83]),
            border_width_px:  8.5,
            corner_radius_px: 9.5,
            margin:           MarginSpec {
                left:   10.5,
                right:  11.5,
                top:    12.5,
                bottom: 13.5,
            },
            font:             None,
        },
        title:        TitleThemeSpec {
            text_color:   Srgba4([0.15, 0.16, 0.17, 0.18]),
            font_size_pt: 14.5,
            font:         None,
        },
        text:         TextThemeSpec {
            text_color:   Srgba4([0.25, 0.26, 0.27, 0.28]),
            font_size_pt: 15.5,
            font:         None,
        },
    }
}

/// **Mechanism (background / panel):** `resolve()` maps the background and
/// panel sub-theme fields to their runtime counterparts.
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
    assert_eq!(
        theme.panel.border_width_px.to_bits(),
        1.5_f32.to_bits(),
        "panel.border_width"
    );
    assert_eq!(
        theme.panel.corner_radius_px.to_bits(),
        2.5_f32.to_bits(),
        "panel.corner_radius"
    );
    assert_eq!(
        theme.panel.margin.l.to_bits(),
        3.5_f32.to_bits(),
        "panel margin l"
    );
    assert_eq!(
        theme.panel.margin.r.to_bits(),
        4.5_f32.to_bits(),
        "panel margin r"
    );
    assert_eq!(
        theme.panel.margin.t.to_bits(),
        5.5_f32.to_bits(),
        "panel margin t"
    );
    assert_eq!(
        theme.panel.margin.b.to_bits(),
        6.5_f32.to_bits(),
        "panel margin b"
    );
    assert_eq!(&**theme.default_font, "fonts/default.ttf", "default_font");
}

/// **Mechanism (button / title / text):** `resolve()` maps the button, title,
/// and text sub-theme fields to their runtime counterparts.
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
    assert_eq!(
        theme.button.border_width_px.to_bits(),
        8.5_f32.to_bits(),
        "button.border_width"
    );
    assert_eq!(
        theme.button.corner_radius_px.to_bits(),
        9.5_f32.to_bits(),
        "button.corner_radius"
    );
    assert_eq!(
        theme.button.margin.l.to_bits(),
        10.5_f32.to_bits(),
        "button margin l"
    );
    assert_eq!(
        theme.button.margin.r.to_bits(),
        11.5_f32.to_bits(),
        "button margin r"
    );
    assert_eq!(
        theme.button.margin.t.to_bits(),
        12.5_f32.to_bits(),
        "button margin t"
    );
    assert_eq!(
        theme.button.margin.b.to_bits(),
        13.5_f32.to_bits(),
        "button margin b"
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

/// **Font override mechanism:** a text-bearing sub-theme with a `font`
/// override resolves to the **override** key's handle, while a sub-theme
/// without one resolves to the **`default_font`** key's handle.
///
/// Drives `resolve` with a closure that mints a distinct
/// [`Handle<Font>`] per key (via a path-keyed map), so the test can prove the
/// override-vs-default key selection without any `AssetServer`.
#[test]
fn resolve_picks_override_font_else_default_font() {
    use std::collections::HashMap;

    // Title overrides its font; button and text do not, so they fall to the
    // default_font. Each distinct key maps to a distinct reserved handle.
    let spec = GdtfThemeSpec {
        default_font: String::from("fonts/default.ttf"),
        background:   BackgroundThemeSpec {
            color: Srgba4([0.0, 0.0, 0.0, 1.0]),
        },
        panel:        PanelThemeSpec {
            color:            Srgba4([0.0, 0.0, 0.0, 1.0]),
            border_color:     Srgba4([0.0, 0.0, 0.0, 1.0]),
            border_width_px:  1.0,
            corner_radius_px: 1.0,
            margin:           MarginSpec {
                left:   0.0,
                right:  0.0,
                top:    0.0,
                bottom: 0.0,
            },
        },
        button:       ButtonThemeSpec {
            color:            Srgba4([0.0, 0.0, 0.0, 1.0]),
            disabled:         Srgba4([0.0, 0.0, 0.0, 1.0]),
            active:           Srgba4([0.0, 0.0, 0.0, 1.0]),
            hover:            Srgba4([0.0, 0.0, 0.0, 1.0]),
            pressed:          Srgba4([0.0, 0.0, 0.0, 1.0]),
            text_color:       Srgba4([0.0, 0.0, 0.0, 1.0]),
            font_size_pt:     18.0,
            border_color:     Srgba4([0.0, 0.0, 0.0, 1.0]),
            border_width_px:  1.0,
            corner_radius_px: 1.0,
            margin:           MarginSpec {
                left:   0.0,
                right:  0.0,
                top:    0.0,
                bottom: 0.0,
            },
            font:             None,
        },
        title:        TitleThemeSpec {
            text_color:   Srgba4([0.0, 0.0, 0.0, 1.0]),
            font_size_pt: 36.0,
            font:         Some(String::from("fonts/title-override.ttf")),
        },
        text:         TextThemeSpec {
            text_color:   Srgba4([0.0, 0.0, 0.0, 1.0]),
            font_size_pt: 18.0,
            font:         None,
        },
    };

    // A path-keyed handle source: each distinct key gets its own weak handle
    // minted from a deterministic-but-distinct UUID, so no `AssetServer` is
    // needed to prove override-vs-default key selection.
    let mut handles: HashMap<String, Handle<Font>> = HashMap::new();
    for key in ["fonts/default.ttf", "fonts/title-override.ttf"] {
        handles.insert(String::from(key), weak_font_handle(key));
    }
    let default_handle = handles["fonts/default.ttf"].clone();
    let title_handle = handles["fonts/title-override.ttf"].clone();

    let theme = spec.resolve(|key| handles[key].clone());

    assert_eq!(
        theme.title.font, title_handle,
        "title with a font override must resolve to the override key's handle",
    );
    assert_eq!(
        theme.button.font, default_handle,
        "button without a font override must resolve to the default_font handle",
    );
    assert_eq!(
        theme.text.font, default_handle,
        "text without a font override must resolve to the default_font handle",
    );
    assert_ne!(
        theme.title.font, theme.text.font,
        "the overriding title font must differ from the default font",
    );
}

/// Mints a deterministic-but-distinct weak [`Handle<Font>`] per font key, so
/// the override test can prove key selection without an `AssetServer`.
///
/// Uses the public [`Handle::Uuid`] weak-handle variant with a FNV-1a hash of
/// the path as the UUID — distinct keys hash to distinct UUIDs, so the
/// resolved handles compare unequal.
fn weak_font_handle(key: &str) -> Handle<Font> {
    use std::marker::PhantomData;

    use bevy::asset::uuid::Uuid;

    let mut id: u128 = 0xcbf2_9ce4_8422_2325;
    for byte in key.bytes() {
        id = id
            .wrapping_mul(0x0000_0100_0000_01b3)
            .wrapping_add(u128::from(byte));
    }
    Handle::Uuid(Uuid::from_u128(id), PhantomData)
}
