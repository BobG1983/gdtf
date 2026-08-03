use bevy::prelude::*;

use super::{
    fallback::{SHIPPED_GRIMDARK_RON, const_fallback_theme, default_theme},
    spec::GdtfThemeSpec,
};

#[test]
fn default_theme_matches_shipped_grimdark() -> Result<(), ron::error::SpannedError> {
    let from_ron: GdtfThemeSpec = ron::from_str(SHIPPED_GRIMDARK_RON)?;
    let from_ron = from_ron.resolve(|_| Handle::<Font>::default());

    let fallback = default_theme();

    assert_eq!(
        fallback, from_ron,
        "default_theme must resolve to the same values as the shipped grimdark RON",
    );

    Ok(())
}

#[test]
fn const_fallback_theme_is_complete() {
    let fallback = const_fallback_theme();

    assert_eq!(
        fallback.button.font,
        Handle::<Font>::default(),
        "const fallback button carries the default font handle",
    );
    assert_eq!(
        fallback.title.font,
        Handle::<Font>::default(),
        "const fallback title carries the default font handle",
    );
    assert_eq!(
        fallback.text.font,
        Handle::<Font>::default(),
        "const fallback text carries the default font handle",
    );
    assert_eq!(
        &**fallback.default_font, "fonts/Alegreya-Variable.ttf",
        "const fallback default_font is the expected loose path",
    );

    assert_eq!(
        fallback.clone(),
        fallback,
        "const fallback is a complete theme"
    );
}
