//! [`setup_battle`] — the 7-phase setup sequencing + final grid/resource insertion
//! (the one public entry point of the situation→entities pour).

use bevy::prelude::Commands;

use super::{
    registries::{BattleRegistries, BattleSetup},
    resolve, seed_cover, seed_slabs, spawn_gangers,
};
use crate::{
    occupancy::{OccupancyGrid, OccupancyInput, TerrainPlacement},
    situation::{BattleSetupError, Situation},
    terrain::{entity::TerrainIndex, floor::FloorCostGrid},
    tuning::MoveCost,
    vertical::build_vertical_link_graph,
};

/// Build the battle in the ECS world from a [`Situation`](crate::situation::Situation)
/// — the E1.8 setup: the setup system that builds the scene from the situation (see the
/// [`crate::situation`] module doc, the setup-on-entry source of truth).
///
/// GTW-414 schema v2: the function accepts `gangs: &GangRegistry` and resolves every
/// [`PlacedGanger`](crate::situation::PlacedGanger)'s `(gang, member)` ref against it BEFORE any entity is spawned
/// (abort-first invariant) — the resolved [`GangMember`](crate::ganger::GangMember) supplies the ganger's identity,
/// eight attributes, and weapon / armor keys (the old `GangerSpawn` fields, now sourced
/// from the reusable gang roster). A missing gang / member returns
/// [`BattleSetupError::GangNotFound`] / [`BattleSetupError::GangMemberNotFound`] (no panic).
///
/// GTW-491 (T07a): the function accepts `terrain: Option<&TerrainDefRegistry>` and resolves
/// every authored terrain DEFINITION UUID (cover / slab) against it BEFORE any entity is
/// spawned (abort-first invariant). If `terrain` is `None` or a UUID resolves to nothing,
/// the function returns [`BattleSetupError::TerrainNotFound`] (no panic, no partial world).
/// Each resolved [`TerrainDef`](crate::terrain::def::TerrainDef)'s
/// [`TerrainSimKind`](crate::terrain::def::TerrainSimKind) supplies the structural stats +
/// entity [`TerrainPieceKind`](crate::terrain::entity::TerrainPieceKind), and its
/// [`TerrainPresenterKind`](crate::terrain::def::TerrainPresenterKind) supplies the graphic
/// (ALL kinds incl. `Wall`) + optional slab footfall. The resolved defs feed:
/// - cover pieces → `CoverEntry` + a `TerrainGraphicKey` on the entity (NET-NEW for `Wall`)
/// - slab pieces → `SlabEntry` (eagerly inserted into `SlabLedger`) + a `TerrainGraphicKey`
///   + an OPTIONAL `FootfallSound`
///
/// The floor cost grid uses the caller-supplied `fallback_floor_cost` uniformly: the new
/// [`TerrainSimKind`](crate::terrain::def::TerrainSimKind) model has no `Floor` variant and
/// carries no per-piece move cost this slice (the move-cost-from-`default_floor` seam is
/// GTW-482), so the legacy registry-driven floor-cost path is retired. The situation's
/// `default_floor` / `floors` UUID references are carried forward but their move cost is not
/// resolved here.
///
/// Steps, in order:
///
/// 1. **Pre-validate** all terrain definition UUIDs (cover + slab) against the registry —
///    abort-first (return early with `Err` before any spawn).
/// 2. **Spawn each ganger + relate its equipment** — for every
///    [`GangerSpawn`](crate::situation::GangerSpawn),
///    `commands.spawn_scene(ganger_scene(..))` the ganger's OWN per-field state
///    as a Bevy [`Scene`](bevy::scene::Scene) (GTW-322).
/// 3. **Seed the [`CoverLedger`](crate::cover::CoverLedger)** — insert a [`CoverEntry`](crate::cover::CoverEntry)
///    for every wall and scatter piece (the one unified ledger). Spawn ONE terrain entity
///    per cover piece carrying `TerrainCell`, `TerrainPieceKind` (derived from the def's
///    sim-kind VARIANT, the T01 invariant), max `CoverHp`, `HeightBand`, `ArmorProtection`,
///    `ArmorHardness`, and a `TerrainGraphicKey` (GTW-491 — NET-NEW on `Wall` entities).
/// 4. **Seed the [`SurfaceGrid`](crate::surface::SurfaceGrid) + [`SlabLedger`](crate::slab::SlabLedger)** — mark every authored slab
///    `Present` and **eagerly insert** its [`SlabEntry`](crate::slab::SlabEntry) (HP/armor from the resolved
///    def's `Slab` sim-kind). The ledger's `entry_seeded` path returns the eagerly-inserted
///    entry unchanged (the `or_insert` wins only for absent keys), so the authored per-slab
///    HP is honored on first strike without a new `deplete_slab` signature (verified per
///    `ledger.rs:68-74`). Spawn ONE terrain entity per slab carrying `TerrainCell`,
///    `TerrainPieceKind::Slab`, `SlabHp`, `ArmorProtection`, `ArmorHardness`,
///    `TerrainGraphicKey`, and (when the def names one) `FootfallSound`.
/// 5. **Build the [`OccupancyGrid`]** + insert [`TerrainIndex`].
/// 6. **Build the [`FloorCostGrid`]** from the uniform fallback floor cost and insert it.
/// 7. **Validate + build the [`crate::vertical::VerticalLinkGraph`]**.
///
/// All resources are inserted via [`Commands`]. Render-free, headless-driven.
///
/// # Errors
///
/// Returns a [`BattleSetupError`]:
/// - [`BattleSetupError::InvalidLink`] — bad vertical link.
/// - [`BattleSetupError::GangNotFound`] — a placed ganger's gang ref is absent (GTW-414).
/// - [`BattleSetupError::GangMemberNotFound`] — its member ref is absent from that gang
///   (GTW-414).
/// - [`BattleSetupError::StackedGangers`] — two gangers share one `(cell, level)` spawn
///   slot (GTW-457).
/// - [`BattleSetupError::WeaponNotFound`] — the resolved roster member's weapon key absent.
/// - [`BattleSetupError::MeleeWeaponNotFound`] — the resolved roster member's melee weapon
///   key (or the `fists` default) absent (GTW-505).
/// - [`BattleSetupError::ArmorNotFound`] — the resolved roster member's armor key absent.
/// - [`BattleSetupError::TerrainNotFound`] — a cover/slab terrain definition UUID absent
///   (GTW-491).
///
/// All are validated BEFORE any entity is spawned (abort-first invariant).
pub fn setup_battle(
    situation: &Situation,
    registries: BattleRegistries<'_>,
    fallback_floor_cost: MoveCost,
    commands: &mut Commands,
) -> Result<BattleSetup, BattleSetupError> {
    // Destructure the borrow-bundle into the named registry refs the phases read. The
    // bundle exists only to keep setup_battle under clippy's argument-count gate (the
    // GTW-414 `gangs` ref pushed the flat list to 8); the phases below are unchanged.
    let BattleRegistries {
        gangs,
        weapons,
        melee_weapons,
        armor,
        stat_tuning,
        terrain,
        fields,
        attachments,
    } = registries;
    // Validate the vertical links FIRST, so a bad authored link aborts the whole
    // setup before any entity is spawned or any resource inserted (no partial,
    // unspawnable world left behind on a validation failure).
    let vertical_graph = build_vertical_link_graph(situation)?;

    // Abort-first pre-spawn resolution (GTW-414 / GTW-491 / GTW-505 / GTW-549): every
    // authored reference resolves against its registry BEFORE any entity is spawned,
    // so a bad key aborts with the typed error and no partial world (see `resolve`).
    let resolved_members = resolve::resolve_members(situation, gangs)?;

    // GTW-457: reject a situation that authors two gangers on the SAME (cell, level)
    // — BEFORE any ganger entity is spawned (abort-first, mirroring the gang/member
    // ref checks above). The GTW-156 occupancy pour is last-write-wins, so a duplicate
    // would silently overwrite the first ganger's occupancy slot while BOTH entities
    // survive stacked on one cell (`docs/combat/resolution.md`: one object per cell).
    // A trusted authored situation with stacked spawns is a DATA bug to fail LOUDLY on,
    // not auto-relocate (auto-relocation belongs in the GTW-424 procgen assembler).
    if let Some(at) = resolve::first_stacked_cell(situation) {
        return Err(BattleSetupError::StackedGangers { at });
    }

    let weapon_bundles = resolve::resolve_weapon_bundles(&resolved_members, weapons, attachments)?;
    let melee_bundles =
        resolve::resolve_melee_bundles(&resolved_members, melee_weapons, attachments)?;
    let armor_specs = resolve::resolve_armor_specs(&resolved_members, armor)?;
    let (resolved_covers, cover_on_death_entries) = resolve::resolve_covers(situation, terrain)?;
    let resolved_slabs = resolve::resolve_slabs(situation, terrain)?;

    // GTW-545: pre-resolve every authored area-damage-field placement against the
    // FieldDefRegistry BEFORE any resource is inserted — abort-first, mirroring the terrain
    // pre-resolve above. A missing field key aborts with FieldNotFound (no panic, no partial
    // world). The built live FieldRegistry is inserted alongside the other battle grids below.
    let field_registry = resolve::build_field_registry(situation, fields)?;

    // GTW-491: the new `TerrainSimKind` model has no `Floor` variant (a walkable floor is a
    // `Slab` def) and `TerrainDef` carries no move cost this slice — the per-cell
    // move-cost-from-default-floor seam is GTW-482. So the floor cost grid uses the
    // caller-supplied `fallback_floor_cost` uniformly; the situation's `floors`/`default_floor`
    // terrain references are carried forward but their move cost is NOT resolved here (the
    // legacy `resolve_floor_costs` registry path is retired with the per-file floor-kind
    // authoring variant it read).
    let (default_floor_cost, floor_overrides) = (fallback_floor_cost, Vec::new());

    // 1. Spawn each ganger + relate its equipment entities (see `spawn_gangers`).
    let occupants = spawn_gangers::spawn_gangers(
        commands,
        &resolved_members,
        weapon_bundles,
        melee_bundles,
        armor_specs,
        stat_tuning,
    );

    // 2. Seed the cover ledger from walls + scatter and spawn the cover terrain
    //    entities (see `seed_cover_terrain`).
    let (mut terrain_pairs, occupancy_kinds) =
        seed_cover::seed_cover_terrain(situation, resolved_covers, commands);

    // GTW-391/392: the stair-cell + brace-stair-cell sets are computed ABOVE the slab
    // seed so the slab loop can consult the brace set when spawning slab entities.
    let (stair_cell_set, brace_stair_cells_set) = seed_slabs::stair_cell_sets(situation);

    // 3. Seed the surface grid / slab ledger + spawn the slab terrain entities
    //    (see `seed_slab_terrain`); the slab pairs extend the cover pairs so the
    //    TerrainIndex below indexes every terrain entity.
    terrain_pairs.extend(seed_slabs::seed_slab_terrain(
        situation,
        &resolved_slabs,
        &brace_stair_cells_set,
        commands,
    ));

    // Own the placements in the result up front, so the occupancy grid can borrow
    // them (no clone) and the same Vec is returned to the caller.
    let setup = BattleSetup { occupants };

    // 4. Build the occupancy grid: terrain from walls + scatter, occupants from the
    //    spawned entities (borrowed from the result's placement list). The terrain kind
    //    is derived from the resolved spec VARIANT — captured into `occupancy_kinds`
    //    (walls-then-scatter order) during the cover-ledger loop above as
    //    `TerrainKind::from(resolved.piece_kind)` (Wall spec → TerrainKind::Wall;
    //    Cover/Scatter spec → TerrainKind::Cover). GTW-483: this replaces the former
    //    list-membership re-derive, so a Cover/Scatter piece authored in the `walls`
    //    list (or a Wall in `scatter`) reads its DEF's own kind, not the kind implied
    //    by which list it sat in.
    let terrain_placements: Vec<TerrainPlacement> = situation
        .walls
        .iter()
        .chain(situation.scatter.iter())
        .zip(occupancy_kinds)
        .map(|(cover, kind)| TerrainPlacement::new(cover.at, kind))
        .collect();
    let occupancy_input = OccupancyInput {
        terrain:   terrain_placements,
        occupants: setup.occupants.clone(),
    };
    // Pass the stair-cell set so build_from_occupancy_input can (a) populate the stair
    // set on the grid AND (b) register upper-cell presence for any stair occupant from
    // the very first frame (GTW-391 Blocker 2). The post-build mark_stair_cell loop is
    // no longer needed — build_from_occupancy_input now handles both.
    let occupancy_grid =
        OccupancyGrid::build_from_occupancy_input(&occupancy_input, &stair_cell_set);
    commands.insert_resource(occupancy_grid);

    // GTW-395: insert the TerrainIndex after the occupancy grid is built — all terrain
    // entities are spawned (pairs accumulated in steps 2.5 and 3), so the index is
    // now complete. Battle-lifetime resource, removed in teardown alongside CoverLedger
    // and SlabLedger.
    commands.insert_resource(TerrainIndex::new(terrain_pairs));

    // 6. Build and insert the FloorCostGrid from the pre-resolved floor costs.
    commands.insert_resource(FloorCostGrid::new(default_floor_cost, floor_overrides));

    // 7. The validated vertical-link graph (validation already ran above).
    commands.insert_resource(vertical_graph);

    // GTW-545: insert the live area-damage-field registry (seeded from the situation's
    // authored `fields:` list, resolved abort-first above). Battle-lifetime — removed at
    // teardown alongside the other battle grids. Empty when the situation authored no fields.
    commands.insert_resource(field_registry);

    // GTW-547: insert the cover-cell on-death registry (seeded from each cover piece's authored
    // `on_death` field above). Battle-lifetime — removed at teardown alongside the other battle
    // grids. Empty when no cover piece authored an on-death effect. `resolve_on_death` reads it
    // for a COVER death (an OnDeathOccurred carrying Entity::PLACEHOLDER, keyed by cell).
    commands.insert_resource(crate::effects::on_death::CoverOnDeathRegistry::new(
        cover_on_death_entries,
    ));

    Ok(setup)
}
