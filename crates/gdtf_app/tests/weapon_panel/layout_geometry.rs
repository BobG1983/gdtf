//! GTW-733 — layout-geometry regression: the LONGEST shipped weapon name renders in the
//! Combined Weapon Panel without the Reload button sharing its pixels.
//!
//! Unlike the rest of this suite (declared-`Node`-field assertions against a `MinimalPlugins`
//! app), this drives the [`real_layout_harness`](super::real_layout_harness) — a REAL `bevy_ui`
//! layout pass, against a REAL window size and a REAL loaded font — so the assertions below read
//! ACTUAL computed pixel geometry ([`ComputedNode`] + [`UiGlobalTransform`]) and an ACTUAL
//! measured glyph width ([`TextLayoutInfo`]), not just the `Val::Percent`/`Val::Vw` the panel
//! DECLARES. That distinction is the whole bug: the old layout's DECLARED split (a 3/4-width text
//! column beside a 1/4-width Reload cell) looked fine on paper, but a real font rendering a real
//! long identifier blew past its declared column.

use std::path::PathBuf;

use bevy::{
    ecs::{entity::Entity, query::With},
    math::Vec2,
    prelude::*,
    text::TextLayoutInfo,
    ui::{ComputedNode, UiGlobalTransform},
};
use gdtf_app::test_support::{ReloadButton, WeaponContent, WeaponNameText};
use gdtf_battle_sim::{
    magazine::{Magazine, ReloadTu},
    weapon::MagazineSize,
};

use super::{
    harness::{spawn_armed_and_select, weapon_kit},
    real_layout_harness::real_layout_battle_running_app,
};

/// The `.weapon.ron` extension every shipped RANGED weapon file carries (the
/// `WeaponsFamily::EXTENSION` this content family stem-keys against — see
/// `gdtf_content_families::weapons`).
const RANGED_WEAPON_EXTENSION: &str = ".weapon.ron";

/// A small epsilon (physical px): sub-pixel rounding between the text measure pass and the
/// `bevy_ui` layout pass.
const EPSILON_PX: f32 = 1.0;

/// Reads the REAL shipped ranged-weapon `.ron` files under `assets/content/weapons/ranged/` and
/// returns the LONGEST file stem (the [`WeaponName`](gdtf_battle_sim::weapon::WeaponName) each
/// resolves to — see `gdtf_content_families::weapons::WeaponsFamily::insert_member`).
/// Data-driven (the ticket's explicit requirement): it reads the actual directory rather than
/// naming a weapon by a hardcoded literal, so a future longer-named weapon file automatically
/// becomes the one this test exercises.
fn longest_shipped_ranged_weapon_name() -> String {
    let weapons_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
        .join("content")
        .join("weapons")
        .join("ranged");
    let mut longest = String::new();
    let Ok(entries) = std::fs::read_dir(&weapons_dir) else {
        return longest;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(stem) = file_name.strip_suffix(RANGED_WEAPON_EXTENSION) else {
            continue;
        };
        if stem.len() > longest.len() {
            stem.clone_into(&mut longest);
        }
    }
    longest
}

/// An axis-aligned rect built from a UI node's REAL computed geometry — its
/// [`ComputedNode::size`] (physical px) centered on its [`UiGlobalTransform`] translation
/// (physical px, per `bevy_ui`'s own over-UI hit-test — both are physical, matching units).
struct PixelRect {
    min: Vec2,
    max: Vec2,
}

impl PixelRect {
    fn from_node(node: &ComputedNode, transform: &UiGlobalTransform) -> Self {
        let size = node.size();
        let center = transform.translation;
        Self {
            min: center - size / 2.0,
            max: center + size / 2.0,
        }
    }

    /// Whether `self` and `other` share ANY pixel — true axis-aligned rect intersection (both
    /// axes overlap), not merely a shared edge/corner (touching bounds is NOT overlap).
    fn overlaps(&self, other: &Self) -> bool {
        self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
    }
}

/// The single entity carrying marker `M`'s [`ComputedNode`] + [`UiGlobalTransform`], or `None` if
/// not exactly one exists.
fn pixel_rect_of<M: Component>(app: &mut App) -> Option<PixelRect> {
    let mut query = app
        .world_mut()
        .query_filtered::<(&ComputedNode, &UiGlobalTransform), With<M>>();
    let mut results: Vec<PixelRect> = query
        .iter(app.world())
        .map(|(node, transform)| PixelRect::from_node(node, transform))
        .collect();
    if results.len() == 1 {
        results.pop()
    } else {
        None
    }
}

/// The single [`WeaponNameText`] entity, or `None` if not exactly one exists.
fn weapon_name_text_entity(app: &mut App) -> Option<Entity> {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, With<WeaponNameText>>();
    let mut results: Vec<Entity> = query.iter(app.world()).collect();
    if results.len() == 1 {
        results.pop()
    } else {
        None
    }
}

/// GTW-733 — the LONGEST shipped ranged weapon name renders in the weapon panel with its name
/// label and the Reload button occupying DISJOINT pixel rects, and the name's REAL measured
/// glyph width fits inside its own text block (no silent off-box overflow). Pin-discriminating:
/// reverting the panel to the pre-GTW-733 layout (Reload sharing the name's row) restores the
/// overlap this test would catch, and shrinking the text block back under the measured glyph
/// width would fail the fit assert.
#[test]
fn longest_shipped_weapon_name_does_not_overlap_reload_button() {
    let longest_name = longest_shipped_ranged_weapon_name();
    assert!(
        !longest_name.is_empty(),
        "must find at least one shipped ranged weapon .ron under assets/content/weapons/ranged/",
    );

    let app_opt = real_layout_battle_running_app();
    assert!(
        app_opt.is_some(),
        "the real-layout harness must reach BattleScapeState::BattleRunning with its font loaded",
    );
    let Some(mut app) = app_opt else { return };

    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            &longest_name,
            Magazine::loaded(MagazineSize::new(4), ReloadTu::new(18)),
        ),
    );
    // Settle: the panel's OnEnter spawn, the post-theme fit pass, and a real bevy_ui layout +
    // text-measurement pass over the newly-shown content each need their own frame.
    for _ in 0..8 {
        app.update();
    }

    let name_rect = pixel_rect_of::<WeaponNameText>(&mut app);
    let reload_rect = pixel_rect_of::<ReloadButton>(&mut app);
    assert!(
        name_rect.is_some() && reload_rect.is_some(),
        "both the weapon name label and the Reload button must exist with real computed geometry",
    );
    let (Some(name_rect), Some(reload_rect)) = (name_rect, reload_rect) else {
        return;
    };
    assert!(
        !name_rect.overlaps(&reload_rect),
        "the weapon name label ({:?}..{:?}) must NOT overlap the Reload button ({:?}..{:?}) — \
         GTW-733: the name row and the Reload button must not share the same pixels",
        name_rect.min,
        name_rect.max,
        reload_rect.min,
        reload_rect.max,
    );

    // The name's REAL measured glyph width fits inside its own text block — proof the longest
    // shipped identifier is not silently overflowing past its box (it either fits on one line or
    // wraps within `WeaponContent`'s width, per the `LineBreak::WordOrCharacter` fallback).
    let content_rect = pixel_rect_of::<WeaponContent>(&mut app);
    let name_entity = weapon_name_text_entity(&mut app);
    assert!(
        content_rect.is_some() && name_entity.is_some(),
        "the weapon-text block and the name entity must exist",
    );
    let (Some(content_rect), Some(name_entity)) = (content_rect, name_entity) else {
        return;
    };
    let layout_info = app.world().get::<TextLayoutInfo>(name_entity);
    assert!(
        layout_info.is_some(),
        "the name text must have a real measured TextLayoutInfo (the real-layout harness's font \
         must have finished loading)",
    );
    let Some(layout_info) = layout_info else {
        return;
    };
    let content_width = content_rect.max.x - content_rect.min.x;
    assert!(
        layout_info.size.x <= content_width + EPSILON_PX,
        "the weapon name \"{longest_name}\"'s REAL measured glyph width ({}) must fit inside its \
         text block's width ({content_width}) — it must not silently overflow past its own box",
        layout_info.size.x,
    );
}
