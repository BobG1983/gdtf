//! [`stair_cell_sets`] + [`seed_slab_terrain`] — phase 3 of
//! [`setup_battle`](super::setup_battle): [`SurfaceGrid`](crate::surface::SurfaceGrid)
//! / [`SlabLedger`](crate::slab::SlabLedger) seeding, slab terrain entities, and the
//! GTW-391/392 stair + brace-stair cell sets.

use bevy::{
    platform::collections::HashSet,
    prelude::{Commands, Entity},
};

use super::super::terrain_resolve::ResolvedSlabPiece;
use crate::{
    metric::CellLevel,
    situation::Situation,
    slab::{BraceStairCells, SlabEntry, SlabLedger},
    surface::{SlabState, SurfaceGrid},
    terrain::{
        entity::{
            BlocksPathfinding, BlocksVision, TerrainBrace, TerrainCell, TerrainIndexKey,
            TerrainPieceKind,
        },
        openable::{OpenState, OpenableBlocking},
    },
    vertical::LinkKind,
};

/// GTW-391/392 — the two stair-cell sets the occupancy build + slab seed consult:
///
/// - the GTW-391 set: BOTH endpoints of every Stair link (for LOS eye-lift +
///   dual-cell presence). Ladders are excluded (no stair eye-lift / no dual-cell
///   body presence).
/// - the GTW-392 brace set: the LOWER endpoint of each Stair link only — a kneeling
///   occupant on that stair braces under the slab above it.
///
/// Returned as `(stair_cell_set, brace_stair_cells_set)`.
pub(super) fn stair_cell_sets(situation: &Situation) -> (HashSet<CellLevel>, HashSet<CellLevel>) {
    let mut stair_cell_set = HashSet::new();
    for link in &situation.vertical_links {
        if matches!(link.kind, LinkKind::Stair { .. }) {
            stair_cell_set.insert(link.from);
            stair_cell_set.insert(link.to);
        }
    }

    // GTW-392: the brace-eligible stair cells are the LOWER endpoint of each Stair link.
    // The upper arrival cell braces against its own storey ceiling (ordinary cover, not the
    // stair-brace slab), so it is excluded. This is a SEPARATE set from `stair_cell_set`
    // (which keeps BOTH endpoints for GTW-391 LOS/presence and must not be narrowed).
    // For a single-storey stair (from.z = n, to.z = n+1) the lower cell is correct.
    // For a multi-storey span (from.z = 0, to.z = 2) only storey 0 is brace-eligible;
    // storey 2's overhead slab at storey 3 would be a phantom brace — excluded here.
    let mut brace_stair_cells_set: bevy::platform::collections::HashSet<crate::metric::CellLevel> =
        bevy::platform::collections::HashSet::new();
    for link in &situation.vertical_links {
        if matches!(link.kind, LinkKind::Stair { .. }) {
            let lower = if link.from.z <= link.to.z {
                link.from
            } else {
                link.to
            };
            brace_stair_cells_set.insert(lower);
        }
    }
    (stair_cell_set, brace_stair_cells_set)
}

/// Phase 3 — seed the surface grid: every authored slab is `Present` (zero ground
/// damage by lazy default). Eagerly seed the [`SlabLedger`] from the resolved
/// per-slab specs (GTW-396 Decision C, major #4): `entry_seeded` returns an
/// eagerly-inserted entry unchanged on subsequent calls, so the authored per-slab HP
/// is honored on first strike via the normal depletion path (no `deplete_slab`
/// signature change). Also spawns ONE terrain entity per slab carrying the static
/// stats + hooks, and inserts the [`SurfaceGrid`] / [`SlabLedger`] /
/// [`BraceStairCells`] battle resources. Returns the slab `(TerrainIndexKey, Entity)`
/// pairs for the orchestrator's [`TerrainIndex`](crate::terrain::entity::TerrainIndex).
pub(super) fn seed_slab_terrain(
    situation: &Situation,
    resolved_slabs: &[ResolvedSlabPiece],
    brace_cells: &HashSet<CellLevel>,
    commands: &mut Commands,
) -> Vec<(TerrainIndexKey, Entity)> {
    let mut surface_grid = SurfaceGrid::new();
    let mut slab_ledger = SlabLedger::new();
    let mut terrain_pairs: Vec<(TerrainIndexKey, Entity)> = Vec::new();
    for (slab_spawn, resolved) in situation.slabs.iter().zip(resolved_slabs.iter()) {
        surface_grid.set_slab(slab_spawn.at, SlabState::Present);

        // GTW-396: eagerly seed the slab ledger from the per-slab authored spec.
        // `entry_seeded` (.entry(key).or_insert(seeded(...))) returns the
        // eagerly-inserted entry unchanged on a subsequent `deplete_slab` call, so the
        // authored per-slab HP is honored (the fallback prototype in the fold arm is
        // consumed ONLY for a slab that was struck with no authored entry — the no-panic
        // contract for an unauthored / out-of-bounds strike). See ledger.rs:68-74.
        let slab_entry = SlabEntry::seeded(
            resolved.max_hp,
            resolved.armor_protection,
            resolved.armor_hardness,
        );
        slab_ledger.insert(slab_spawn.at, slab_entry);

        // GTW-491: spawn the slab entity. Carries STATIC stats (max HP / armor) plus the
        // presentation graphic and — slab-ONLY — the OPTIONAL footfall. The live pool stays in
        // slab_ledger.
        let slab_entity = commands
            .spawn((
                TerrainCell::new(slab_spawn.at),
                TerrainPieceKind::Slab,
                resolved.max_hp,           // SlabHp — the static max (now Component)
                resolved.armor_protection, // ArmorProtection — already Component
                resolved.armor_hardness,   // ArmorHardness — already Component
                resolved.graphic.clone(),  // TerrainGraphicKey — presenter resolves to atlas entry
            ))
            .id();
        // The OPTIONAL footfall (slab-only in the GTW-491 model): inserted only when the def
        // names one (`Bevy 0.19`'s `Option<C>` is NOT a Bundle, so insert it conditionally
        // rather than tupling). UNCONSUMED — no footfall-audio system is built yet (guns-only).
        if let Some(footfall) = resolved.footfall.clone() {
            commands.entity(slab_entity).insert(footfall);
        }

        // GTW-501 C1/D2: a slab does NOT block path by default — attach the
        // BlocksPathfinding marker ONLY when the def carries an explicit BlocksPathfinding
        // tag (`resolved.blocks_path`), the C1 opt-in for a barricade/lip slab. `Added`
        // fires on insert so the GTW-501 projection blocks the cell.
        if *resolved.blocks_path {
            commands.entity(slab_entity).insert(BlocksPathfinding);
        }
        // GTW-502 C1/C2: a slab does NOT occlude vision by default (the slab march already
        // stops sight at its z-boundary) — attach BlocksVision (banded HIGH) ONLY when the def
        // carries an explicit BlocksVision tag (`resolved.occludes_vision`), the C1 opt-in for
        // an opaque screen/blast-wall slab. `Added` fires on insert so the GTW-502 projection
        // occludes the cell at that band.
        if let Some(band) = resolved.occludes_vision {
            commands.entity(slab_entity).insert(BlocksVision::new(band));
        }
        // GTW-503 C1/C2: an Openable Slab (a hatch) blocks BOTH path AND vision WHEN CLOSED
        // even though a slab does not block by default — attach OpenState::Closed +
        // OpenableBlocking(High) and FORCE both BlocksPathfinding + BlocksVision(High). This is
        // the C2 case the GTW-501/502 inserts above do NOT cover (an untagged slab gets
        // neither): GTW-503 owns the closed-hatch blocking. The band is HeightBand::High (a
        // slab spans the storey).
        if let Some(band) = resolved.openable {
            commands.entity(slab_entity).insert((
                OpenState::Closed,
                OpenableBlocking::new(band),
                BlocksPathfinding,
                BlocksVision::new(band),
            ));
        }

        // GTW-392: a slab is a stair-brace slab when the cell DIRECTLY BELOW it is a
        // brace-eligible (LOWER-endpoint) stair cell — a kneeling occupant on that stair
        // braces under this slab. Uses the brace-only set (lower endpoints), NOT
        // `stair_cell_set` (which has both endpoints for GTW-391 LOS/presence).
        //
        // `cell_below` returns None for level 0 (no cell below ground) — a level-0 slab
        // never becomes a brace slab (correct: ground level has no lower stair endpoint).
        if let Some(below) = cell_below(slab_spawn.at)
            && brace_cells.contains(&below)
        {
            commands.entity(slab_entity).insert(TerrainBrace);
        }

        terrain_pairs.push((TerrainIndexKey::Slab(slab_spawn.at), slab_entity));
    }
    commands.insert_resource(surface_grid);
    commands.insert_resource(slab_ledger);

    // GTW-392: insert the BraceStairCells resource (the lower-endpoint stair-cell set).
    // Removed at teardown alongside the other battle-lifetime resources.
    commands.insert_resource(BraceStairCells::new(brace_cells.clone()));

    terrain_pairs
}

/// The cell directly below `cell` (`(x, y, z − 1)`), or `None` when `cell` is at
/// level 0 (no cell below ground).
///
/// A private helper for GTW-392 stair-brace slab placement: a slab at `(x, y, z)`
/// is a brace slab only when the cell at `(x, y, z − 1)` is a brace-eligible stair
/// cell. Level-0 slabs never become brace slabs (their `z − 1` would be negative).
fn cell_below(cell: crate::metric::CellLevel) -> Option<crate::metric::CellLevel> {
    // The storey via the canonical CellLevel::level accessor (GTW-565), then a
    // checked u8 subtract — a level-0 cell has nothing below (checked_sub is None).
    let storey = (*cell.level()).checked_sub(1)?;
    Some(crate::metric::CellLevel::new(
        cell.cell(),
        crate::metric::Level::new(storey),
    ))
}
