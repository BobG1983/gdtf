//! Pure THEME-form RESOLUTION helpers (relocated here in the GTW-512 egui swap).
//!
//! The pre-egui shell tangled these pure functions inside the `bevy_ui` `render` / `systems`
//! drive files. The egui swap un-declared those files (the form is deferred to the C3 child), so the
//! pure logic the in-crate tests pin moves here:
//!
//! - [`resolved_stats`] — a terrain def → `(human summary, HP fraction)` for the C3 resolved-stats
//!   readout (proves the theme stores references, not inlined stats),
//! - [`floor_candidates`] — the default-floor candidate list for a draft (Slab-kind FIRST — C6),
//! - [`slab_floor_candidates`] — the Slab-ONLY default-floor candidate list (GTW-530 C3),
//! - [`sim_kind_label`] — the short Wall / Cover / Slab label.
//!
//! All three are pure (sim types + std only), so the C7 tests pin them without an app.

use gdtf_battle_sim::terrain::def::{TerrainDef, TerrainDefRegistry, TerrainSimKind, TerrainUuid};

use super::types::ThemeDraft;

/// The HP-bar ceiling the resolved-stats readout normalizes a structural HP against, for the C3 HP
/// fraction. A framework display const (not a domain value).
const HP_BAR_CEILING: f32 = 1000.0;

/// Resolve a terrain def into a `(human summary, HP fraction)` for the C3 readout — its kind, HP,
/// armor, hardness (+ band for Wall / Cover), and the HP fraction against [`HP_BAR_CEILING`]. Pure,
/// so the C7 test pins the resolution against a registry fixture.
#[must_use]
pub fn resolved_stats(def: &TerrainDef) -> (String, f32) {
    // `**hp` is a `u32` structural pool; the documented small-magnitude cast is clippy-clean under
    // the workspace `cast_precision_loss` allow (the presenter `framing.rs` precedent).
    #[expect(
        clippy::cast_precision_loss,
        reason = "structural HP is a small u32 pool well within f32 exact range; the readout shows \
                  a rounded HP and an HP-bar fraction"
    )]
    let (kind, hp, protection, hardness, band) = match &def.sim_kind {
        TerrainSimKind::Wall {
            hp,
            armor_protection,
            armor_hardness,
            height_band,
        } => (
            "Wall",
            **hp as f32,
            **armor_protection,
            **armor_hardness,
            Some(format!("{height_band:?}")),
        ),
        TerrainSimKind::Cover {
            hp,
            armor_protection,
            armor_hardness,
            height_band,
        } => (
            "Cover",
            **hp as f32,
            **armor_protection,
            **armor_hardness,
            Some(format!("{height_band:?}")),
        ),
        TerrainSimKind::Slab {
            hp,
            armor_protection,
            armor_hardness,
        } => (
            "Slab",
            **hp as f32,
            **armor_protection,
            **armor_hardness,
            None,
        ),
    };
    let name = (*def.display_name).clone();
    let band_line = band.map_or_else(String::new, |b| format!("\nBand: {b}"));
    let summary = format!(
        "{name}\n{kind}\nHP: {hp:.0}\nArmor: {protection}\nHardness: {hardness}{band_line}"
    );
    (summary, hp / HP_BAR_CEILING)
}

/// The default-floor candidate list for a draft: its selected terrain, Slab-kind FIRST (C6), each
/// as `(key, "display name [Kind]")`. Pure, so the C7 test pins the Slab-preference ordering.
#[must_use]
pub fn floor_candidates(
    draft: &ThemeDraft,
    terrain: &TerrainDefRegistry,
) -> Vec<(TerrainUuid, String)> {
    let mut slabs: Vec<(TerrainUuid, String)> = Vec::new();
    let mut others: Vec<(TerrainUuid, String)> = Vec::new();
    for key in draft.terrain() {
        let Some(def) = terrain.def(key) else {
            continue;
        };
        let label = format!("{}  [{}]", *def.display_name, sim_kind_label(&def.sim_kind));
        if matches!(def.sim_kind, TerrainSimKind::Slab { .. }) {
            slabs.push((*key, label));
        } else {
            others.push((*key, label));
        }
    }
    slabs.extend(others);
    slabs
}

/// The default-floor candidate list RESTRICTED to Slab-kind terrain (GTW-530 C3) — the walkable
/// floor is always a slab, so the default-floor picker offers ONLY the draft's selected terrain
/// whose [`sim_kind`](TerrainDef::sim_kind) is [`TerrainSimKind::Slab`]; every Wall / Cover terrain
/// is filtered OUT. Each surviving candidate is `(key, "display name [Slab]")`.
///
/// Where [`floor_candidates`] merely ORDERS slabs first (keeping non-slabs as trailing options),
/// this DROPS non-slabs entirely so no Wall / Cover can ever be chosen as the default floor. Pure,
/// so the C3 test pins the Slab-only filter against a mixed-kind registry fixture.
#[must_use]
pub fn slab_floor_candidates(
    draft: &ThemeDraft,
    terrain: &TerrainDefRegistry,
) -> Vec<(TerrainUuid, String)> {
    draft
        .terrain()
        .iter()
        .filter_map(|key| {
            let def = terrain.def(key)?;
            if matches!(def.sim_kind, TerrainSimKind::Slab { .. }) {
                let label = format!("{}  [{}]", *def.display_name, sim_kind_label(&def.sim_kind));
                Some((*key, label))
            } else {
                None
            }
        })
        .collect()
}

/// The short human label for a terrain sim kind (Wall / Cover / Slab) — shown beside each library
/// row's name so the author sees the structural kind at a glance (C2).
#[must_use]
pub(crate) const fn sim_kind_label(kind: &TerrainSimKind) -> &'static str {
    match kind {
        TerrainSimKind::Wall { .. } => "Wall",
        TerrainSimKind::Cover { .. } => "Cover",
        TerrainSimKind::Slab { .. } => "Slab",
    }
}
