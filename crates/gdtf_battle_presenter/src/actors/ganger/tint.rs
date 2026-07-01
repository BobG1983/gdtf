//! The per-ganger sprite tints: faction colour, the Downed grey-out, and the
//! stance/aiming modulation.

use bevy::prelude::*;
use gdtf_battle_sim::{Aiming, Faction, LifeState, Stance, StanceKind};

/// The faction (gang) tint applied to a ganger sprite so the two gangs read as two
/// colours at a glance — the "faction-coloured" signal layered on top of the distinct
/// per-faction actor tile.
///
/// Faction 0 draws a cool blue tint, faction 1 a warm red tint (any other gang, none in
/// the two-gang design, reuses faction 0's). [`Color`] is a framework type, so the tint
/// itself is framework plumbing; this fn is the per-faction CHOICE. A [`Downed`](LifeState::Downed)
/// ganger overrides this with [`downed_tint`].
#[must_use]
pub(super) fn faction_tint(faction: Faction) -> Color {
    match *faction {
        1 => Color::srgb(1.0, 0.55, 0.5),
        _ => Color::srgb(0.55, 0.7, 1.0),
    }
}

/// The grey-out tint a [`LifeState::Downed`] ganger draws with — a desaturated, dimmed
/// overlay so a downed body reads as out of the fight while still on the field.
///
/// The documented Downed delta the AC asserts: a distinct darker / desaturated tint
/// from the live faction tint. [`Color`] is framework plumbing; this fn is the CHOICE.
#[must_use]
const fn downed_tint() -> Color {
    Color::srgb(0.4, 0.4, 0.45)
}

/// The tint a ganger sprite draws with given its faction and life state.
///
/// A live ([`LifeState::Alive`]) ganger draws its [`faction_tint`]; a [`Downed`](LifeState::Downed)
/// ganger draws the [`downed_tint`] (a [`Dead`](LifeState::Dead) ganger has no sprite — it is
/// despawned). The aiming delta is layered separately in
/// [`reframe_ganger_sprites`](super::reframe_ganger_sprites).
#[must_use]
pub(super) fn ganger_tint(faction: Faction, life: LifeState) -> Color {
    match life {
        LifeState::Downed => downed_tint(),
        // Dead has no sprite (despawned); treat it as the live tint for completeness.
        LifeState::Alive | LifeState::Dead => faction_tint(faction),
    }
}

/// The tint a ganger sprite draws with given its faction, life state, stance, aiming
/// flag, and SUPPRESSED flag — the combined re-tint
/// [`reframe_ganger_sprites`](super::reframe_ganger_sprites) applies.
///
/// Starts from [`ganger_tint`] (the faction / Downed base), then layers the stance +
/// aiming deltas: a [`Prone`](gdtf_battle_sim::StanceKind::Prone) ganger dims (a
/// flattened, low silhouette), and an aiming ganger brightens (reads "ready to fire").
/// The deltas only apply to a live ganger — a Downed body keeps its grey-out, undimmed
/// by stance / aim.
///
/// A SUPPRESSED ganger (GTW-526 C8: it carries a [`Suppressed`](gdtf_battle_sim::Suppressed)
/// component) is additionally DESATURATED toward grey AND darkened, so a pinned-down ganger
/// reads distinctly at a glance — the colour drains from a suppressed unit (it has lost its
/// nerve and cannot reaction-fire). This is applied LAST, on top of the stance/aim value shift,
/// and pulls the faction hue toward the neutral grey rather than merely scaling it, so a
/// suppressed ganger is discriminable from a merely-prone or Downed one. A ganger that is NO
/// LONGER suppressed passes `suppressed = false` and returns to the ordinary faction tint.
/// [`Color`] is framework plumbing; this fn is the CHOICE the AC asserts.
#[must_use]
pub(super) fn stance_aiming_tint(
    faction: Faction,
    life: LifeState,
    stance: Stance,
    aiming: Aiming,
    suppressed: bool,
) -> Color {
    let base = ganger_tint(faction, life);
    // Downed keeps its grey-out — stance / aim do not modulate an out-of-fight body.
    if matches!(life, LifeState::Downed) {
        return base;
    }
    // A prone ganger dims; an aiming ganger brightens. Multiplicative on the linear
    // colour so the faction hue is preserved, only the value shifts.
    let stance_scale = match *stance {
        StanceKind::Prone => 0.7,
        StanceKind::Standing | StanceKind::Crouching => 1.0,
    };
    let aim_scale = if *aiming { 1.2 } else { 1.0 };
    let factor = stance_scale * aim_scale;
    let linear = base.to_linear();
    let value_shifted = Color::linear_rgba(
        linear.red * factor,
        linear.green * factor,
        linear.blue * factor,
        linear.alpha,
    );
    if suppressed {
        suppressed_tint(value_shifted)
    } else {
        value_shifted
    }
}

/// Drain the colour out of a suppressed ganger's tint — desaturate it toward grey and darken
/// it (GTW-526 C8), so a pinned-down ganger reads as distinctly "washed out" from a live one.
///
/// Mixes the input tint toward its own grey (its unweighted mean value) by
/// [`SUPPRESSED_DESATURATION`] and then scales the result by [`SUPPRESSED_DARKEN`]: the hue
/// drains while the whole swatch dims, a combination no stance / aim / Downed state produces,
/// so the suppressed look is discriminable. The alpha is preserved. [`Color`] is framework
/// plumbing; this fn is the CHOICE.
#[must_use]
fn suppressed_tint(tint: Color) -> Color {
    /// How far a suppressed ganger's tint is pulled toward neutral grey (0 = no change, 1 =
    /// fully grey) — the colour-drain that reads as "lost its nerve".
    const SUPPRESSED_DESATURATION: f32 = 0.6;
    /// The value scale a suppressed ganger's (desaturated) tint is darkened by — a pinned
    /// unit also dims, distinct from the aiming brighten / prone dim.
    const SUPPRESSED_DARKEN: f32 = 0.75;

    let linear = tint.to_linear();
    // The grey the swatch desaturates toward: its own unweighted mean value, so a bright
    // faction tint greys to a bright grey and a dim one to a dim grey (a proportional drain).
    let grey = (linear.red + linear.green + linear.blue) / 3.0;
    let mix = |channel: f32| {
        channel.mul_add(
            1.0 - SUPPRESSED_DESATURATION,
            grey * SUPPRESSED_DESATURATION,
        ) * SUPPRESSED_DARKEN
    };
    Color::linear_rgba(
        mix(linear.red),
        mix(linear.green),
        mix(linear.blue),
        linear.alpha,
    )
}
