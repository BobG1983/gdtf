use bevy::prelude::*;

use super::super::{fallback::SHIPPED_GRIMDARK_RON, spec::GdtfThemeSpec};

#[test]
fn shipped_grimdark_ron_parses_and_resolves() -> Result<(), ron::error::SpannedError> {
    let spec: GdtfThemeSpec = ron::from_str(SHIPPED_GRIMDARK_RON)?;

    let font = Handle::<Font>::default();
    let theme = spec.resolve(|_| font.clone());

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
