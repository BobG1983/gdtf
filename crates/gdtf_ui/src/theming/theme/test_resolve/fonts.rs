use bevy::prelude::*;

use super::super::{
    newtypes::{MarginSpec, Srgba4},
    spec::{
        BackgroundThemeSpec, ButtonThemeSpec, GdtfThemeSpec, PanelThemeSpec, TextThemeSpec,
        TitleThemeSpec,
    },
};

#[test]
fn resolve_picks_override_font_else_default_font() {
    use std::collections::HashMap;

    let spec = GdtfThemeSpec {
        default_font: String::from("fonts/default.ttf"),
        background:   BackgroundThemeSpec {
            color: Srgba4::new([0.0, 0.0, 0.0, 1.0]),
        },
        panel:        PanelThemeSpec {
            color:         Srgba4::new([0.0, 0.0, 0.0, 1.0]),
            border_color:  Srgba4::new([0.0, 0.0, 0.0, 1.0]),
            border_width:  1.0,
            corner_radius: 1.0,
            margin:        MarginSpec {
                left:   0.0,
                right:  0.0,
                top:    0.0,
                bottom: 0.0,
            },
        },
        button:       ButtonThemeSpec {
            color:         Srgba4::new([0.0, 0.0, 0.0, 1.0]),
            disabled:      Srgba4::new([0.0, 0.0, 0.0, 1.0]),
            active:        Srgba4::new([0.0, 0.0, 0.0, 1.0]),
            hover:         Srgba4::new([0.0, 0.0, 0.0, 1.0]),
            pressed:       Srgba4::new([0.0, 0.0, 0.0, 1.0]),
            text_color:    Srgba4::new([0.0, 0.0, 0.0, 1.0]),
            font_size_pt:  18.0,
            border_color:  Srgba4::new([0.0, 0.0, 0.0, 1.0]),
            border_width:  1.0,
            corner_radius: 1.0,
            margin:        MarginSpec {
                left:   0.0,
                right:  0.0,
                top:    0.0,
                bottom: 0.0,
            },
            font:          None,
        },
        title:        TitleThemeSpec {
            text_color:   Srgba4::new([0.0, 0.0, 0.0, 1.0]),
            font_size_pt: 36.0,
            font:         Some(String::from("fonts/title-override.ttf")),
        },
        text:         TextThemeSpec {
            text_color:   Srgba4::new([0.0, 0.0, 0.0, 1.0]),
            font_size_pt: 18.0,
            font:         None,
        },
    };

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
