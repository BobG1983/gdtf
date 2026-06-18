//! [`fit_weapon_panel`] — post-theme refinement of the weapon cluster's fit (GTW-298,
//! screenshot review 2026-06-18).
//!
//! Two small visual tweaks that must survive the central `gdtf_ui::apply_theme` pass, which
//! re-applies the THEME-owned padding (`theme.panel.margin`) onto every `Themed(Panel)` Node and
//! the THEME-owned font size (`theme.text` / `theme.button` `font_size_pt`) onto every
//! `Themed(Text)` / `Themed(ButtonText)` label every time it runs. Changing those theme scalars
//! would resize / re-pad EVERY panel + button app-wide, so this system instead post-processes ONLY
//! the weapon-cluster widgets, `.after(UiSystems::ApplyTheme)`:
//!
//! 1. **Item panel horizontal padding** — the [`WeaponItemPanel`] is a `Themed(Panel)`, so its
//!    L/R padding is the theme's 12 px panel margin; that inset made the two item buttons too
//!    narrow within the item column. This tightens its left/right padding to [`ITEM_PAD_VW`] so
//!    the item buttons fill more of the column (the top/bottom padding is left at the theme value).
//! 2. **Weapon name / magazine + item / reload label font** — those labels (`Themed(Text)` /
//!    `Themed(ButtonText)`) render at the theme's 18 pt, at which a name like "autogun" clipped to
//!    "autogur" in the narrow Combined panel. This shrinks them to [`WEAPON_LABEL_PT`] so the
//!    captions fit without clipping.
//!
//! Each write is gated on a real change (change-detection hygiene), so once the freshly-spawned
//! widgets settle the system is a no-op; a theme repaint that re-applies the theme padding / font
//! is re-corrected the next frame (the system runs every frame under the live-battle gate). It
//! does NOT touch the shared `gdtf_ui` widgets or the theme — purely a local fit pass over the
//! cluster's own markers. UI/view only; no [`Commands`], no `&mut World` (`bevy-traps.md` #7).

use bevy::{
    prelude::*,
    text::TextFont,
    ui::{Node, Val},
};

use crate::scenes::running::game::battlescape::weapon_panel::components::{
    ReloadButton, WeaponItemButton, WeaponItemPanel, WeaponMagazineText, WeaponNameText,
};

/// The tightened LEFT/RIGHT padding of the Item Panel, as a fraction of the viewport WIDTH
/// ([`Val::Vw`](bevy::ui::Val::Vw)), replacing the theme's panel margin so the two item buttons
/// are wider within the item column.
///
/// A named newtype over the padding rather than a bare `f32` (no-bare-types rule): it is layout
/// spacing, the `ModeGapVw` / `GAP` plumbing-newtype precedent. Horizontal padding, so the unit
/// is `Vw` (`ui-responsive-not-px`). Applied per-panel here so the theme (and every other panel)
/// is untouched.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct ItemPadVw(f32);

/// The Item Panel's tightened horizontal padding: 0.3125 vw (4 px at the 1280-wide reference
/// window — a tight inset).
const ITEM_PAD_VW: ItemPadVw = ItemPadVw(0.3125);

/// The font size (pt) the weapon name / magazine + item / reload labels are shrunk to so a name
/// like "autogun" fits the narrow Combined panel without clipping.
///
/// A named newtype over the size rather than a bare `f32` (no-bare-types rule): a presentation
/// font size, the `ControlLabelPt` precedent. Smaller than the theme's 18 pt; applied per-label
/// here so the theme (and every other label) is untouched.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct WeaponLabelPt(f32);

/// The weapon name / magazine + item / reload label size: 14 pt — small enough that a typical
/// weapon name fits the Combined panel's text column without clipping, while staying legible.
const WEAPON_LABEL_PT: WeaponLabelPt = WeaponLabelPt(14.0);

/// Query FILTER selecting the weapon-cluster widgets whose label font should shrink — the weapon
/// name / magazine lines (the marker sits on the `Text` node itself) and the item / reload buttons
/// (the marker sits on the button ROOT; its label is a `Text` CHILD). An `Or` over their markers
/// so one query reaches every cluster label owner (GTW-298).
///
/// Aliased so the [`fit_weapon_panel`] owner query type stays legible (clippy `type_complexity`).
type WeaponLabelOwner = Or<(
    With<WeaponNameText>,
    With<WeaponMagazineText>,
    With<WeaponItemButton>,
    With<ReloadButton>,
)>;

/// Post-theme fit pass over the weapon cluster (GTW-298): tightens the Item Panel's horizontal
/// padding and shrinks the cluster's labels so captions fit without clipping.
///
/// It runs `.after(UiSystems::ApplyTheme)` under the weapon-panel plugin's live-battle gate. For
/// the Item Panel it overrides the theme-re-applied left/right padding to [`ITEM_PAD_VW`]. For each
/// label owner ([`WeaponLabelOwner`]) it shrinks the label `TextFont` to [`WEAPON_LABEL_PT`]: a
/// name / magazine marker sits on the `Text` node itself, while an item / reload marker sits on the
/// button root whose label is a `Text` child — so it sets the font on the owner entity AND on each
/// of its children (whichever resolves to a label `Text`). Every write is gated on a real change
/// (change-detection hygiene), so it settles in the first frame after spawn and is a no-op
/// thereafter; a theme repaint is re-corrected the next frame.
///
/// Param-only (`bevy-traps.md` #7): a `&mut Node` write query over the item panel, a read-only
/// owner query (the label markers + their children), and a `&mut TextFont` write query over the
/// label `Text` nodes — no [`Commands`], no `&mut World`. The [`Node`] query and the [`TextFont`]
/// query are disjoint (a panel [`Node`] is not a label `Text`), so they never conflict.
pub(in crate::scenes::running::game::battlescape) fn fit_weapon_panel(
    mut item_panels: Query<&mut Node, With<WeaponItemPanel>>,
    owners: Query<(Entity, Option<&Children>), WeaponLabelOwner>,
    mut fonts: Query<&mut TextFont, With<Text>>,
) {
    // 1. Tighten the Item Panel's horizontal padding so the item buttons are wider (the theme
    //    re-applies its 12 px panel margin every repaint; we override only L/R here).
    for mut node in &mut item_panels {
        if node.padding.left != Val::Vw(*ITEM_PAD_VW) {
            node.padding.left = Val::Vw(*ITEM_PAD_VW);
        }
        if node.padding.right != Val::Vw(*ITEM_PAD_VW) {
            node.padding.right = Val::Vw(*ITEM_PAD_VW);
        }
    }

    // 2. Shrink the cluster labels so captions fit without clipping. A name / magazine marker is
    //    on the `Text` node itself (the owner resolves in `fonts`); an item / reload marker is on
    //    the button root whose label `Text` is a child (the children resolve in `fonts`). Setting
    //    the font on BOTH the owner and its children covers both shapes — the font query only
    //    matches a `Text`, so a non-`Text` owner / child is skipped.
    for (owner, children) in &owners {
        set_label_font(&mut fonts, owner);
        let Some(children) = children else {
            continue;
        };
        for &child in children {
            set_label_font(&mut fonts, child);
        }
    }
}

/// Sets one label `Text`'s font to [`WEAPON_LABEL_PT`], writing only on a real change; a no-op when
/// `label` is not a label `Text` (the font query is filtered `With<Text>`). Shared by
/// [`fit_weapon_panel`] across the owner entities and their children.
fn set_label_font(fonts: &mut Query<&mut TextFont, With<Text>>, label: Entity) {
    let Ok(mut font) = fonts.get_mut(label) else {
        return;
    };
    if font.font_size != *WEAPON_LABEL_PT {
        font.font_size = *WEAPON_LABEL_PT;
    }
}
