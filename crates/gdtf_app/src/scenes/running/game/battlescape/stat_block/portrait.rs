//! The deterministic ganger → portrait mapping + the portrait `bevy_ui` node builder
//! (GTW-278).
//!
//! The portrait is a presentation-only affordance: which face a ganger shows is a
//! DETERMINISTIC, presenter/UI-side mapping from its stable [`GangerName`] — the
//! sim/ganger model and `skirmish.ron` stay untouched ([[sim-is-presentation-agnostic-no-px]]).
//! The face index is a FIXED-seed hash of the name (a
//! [`DefaultHasher`](std::collections::hash_map::DefaultHasher), NOT the randomized
//! [`RandomState`](std::collections::hash_map::RandomState)) modulo the
//! portrait count, so a given ganger shows the SAME face every run; a nameless ganger
//! falls back to face 0.
//!
//! The portrait renders as a `bevy_ui` [`ImageNode`](bevy::ui::widget::ImageNode) atlas
//! variant over the presenter's [`SheetRole::Portraits`](gdtf_battle_presenter::SheetRole)
//! sheet — a child node of the stat block, in the UI layer, NOT a world `Sprite`. The
//! update mutates the node's [`TextureAtlas::index`] in place ([[ui-mutate-not-respawn]]).

use std::hash::{Hash, Hasher};

use bevy::{
    image::TextureAtlas,
    prelude::*,
    ui::{Node, Val, widget::ImageNode},
};
use gdtf_battle_sim::GangerName;

/// The number of distinct portrait faces in the
/// [`SheetRole::Portraits`](gdtf_battle_presenter::SheetRole) sheet.
///
/// The portrait atlas (`assets/tiles/alt_tileset_portraits.png`) is a 10×10 grid of
/// 32×32-px faces — 100 indices, `0..=99`. The deterministic name → face mapping reduces
/// modulo this count. A `const` (the layout fact, not a domain newtype — the
/// framework-plumbing carve-out, `.claude/rules/no-bare-types.md` clause 4): it pairs the
/// presenter's `(10, 10)` grid for this sheet.
const PORTRAIT_COUNT: usize = 100;

/// The on-screen edge of the portrait node, as a fraction of the WINDOW HEIGHT
/// ([`Val::Vh`](bevy::ui::Val) — GTW-295 responsive ruling).
///
/// A `const`, NOT a domain newtype — layout plumbing fed straight to a [`Node`]'s
/// `width`/`height` (the `CELL_PX`-class carve-out). Used for BOTH the width and the
/// height so the face stays SQUARE while scaling with the window (a `vh` on each edge keeps
/// the same window-relative edge length both ways). `8` vh sits in the mockup's
/// "portrait-sized" band, a square face above the vitals — and replaces the old fixed
/// 56-px size so the portrait scales instead of staying pinned at one resolution.
const PORTRAIT_VH: f32 = 8.0;

/// A deterministic index into the portrait atlas (`0..`[`PORTRAIT_COUNT`]).
///
/// A named newtype over the bare atlas index (no-bare-types: which face a ganger shows
/// is a domain value, the `gdtf_ui` `theme` house style — private inner + derived
/// [`Deref`]). Built ONLY through [`PortraitIndex::for_name`], which guarantees the value
/// is in range (`% PORTRAIT_COUNT`), so a constructed [`PortraitIndex`] can always index
/// the atlas without an out-of-bounds face.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::scenes::running::game::battlescape) struct PortraitIndex(usize);

impl PortraitIndex {
    /// The deterministic portrait index for `name`, or face 0 for a nameless ganger.
    ///
    /// Hashes the [`GangerName`]'s inner `&str` with a FIXED-seed
    /// [`DefaultHasher`](std::collections::hash_map::DefaultHasher) (default-constructed —
    /// its seed is constant, unlike
    /// [`RandomState`](std::collections::hash_map::RandomState)'s per-process random
    /// seed) and reduces the digest modulo [`PORTRAIT_COUNT`], so the SAME name always
    /// maps to the SAME face across runs. [`None`] → index 0 (the contract's nameless
    /// fallback).
    #[must_use]
    pub(in crate::scenes::running::game::battlescape) fn for_name(
        name: Option<&GangerName>,
    ) -> Self {
        let Some(name) = name else {
            return Self(0);
        };
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        // Hash the inner &str (GangerName derefs to it), not the wrapper, so the digest is
        // exactly a function of the displayed name.
        (**name).hash(&mut hasher);
        // PORTRAIT_COUNT is a non-zero const, so the modulo never divides by zero; the
        // result is always in 0..PORTRAIT_COUNT.
        Self(usize::try_from(hasher.finish() % PORTRAIT_COUNT as u64).unwrap_or(0))
    }
}

/// The deterministic portrait atlas index for `name` (the bare `usize` of
/// [`PortraitIndex::for_name`]).
///
/// Defined ONLY under `test-support` (it has no production caller — the update uses
/// [`PortraitIndex::for_name`] directly), so the external portrait integration test can
/// compute the EXPECTED index from the SAME rule the production update applies (the
/// contract: "compute the expected index in the test from the same hash rule"). Fully gated
/// behind the feature so the production binary build carries no dead `pub(crate)` fn.
#[cfg(feature = "test-support")]
#[must_use]
pub fn portrait_index_for_name(name: Option<&GangerName>) -> usize {
    *PortraitIndex::for_name(name)
}

/// Builds the portrait `bevy_ui` [`ImageNode`](bevy::ui::widget::ImageNode) atlas-variant
/// bundle for `image` + `layout` at `index`, sized [`PORTRAIT_VH`] square.
///
/// The portrait is a UI node (NOT a world `Sprite` — the wrong layer for a panel): a
/// window-relative square [`Node`] (both edges [`PORTRAIT_VH`] vh, so it scales with the
/// window yet stays square — GTW-295 responsive ruling) carrying an [`ImageNode`] built via
/// [`ImageNode::from_atlas_image`] over the portraits sheet at the given atlas `index`.
/// The caller's identity marker is attached alongside. The update mutates the node's
/// [`TextureAtlas::index`] in place ([[ui-mutate-not-respawn]]).
pub(in crate::scenes::running::game::battlescape) fn portrait_node(
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    index: PortraitIndex,
    marker: impl Bundle,
) -> impl Bundle {
    (
        ImageNode::from_atlas_image(
            image,
            TextureAtlas {
                layout,
                index: *index,
            },
        ),
        Node {
            width: Val::Vh(PORTRAIT_VH),
            height: Val::Vh(PORTRAIT_VH),
            ..default()
        },
        marker,
    )
}
