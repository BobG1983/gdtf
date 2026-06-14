//! Data-driven UI theme: the on-disk RON shape, its typed runtime form, and the
//! pure spec-to-resource resolution that bridges them.
//!
//! The theme is **data-driven** (ADR 0003): visual constants live in a loose
//! `assets/theme/*.ron` file, not in code. As of GTW-149 the theme is **nested**
//! into per-widget sub-themes rather than one flat record: a top-level
//! [`GdtfTheme`] holds a [`BackgroundTheme`], a [`PanelTheme`], a [`ButtonTheme`],
//! a [`TitleTheme`], and a [`TextTheme`], each painting one role of the UI.
//!
//! This module owns three layers, keeping the spec/runtime split per sub-theme:
//!
//! - [`GdtfThemeSpec`] (and a `*Spec` for each sub-theme) — the serde-`Deserialize`
//!   mirror of the on-disk RON. It holds wire-friendly shapes ([`Srgba4`] color
//!   quads, `String` font keys, bare px scalars) and nothing Bevy-asset-bound, so
//!   it deserializes with no `World`.
//! - [`GdtfTheme`] (and a runtime struct for each sub-theme) — the runtime
//!   [`Resource`]. Every field is a typed newtype over a resolved value (a
//!   [`Color`], a px scalar, a loaded [`Handle<Font>`]); it carries **no** raw
//!   `Color`/`f32`/`String` domain fields.
//! - [`GdtfThemeSpec::resolve`] — the pure bridge: it consumes a spec plus a
//!   **font resolver** closure and produces a [`GdtfTheme`]. It touches no `World`
//!   and no `AssetServer` directly (the resolver abstracts that), so it is
//!   unit-testable fully headless.
//!
//! ## Font overrides
//!
//! Only the **text-bearing** sub-themes ([`ButtonTheme`], [`TitleTheme`],
//! [`TextTheme`]) carry an optional per-sub-theme `font` override: in the spec it
//! is an `Option<String>` with `#[serde(default)]`, so an absent field means
//! "use the theme's [`default_font`](GdtfThemeSpec::default_font)". The
//! [`BackgroundTheme`] and [`PanelTheme`] render no text, so they carry **no**
//! font field at all. Resolution picks `font.as_deref().unwrap_or(&default_font)`
//! and feeds that key through the resolver, so each text-bearing runtime sub-theme
//! ends up with a concrete resolved [`Handle<Font>`].

use bevy::prelude::*;
use gdtf_assets::RonAsset;
use serde::Deserialize;

/// The full-screen backdrop fill color (the screen behind all UI).
///
/// Named [`ScreenColor`] rather than `BackgroundColor` to avoid clashing with
/// `bevy::ui::BackgroundColor`, the component `apply_theme` writes this into.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct ScreenColor(Color);

/// The resting fill color of a themed panel box.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct PanelColor(Color);

/// The resting fill color of a themed button.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct ButtonColor(Color);

/// The flat background fill of a disabled / non-interactive button.
///
/// A deliberately muted color so a disabled control reads as inert and visibly
/// distinct from the active button fill — an explicit, data-driven value, not a
/// computed alpha-dim.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct DisabledColor(Color);

/// The button fill shown while a themed button is hovered.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct HoverColor(Color);

/// The button fill shown while a themed button is pressed.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct PressedColor(Color);

/// Foreground color of themed text (labels, button captions, titles, rich text).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct TextColor(Color);

/// Color of a themed box's border stroke (a panel or a button border).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct BorderColor(Color);

/// Width of a themed box's border stroke, in logical pixels.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct BorderWidthPx(f32);

/// Corner radius of a themed box, in logical pixels.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct CornerRadiusPx(f32);

/// A single content-margin edge inset, in logical pixels.
///
/// One newtype shared by all four edges of [`ContentMargin`]: the four edges are
/// the same *kind* of value (a px inset), distinguished by their field, so they
/// share a type rather than each owning a near-identical one.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct MarginPx(f32);

/// Inner padding between a themed box's border and its content, per edge.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ContentMargin {
    /// Left edge inset.
    pub l: MarginPx,
    /// Right edge inset.
    pub r: MarginPx,
    /// Top edge inset.
    pub t: MarginPx,
    /// Bottom edge inset.
    pub b: MarginPx,
}

/// Text size for a themed text role, in typographic points.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct FontSizePt(f32);

/// Identifier of a theme font asset — a loose font's path under `assets/`.
///
/// On disk this is the path string (e.g. `"fonts/Alegreya-Variable.ttf"`); at
/// runtime each text-bearing sub-theme carries the resolved [`Handle<Font>`] for
/// its chosen font (its override, or the theme default).
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct FontKey(String);

/// A non-premultiplied sRGB color quad as it appears on disk: `(r, g, b, a)`,
/// each channel in `0.0..=1.0`.
///
/// We deserialize colors through this newtype rather than serde-on-[`Color`]
/// directly: the wire shape is a plain `[f32; 4]`, and resolution maps it into a
/// [`Color`] via [`Color::srgba`]. Keeping the on-disk type primitive-shaped
/// avoids coupling the RON schema to Bevy's internal color representation.
#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(transparent)]
pub struct Srgba4([f32; 4]);

impl Srgba4 {
    /// Resolve this on-disk color quad into a runtime sRGB [`Color`].
    const fn into_color(self) -> Color {
        let [r, g, b, a] = self.0;
        Color::srgba(r, g, b, a)
    }
}

/// The on-disk margin shape: four named edge insets in logical pixels.
///
/// Mirrors the nested `margin: (left:, right:, top:, bottom:)` RON form and
/// resolves into the runtime [`ContentMargin`].
#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct MarginSpec {
    /// Left edge inset, in logical pixels.
    pub left:   f32,
    /// Right edge inset, in logical pixels.
    pub right:  f32,
    /// Top edge inset, in logical pixels.
    pub top:    f32,
    /// Bottom edge inset, in logical pixels.
    pub bottom: f32,
}

impl MarginSpec {
    /// Resolve this on-disk margin into the runtime [`ContentMargin`].
    const fn resolve(self) -> ContentMargin {
        ContentMargin {
            l: MarginPx(self.left),
            r: MarginPx(self.right),
            t: MarginPx(self.top),
            b: MarginPx(self.bottom),
        }
    }
}

/// On-disk shape of the full-screen backdrop sub-theme.
#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct BackgroundThemeSpec {
    /// The backdrop fill color.
    pub color: Srgba4,
}

impl BackgroundThemeSpec {
    /// Resolve the backdrop sub-theme (no font, no resolver needed).
    const fn resolve(self) -> BackgroundTheme {
        BackgroundTheme {
            color: ScreenColor(self.color.into_color()),
        }
    }
}

/// Runtime backdrop sub-theme — the full-screen fill behind all UI.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BackgroundTheme {
    /// The backdrop fill color.
    pub color: ScreenColor,
}

/// On-disk shape of the panel-box sub-theme.
#[derive(Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct PanelThemeSpec {
    /// Panel fill color.
    pub color:            Srgba4,
    /// Panel border stroke color.
    pub border_color:     Srgba4,
    /// Panel border stroke width, in logical pixels.
    pub border_width_px:  f32,
    /// Panel corner radius, in logical pixels.
    pub corner_radius_px: f32,
    /// Panel inner content padding, per edge.
    pub margin:           MarginSpec,
}

impl PanelThemeSpec {
    /// Resolve the panel sub-theme (no font, no resolver needed).
    const fn resolve(self) -> PanelTheme {
        PanelTheme {
            color:            PanelColor(self.color.into_color()),
            border_color:     BorderColor(self.border_color.into_color()),
            border_width_px:  BorderWidthPx(self.border_width_px),
            corner_radius_px: CornerRadiusPx(self.corner_radius_px),
            margin:           self.margin.resolve(),
        }
    }
}

/// Runtime panel-box sub-theme — a box drawn around grouped UI.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PanelTheme {
    /// Panel fill color.
    pub color:            PanelColor,
    /// Panel border stroke color.
    pub border_color:     BorderColor,
    /// Panel border stroke width.
    pub border_width_px:  BorderWidthPx,
    /// Panel corner radius.
    pub corner_radius_px: CornerRadiusPx,
    /// Panel inner content padding, per edge.
    pub margin:           ContentMargin,
}

/// On-disk shape of the button sub-theme.
///
/// Text-bearing, so it carries an optional [`font`](Self::font) override
/// (`#[serde(default)]` — absent means "use the theme's `default_font`").
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct ButtonThemeSpec {
    /// Resting button fill color.
    pub color:            Srgba4,
    /// Disabled button fill color.
    pub disabled:         Srgba4,
    /// Hovered button fill color.
    pub hover:            Srgba4,
    /// Pressed button fill color.
    pub pressed:          Srgba4,
    /// Button caption text color.
    pub text_color:       Srgba4,
    /// Button caption text size, in typographic points.
    pub font_size_pt:     f32,
    /// Button border stroke color.
    pub border_color:     Srgba4,
    /// Button border stroke width, in logical pixels.
    pub border_width_px:  f32,
    /// Button corner radius, in logical pixels.
    pub corner_radius_px: f32,
    /// Button inner content padding, per edge.
    pub margin:           MarginSpec,
    /// Optional font override; absent means use the theme `default_font`.
    #[serde(default)]
    pub font:             Option<String>,
}

impl ButtonThemeSpec {
    /// Resolve the button sub-theme, threading the chosen font handle in.
    fn resolve(self, font: Handle<Font>) -> ButtonTheme {
        ButtonTheme {
            color: ButtonColor(self.color.into_color()),
            disabled: DisabledColor(self.disabled.into_color()),
            hover: HoverColor(self.hover.into_color()),
            pressed: PressedColor(self.pressed.into_color()),
            text_color: TextColor(self.text_color.into_color()),
            font_size_pt: FontSizePt(self.font_size_pt),
            border_color: BorderColor(self.border_color.into_color()),
            border_width_px: BorderWidthPx(self.border_width_px),
            corner_radius_px: CornerRadiusPx(self.corner_radius_px),
            margin: self.margin.resolve(),
            font,
        }
    }
}

/// Runtime button sub-theme — the box, state fills, and caption typography of a
/// themed button.
#[derive(Clone, PartialEq, Debug)]
pub struct ButtonTheme {
    /// Resting button fill color.
    pub color:            ButtonColor,
    /// Disabled button fill color.
    pub disabled:         DisabledColor,
    /// Hovered button fill color.
    pub hover:            HoverColor,
    /// Pressed button fill color.
    pub pressed:          PressedColor,
    /// Button caption text color.
    pub text_color:       TextColor,
    /// Button caption text size.
    pub font_size_pt:     FontSizePt,
    /// Button border stroke color.
    pub border_color:     BorderColor,
    /// Button border stroke width.
    pub border_width_px:  BorderWidthPx,
    /// Button corner radius.
    pub corner_radius_px: CornerRadiusPx,
    /// Button inner content padding, per edge.
    pub margin:           ContentMargin,
    /// The resolved caption font handle (the override, or the theme default).
    pub font:             Handle<Font>,
}

/// On-disk shape of the title sub-theme.
///
/// Text-bearing, so it carries an optional [`font`](Self::font) override
/// (`#[serde(default)]` — absent means "use the theme's `default_font`").
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct TitleThemeSpec {
    /// Title text color.
    pub text_color:   Srgba4,
    /// Title text size, in typographic points.
    pub font_size_pt: f32,
    /// Optional font override; absent means use the theme `default_font`.
    #[serde(default)]
    pub font:         Option<String>,
}

impl TitleThemeSpec {
    /// Resolve the title sub-theme, threading the chosen font handle in.
    fn resolve(self, font: Handle<Font>) -> TitleTheme {
        TitleTheme {
            text_color: TextColor(self.text_color.into_color()),
            font_size_pt: FontSizePt(self.font_size_pt),
            font,
        }
    }
}

/// Runtime title sub-theme — the typography of a heading.
#[derive(Clone, PartialEq, Debug)]
pub struct TitleTheme {
    /// Title text color.
    pub text_color:   TextColor,
    /// Title text size.
    pub font_size_pt: FontSizePt,
    /// The resolved title font handle (the override, or the theme default).
    pub font:         Handle<Font>,
}

/// On-disk shape of the body-text sub-theme.
///
/// Text-bearing, so it carries an optional [`font`](Self::font) override
/// (`#[serde(default)]` — absent means "use the theme's `default_font`").
#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct TextThemeSpec {
    /// Body text color.
    pub text_color:   Srgba4,
    /// Body text size, in typographic points.
    pub font_size_pt: f32,
    /// Optional font override; absent means use the theme `default_font`.
    #[serde(default)]
    pub font:         Option<String>,
}

impl TextThemeSpec {
    /// Resolve the body-text sub-theme, threading the chosen font handle in.
    fn resolve(self, font: Handle<Font>) -> TextTheme {
        TextTheme {
            text_color: TextColor(self.text_color.into_color()),
            font_size_pt: FontSizePt(self.font_size_pt),
            font,
        }
    }
}

/// Runtime body-text sub-theme — the typography of ordinary labels / rich text.
#[derive(Clone, PartialEq, Debug)]
pub struct TextTheme {
    /// Body text color.
    pub text_color:   TextColor,
    /// Body text size.
    pub font_size_pt: FontSizePt,
    /// The resolved body-text font handle (the override, or the theme default).
    pub font:         Handle<Font>,
}

/// The on-disk RON shape of a GDTF theme.
///
/// The deserialization mirror of `assets/theme/*.ron`: a `default_font` path plus
/// one nested `*Spec` per widget role. It carries nothing Bevy-asset-bound, so
/// `ron::from_str` into it needs no `World` and no `AssetServer`.
///
/// Resolve it into the runtime [`GdtfTheme`] with [`GdtfThemeSpec::resolve`].
///
/// It derives [`TypePath`] so it can be the payload of a
/// `RonAsset<GdtfThemeSpec>` (the generic GTW-136 loader requires `T: TypePath`):
/// the `Load` scene loads `theme/grimdark.ron` as that asset, then resolves the
/// deserialized spec into a [`GdtfTheme`].
#[derive(Deserialize, TypePath, Clone, PartialEq, Debug)]
pub struct GdtfThemeSpec {
    /// The font used by any text-bearing sub-theme that does not override it.
    pub default_font: String,
    /// The full-screen backdrop sub-theme.
    pub background:   BackgroundThemeSpec,
    /// The panel-box sub-theme.
    pub panel:        PanelThemeSpec,
    /// The button sub-theme.
    pub button:       ButtonThemeSpec,
    /// The title / heading sub-theme.
    pub title:        TitleThemeSpec,
    /// The body-text sub-theme.
    pub text:         TextThemeSpec,
}

impl GdtfThemeSpec {
    /// Resolve this on-disk spec into the runtime [`GdtfTheme`] resource.
    ///
    /// Pure: it consumes the spec and a `resolve_font` closure that maps a loose
    /// font-path key to a [`Handle<Font>`], and builds the typed runtime values.
    /// It accesses no `World` and no `AssetServer` itself — the closure abstracts
    /// font loading — so it is fully unit-testable headless. For each text-bearing
    /// sub-theme it picks the sub-theme's `font` override if present, else the
    /// top-level [`default_font`](Self::default_font), and resolves that key.
    ///
    /// In the running app the closure is `|key| asset_server.load::<Font>(key)`
    /// (idempotent — it returns the already-loaded handle for a preloaded font);
    /// in the error-path fallback it returns `Handle::<Font>::default()`.
    #[must_use]
    pub fn resolve(self, resolve_font: impl Fn(&str) -> Handle<Font>) -> GdtfTheme {
        let button_font = resolve_font(self.button.font.as_deref().unwrap_or(&self.default_font));
        let title_font = resolve_font(self.title.font.as_deref().unwrap_or(&self.default_font));
        let text_font = resolve_font(self.text.font.as_deref().unwrap_or(&self.default_font));

        GdtfTheme {
            default_font: FontKey(self.default_font),
            background:   self.background.resolve(),
            panel:        self.panel.resolve(),
            button:       self.button.resolve(button_font),
            title:        self.title.resolve(title_font),
            text:         self.text.resolve(text_font),
        }
    }
}

/// The shipped grimdark theme RON, embedded at compile time from the repo's
/// loose asset.
///
/// This is the **same authoritative file** the success path loads through the
/// `AssetServer` (`assets/theme/grimdark.ron`) — embedding it here lets the
/// error-path fallback ([`default_theme`]) reuse the authoritative grimdark
/// values rather than a divergent hand-written palette, so the safety-net looks
/// like the real theme. It is *not* an `embedded_asset!` (ADR 0003 bans those):
/// it is a plain `&str` parsed in-process, only ever reached when the loose
/// load failed.
const SHIPPED_GRIMDARK_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/theme/grimdark.ron"
));

/// The last-resort, code-level default [`GdtfTheme`].
///
/// **ADR-0003 sanctioned exception (GTW-143):** ADR 0003 clause 4 forbids
/// hardcoding theme *values* as `const Color`s — `assets/theme/grimdark.ron`
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
/// **ADR-0003 sanctioned exception (GTW-143):** these are the only hardcoded
/// theme values in the codebase, and they exist solely so the error path can
/// always hand back *some* legible theme. The values mirror the shipped grimdark
/// palette so the unreachable-in-practice fallback still reads as the intended
/// look. The fidelity gate must not flag this as a hardcoded-color violation —
/// it is the explicitly-blessed last resort, not the styling source of truth.
fn const_fallback_theme() -> GdtfTheme {
    let font = Handle::<Font>::default();
    let margin = ContentMargin {
        l: MarginPx(12.0),
        r: MarginPx(12.0),
        t: MarginPx(6.0),
        b: MarginPx(6.0),
    };
    GdtfTheme {
        default_font: FontKey(String::from("fonts/Alegreya-Variable.ttf")),
        background:   BackgroundTheme {
            color: ScreenColor(Color::srgba(0.05, 0.05, 0.06, 1.0)),
        },
        panel:        PanelTheme {
            color: PanelColor(Color::srgba(0.16, 0.16, 0.18, 0.55)),
            border_color: BorderColor(Color::srgba(0.20, 0.20, 0.24, 1.0)),
            border_width_px: BorderWidthPx(2.0),
            corner_radius_px: CornerRadiusPx(5.0),
            margin,
        },
        button:       ButtonTheme {
            color: ButtonColor(Color::srgba(0.12, 0.12, 0.15, 0.96)),
            disabled: DisabledColor(Color::srgba(0.08, 0.08, 0.10, 0.55)),
            hover: HoverColor(Color::srgba(0.80, 0.16, 0.19, 0.96)),
            pressed: PressedColor(Color::srgba(0.10, 0.10, 0.12, 0.96)),
            text_color: TextColor(Color::srgba(0.84, 0.80, 0.73, 1.0)),
            font_size_pt: FontSizePt(18.0),
            border_color: BorderColor(Color::srgba(0.20, 0.20, 0.24, 1.0)),
            border_width_px: BorderWidthPx(2.0),
            corner_radius_px: CornerRadiusPx(5.0),
            margin,
            font: font.clone(),
        },
        title:        TitleTheme {
            text_color:   TextColor(Color::srgba(0.84, 0.80, 0.73, 1.0)),
            font_size_pt: FontSizePt(36.0),
            font:         font.clone(),
        },
        text:         TextTheme {
            text_color: TextColor(Color::srgba(0.84, 0.80, 0.73, 1.0)),
            font_size_pt: FontSizePt(18.0),
            font,
        },
    }
}

/// The resolved, runtime GDTF UI theme — a nested record of per-widget sub-themes.
///
/// Every field is a typed value (a sub-theme of newtypes over resolved
/// [`Color`]s / px scalars / loaded [`Handle<Font>`]s, or the
/// [`default_font`](Self::default_font) path) — there are **no** raw
/// `Color`/`f32`/`String` domain fields. Built only via
/// [`GdtfThemeSpec::resolve`].
///
/// This resource is **not** inserted at startup. The `Load` scene populates it
/// during `AppState::Load`; readers must guard for its absence per the project's
/// state-scoped-resource convention. `Themed`/`apply_theme` consume it.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct GdtfTheme {
    /// The loose font-path key used by any text-bearing sub-theme that does not
    /// override its own font.
    pub default_font: FontKey,
    /// The full-screen backdrop sub-theme.
    pub background:   BackgroundTheme,
    /// The panel-box sub-theme.
    pub panel:        PanelTheme,
    /// The button sub-theme.
    pub button:       ButtonTheme,
    /// The title / heading sub-theme.
    pub title:        TitleTheme,
    /// The body-text sub-theme.
    pub text:         TextTheme,
}

/// The handle to the **active** theme RON asset (`theme/grimdark.ron`), held as a
/// persistent resource so the live-retheme layer can react to its changes.
///
/// A named [`Deref`] newtype over the `RonAsset<GdtfThemeSpec>` handle rather than
/// a bare `Handle` (no-bare-types rule): the name says "the theme asset currently
/// driving [`GdtfTheme`]". The retheme system
/// ([`redrive_theme_on_asset_event`](crate::retheme::redrive_theme_on_asset_event))
/// filters incoming [`AssetEvent`](bevy::asset::AssetEvent) ids against this
/// handle's id, ignoring events for any other asset.
///
/// Inserted alongside [`GdtfTheme`] during `AppState::Load` (on **both** the
/// success and the const-fallback paths — the handle is valid even when the load
/// failed, so a later file-watcher reload can recover) and, like [`GdtfTheme`], it
/// **persists** past `OnExit(Load)`. Holding the handle keeps a **strong**
/// reference to the asset so it stays loaded for that future watcher.
///
/// It is **not** inserted at startup; readers guard for its absence per the
/// state-scoped-resource convention (bevy-traps rule 1).
#[derive(Resource, Deref, Clone, Debug)]
pub struct ActiveThemeHandle(pub Handle<RonAsset<GdtfThemeSpec>>);

#[cfg(test)]
mod tests {
    use super::*;

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
}
