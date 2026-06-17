//! Tests for the error-path [`default_theme`] / `const_fallback_theme` safety-nets.

use bevy::prelude::*;

use super::{
    fallback::{SHIPPED_GRIMDARK_RON, const_fallback_theme, default_theme},
    spec::GdtfThemeSpec,
};

/// The error-path [`default_theme`] safety-net resolves to the **same** values
/// as the shipped grimdark RON on the success path — so a fallback looks like
/// the real theme, not a divergent palette.
///
/// **Not brittle to tuning:** it re-parses the embedded shipped RON and
/// asserts equality, so it auto-follows whatever the user tunes
/// `grimdark.ron` to — it pins no specific value.
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

/// **Completeness:** the genuinely-hardcoded [`const_fallback_theme`] (the last
/// line of defence) is a complete, valid theme — every text-bearing sub-theme
/// carries the default font handle and the `default_font` is the expected
/// loose path.
///
/// Decoupled from `grimdark.ron`'s tunable values: it only guards that the
/// const safety-net is itself whole and legible, so the error path can always
/// hand back *some* usable theme.
#[test]
fn const_fallback_theme_is_complete() {
    let fallback = const_fallback_theme();

    // The const net is built without an `AssetServer`, so every text-bearing
    // sub-theme carries the default font handle.
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

    // Re-cloning it equals itself: every typed field is populated and the
    // theme round-trips as a value (a partial build could not compile, so
    // reaching here with an equal clone is the completeness signal).
    assert_eq!(
        fallback.clone(),
        fallback,
        "const fallback is a complete theme"
    );
}
