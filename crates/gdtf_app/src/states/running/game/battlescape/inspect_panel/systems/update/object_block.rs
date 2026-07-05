//! The inspect panel's OBJECT (wall / cover) block: the cover-ledger entry resolution
//! and the labeled block fill. Split out of the monolithic `update.rs` (GTW-583); the
//! repaint rationale lives on the parent `update` module.

use bevy::prelude::*;
use gdtf_battle_sim::{CoverEntry, CoverLedger, HeightBand, OccupancyGrid, TerrainKind};
use gdtf_ui::{FillFraction, set_progress_bar};

use super::params::InspectNodes;
use crate::states::running::game::battlescape::stat_block::StatBlockWidgets;

/// The cover/object entry for a hovered cell, if it is a non-floor object — `None` for bare
/// floor (so the panel hides).
///
/// A cell is an OBJECT when its [`TerrainKind`] is [`Wall`](TerrainKind::Wall) or
/// [`Cover`](TerrainKind::Cover). Its structural stats come from the
/// [`CoverLedger`](gdtf_battle_sim::CoverLedger) `peek` (seeded at setup for every authored
/// piece); a wall with no ledger entry falls back to a default full-integrity entry so the
/// object block still renders a name. `None` (the floor case) hides the panel.
pub(super) fn object_entry(
    cell: gdtf_battle_sim::CellLevel,
    grid: Option<&OccupancyGrid>,
    ledger: Option<&CoverLedger>,
) -> Option<CoverEntry> {
    let grid = grid?;
    let terrain = grid.terrain(&cell);
    if !matches!(terrain, TerrainKind::Wall | TerrainKind::Cover) {
        return None;
    }
    // Prefer the seeded ledger entry; fall back to a minimal full-integrity entry for a
    // bare wall the ledger never registered (so the block still renders a name + a full bar).
    ledger.and_then(|l| l.peek(&cell).copied()).or_else(|| {
        Some(CoverEntry::seeded(
            gdtf_battle_sim::CoverHp::new(1),
            gdtf_battle_sim::HeightBand::High,
            gdtf_battle_sim::ArmorProtection::new(0),
            gdtf_battle_sim::ArmorHardness::new(0),
        ))
    })
}

/// Fills the object block from a hovered [`CoverEntry`] (GTW-295 AC3) — a readable object
/// stat block comparable to the ganger block, mutate-in-place ([[ui-mutate-not-respawn]]):
///
/// - the TITLE line ([`InspectObjectText`](crate::states::running::game::battlescape::inspect_panel::components::InspectObjectText)) → `"Cover"` (the object kind);
/// - the Integrity [`ProgressBar`] ([`InspectObjectBar`](crate::states::running::game::battlescape::inspect_panel::components::InspectObjectBar)) → `current_hp / max_hp`;
/// - the Hardness line ([`InspectObjectHardness`](crate::states::running::game::battlescape::inspect_panel::components::InspectObjectHardness)) → `"Hardness {armor_hardness}"`;
/// - the Protection line ([`InspectObjectProtection`](crate::states::running::game::battlescape::inspect_panel::components::InspectObjectProtection)) → `"Protection {armor_protection}"`;
/// - the Height-band line ([`InspectObjectHeight`](crate::states::running::game::battlescape::inspect_panel::components::InspectObjectHeight)) → `"Height: {height_band}"`.
///
/// Each `Text` write is gated on a real change; the static "Integrity" label is spawned once
/// and never rewritten.
pub(super) fn fill_object_block(
    widgets: &mut StatBlockWidgets,
    nodes: &InspectNodes,
    entry: CoverEntry,
) {
    set_line(widgets, nodes.object_title.iter().next(), "Cover");
    set_line(
        widgets,
        nodes.hardness.iter().next(),
        &format!("Hardness {}", *entry.armor_hardness),
    );
    set_line(
        widgets,
        nodes.protection.iter().next(),
        &format!("Protection {}", *entry.armor_protection),
    );
    set_line(
        widgets,
        nodes.height.iter().next(),
        &format!("Height: {}", height_label(entry.height_band)),
    );
    if let Some(bar) = nodes.object_bar.iter().next() {
        let fraction =
            FillFraction::from_ratio(hp_as_f32(*entry.current_hp), hp_as_f32(*entry.max_hp));
        set_progress_bar(bar, fraction, &widgets.children, &mut widgets.fills);
    }
}

/// Writes `value` into the `Text` of `entity` (if present) via the stat-block `texts` writer,
/// mutate-in-place and gated on a real change ([[ui-mutate-not-respawn]]).
fn set_line(widgets: &mut StatBlockWidgets, entity: Option<Entity>, value: &str) {
    if let Some(entity) = entity
        && let Ok(mut text) = widgets.texts.get_mut(entity)
        && text.as_str() != value
    {
        value.clone_into(&mut text.0);
    }
}

/// The display label for a cover's [`HeightBand`] (the LOW / MID / HIGH clearance band).
///
/// [`HeightBand`] has no `Display`, so the hover block names it directly here (a small,
/// fixed enum — no domain logic, just the on-screen caption).
const fn height_label(band: HeightBand) -> &'static str {
    match band {
        HeightBand::Low => "Low",
        HeightBand::Mid => "Mid",
        HeightBand::High => "High",
    }
}

/// A cover-HP magnitude (`u32`) as an `f32` for the bar ratio — clamped through `u16` so the
/// conversion is lossless and lint-clean (no `cast_precision_loss`); cover HP is a small pool,
/// so the `u16` ceiling is never reached in practice and saturating it is harmless for a bar.
fn hp_as_f32(hp: u32) -> f32 {
    f32::from(u16::try_from(hp).unwrap_or(u16::MAX))
}
