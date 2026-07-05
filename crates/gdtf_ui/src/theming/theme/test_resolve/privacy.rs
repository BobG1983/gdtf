//! GTW-325 newtype privacy regression guard: the RON wire shape and the
//! `Deref` read path survive the private-inner sweep.

use bevy::prelude::*;

use super::super::{newtypes::Srgba4, spec::BackgroundThemeSpec};

/// **Privacy regression guard (GTW-325):** the RON-deserialized theme newtype
/// group still `Deserialize`s through its now-**private** inner, and the
/// resolved value is reachable through the derived [`Deref`].
///
/// Two halves cover the group. (1) [`Srgba4`] is the directly-`serde(transparent)`
/// newtype: deserializing a representative RON quad and asserting it equals
/// [`Srgba4::new`] proves `Deserialize` built the value through the private inner
/// (the wire shape and constructor contract are intact). (2) Feeding that parsed
/// quad through a [`BackgroundThemeSpec`] deserialize + `resolve` and reading the
/// resolved `ScreenColor` through `Deref` proves the wrap-on-resolve path and
/// the `Deref` accessor still work over the private fields. Together they prove
/// privatizing the inners changed neither the on-disk shape nor the read path.
#[test]
fn theme_newtypes_round_trip_through_private_inner() -> Result<(), ron::error::SpannedError> {
    // (1) Directly-`Deserialize` newtype: the parsed value equals `::new`,
    // proving `Deserialize` builds through the private inner via `PartialEq`.
    let input = [0.12_f32, 0.34, 0.56, 0.78];
    let parsed: Srgba4 = ron::from_str("(0.12, 0.34, 0.56, 0.78)")?;
    assert_eq!(
        parsed,
        Srgba4::new(input),
        "Srgba4 must deserialize through its private inner"
    );

    // (2) Deserialize + resolve, then read the resolved `ScreenColor` through
    // `Deref` (`*resolved.color`), proving the resolve-time `::new` wrap and the
    // `Deref` accessor are intact over the private field.
    let bg: BackgroundThemeSpec = ron::from_str("(color: (0.12, 0.34, 0.56, 0.78))")?;
    let resolved = bg.resolve();
    assert_eq!(
        *resolved.color,
        Color::srgba(0.12, 0.34, 0.56, 0.78),
        "ScreenColor must be readable through Deref after resolving the parsed spec"
    );

    Ok(())
}
