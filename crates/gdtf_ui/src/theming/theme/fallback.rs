//! The error-path safety-net: the embedded shipped grimdark RON and the
//! [`default_theme`] / [`const_fallback_theme`] the `Load` scene falls back to
//! when the loose theme asset (or its fonts) fails to load.

use bevy::prelude::*;

use super::{
    newtypes::{
        ActiveColor, BorderColor, BorderWidthVw, ButtonColor, ContentMargin, CornerRadiusVw,
        DisabledColor, FontKey, FontSizePt, HoverColor, MarginVh, MarginVw, PanelColor,
        PressedColor, ScreenColor, TextColor,
    },
    runtime::{BackgroundTheme, ButtonTheme, GdtfTheme, PanelTheme, TextTheme, TitleTheme},
    spec::GdtfThemeSpec,
};

/// The shipped grimdark theme RON, embedded at compile time from the repo's
/// loose asset.
///
/// This is the **same authoritative file** the success path loads through the
/// `AssetServer` (`assets/core_tuning/ui_theme.tuning.ron`) — embedding it here lets the
/// error-path fallback ([`default_theme`]) reuse the authoritative grimdark
/// values rather than a divergent hand-written palette, so the safety-net looks
/// like the real theme. It is *not* an `embedded_asset!` (ADR 0003 bans those):
/// it is a plain `&str` parsed in-process, only ever reached when the loose
/// load failed.
pub(super) const SHIPPED_GRIMDARK_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/core_tuning/ui_theme.tuning.ron"
));

/// The last-resort, code-level default [`GdtfTheme`].
///
/// **ADR-0003 exception (GTW-143):** ADR 0003 clause 4 forbids
/// hardcoding theme *values* as `const Color`s — `assets/core_tuning/ui_theme.tuning.ron`
/// remains the single styling source of truth on the success path. This function
/// is the deliberately-narrow exception: it is the error-path safety-net the
/// `Load` scene falls back to **only** when the loose theme RON (or its fonts)
/// fails to load, so the app never leaves `Load` without a `GdtfTheme` and never
/// hangs on a bad asset.
///
/// It first reparses the embedded, test-verified [`SHIPPED_GRIMDARK_RON`] (the
/// authoritative grimdark values, not a fresh palette) and resolves it with a
/// closure returning the default [`Handle<Font>`] (the fallback has no
/// `AssetServer` to load fonts). That parse cannot realistically fail — the same
/// bytes are asserted to deserialize by this module's tests — but to honour the
/// no-`unwrap`/`expect`/`panic` rule it falls through, on a parse error, to
/// [`const_fallback_theme`]: the genuinely hardcoded last line of defence.
#[must_use]
pub fn default_theme() -> GdtfTheme {
    match ron::from_str::<GdtfThemeSpec>(SHIPPED_GRIMDARK_RON) {
        Ok(spec) => spec.resolve(|_| Handle::<Font>::default()),
        Err(_) => const_fallback_theme(),
    }
}

/// The hardcoded final safety-net [`GdtfTheme`], reached only if even the
/// embedded [`SHIPPED_GRIMDARK_RON`] fails to parse.
///
/// **ADR-0003 exception (GTW-143):** these are the only hardcoded
/// theme values in the codebase, and they exist solely so the error path can
/// always hand back *some* legible theme. The values mirror the shipped grimdark
/// palette so the unreachable-in-practice fallback still reads as the intended
/// look. The fidelity gate must not flag this as a hardcoded-color violation —
/// it is the explicitly-blessed last resort, not the styling source of truth.
pub(super) fn const_fallback_theme() -> GdtfTheme {
    let font = Handle::<Font>::default();
    // GTW-296: relative-length insets calibrated to the 1280x720 reference window —
    // L/R as `Vw` (12px / 1280 = 0.9375), T/B as `Vh` (6px / 720 = 0.83333) — mirroring
    // the shipped `ui_theme.tuning.ron` margin so the const safety-net matches the real theme.
    let margin = ContentMargin {
        l: MarginVw::new(0.9375),
        r: MarginVw::new(0.9375),
        t: MarginVh::new(0.83333),
        b: MarginVh::new(0.83333),
    };
    GdtfTheme {
        default_font: FontKey::new("fonts/Alegreya-Variable.ttf"),
        background:   BackgroundTheme {
            color: ScreenColor::new(Color::srgba(0.05, 0.05, 0.06, 1.0)),
        },
        panel:        PanelTheme {
            color: PanelColor::new(Color::srgba(0.16, 0.16, 0.18, 0.55)),
            border_color: BorderColor::new(Color::srgba(0.20, 0.20, 0.24, 1.0)),
            // GTW-296: Vw fractions calibrated to 1280px width (2px / 1280, 5px / 1280).
            border_width: BorderWidthVw::new(0.15625),
            corner_radius: CornerRadiusVw::new(0.390_625),
            margin,
        },
        button:       ButtonTheme {
            color: ButtonColor::new(Color::srgba(0.12, 0.12, 0.15, 0.96)),
            disabled: DisabledColor::new(Color::srgba(0.08, 0.08, 0.10, 0.55)),
            active: ActiveColor::new(Color::srgba(0.45, 0.62, 0.30, 0.96)),
            hover: HoverColor::new(Color::srgba(0.80, 0.16, 0.19, 0.96)),
            pressed: PressedColor::new(Color::srgba(0.10, 0.10, 0.12, 0.96)),
            text_color: TextColor::new(Color::srgba(0.84, 0.80, 0.73, 1.0)),
            font_size_pt: FontSizePt::new(18.0),
            border_color: BorderColor::new(Color::srgba(0.20, 0.20, 0.24, 1.0)),
            // GTW-296: Vw fractions calibrated to 1280px width (2px / 1280, 5px / 1280).
            border_width: BorderWidthVw::new(0.15625),
            corner_radius: CornerRadiusVw::new(0.390_625),
            margin,
            font: font.clone(),
        },
        title:        TitleTheme {
            text_color:   TextColor::new(Color::srgba(0.84, 0.80, 0.73, 1.0)),
            font_size_pt: FontSizePt::new(36.0),
            font:         font.clone(),
        },
        text:         TextTheme {
            text_color: TextColor::new(Color::srgba(0.84, 0.80, 0.73, 1.0)),
            font_size_pt: FontSizePt::new(18.0),
            font,
        },
    }
}
