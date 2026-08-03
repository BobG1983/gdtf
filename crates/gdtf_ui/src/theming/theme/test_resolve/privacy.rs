use bevy::prelude::*;

use super::super::{newtypes::Srgba4, spec::BackgroundThemeSpec};

#[test]
fn theme_newtypes_round_trip_through_private_inner() -> Result<(), ron::error::SpannedError> {
    let input = [0.12_f32, 0.34, 0.56, 0.78];
    let parsed: Srgba4 = ron::from_str("(0.12, 0.34, 0.56, 0.78)")?;
    assert_eq!(
        parsed,
        Srgba4::new(input),
        "Srgba4 must deserialize through its private inner"
    );

    let bg: BackgroundThemeSpec = ron::from_str("(color: (0.12, 0.34, 0.56, 0.78))")?;
    let resolved = bg.resolve();
    assert_eq!(
        *resolved.color,
        Color::srgba(0.12, 0.34, 0.56, 0.78),
        "ScreenColor must be readable through Deref after resolving the parsed spec"
    );

    Ok(())
}
