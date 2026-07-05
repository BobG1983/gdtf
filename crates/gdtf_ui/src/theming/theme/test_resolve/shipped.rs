//! Shipped `ui_theme` RON smoke test: the tunable file parses and fully
//! resolves (value-agnostic).

use bevy::prelude::*;

use super::super::{fallback::SHIPPED_GRIMDARK_RON, spec::GdtfThemeSpec};

/// **Structure / smoke:** the shipped `ui_theme.tuning.ron` deserializes into a
/// [`GdtfThemeSpec`] and `resolve()` yields a complete `GdtfTheme` — every
/// text-bearing sub-theme carries exactly the handle the resolver returned.
///
/// Deliberately value-agnostic: `ui_theme.tuning.ron` is the **tunable**,
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
