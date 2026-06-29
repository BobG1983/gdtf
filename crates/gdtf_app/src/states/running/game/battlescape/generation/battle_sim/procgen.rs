//! The app-side **procgen trigger** (GTW-433): build a battle's terrain by running the
//! sim's space-packing pipeline at `OnEnter(BattleScapeState::Generation)`, then merge that
//! terrain with the authored situation's gangers / spawn data.
//!
//! This is the LIVE trigger that makes procgen drive real battles. The authored
//! [`Situation`] now carries only `theme` + `grid_size` + the placed gangers (GTW-433 C1:
//! `skirmish.ron` no longer authors inline terrain); the terrain is GENERATED here by
//! calling the sim-owned [`generate_level`] and pouring its result back over the authored
//! gangers.
//!
//! # The UUID model (GTW-492)
//!
//! GTW-492 (child T07b of the GTW-476 data-model refactor) switched the sim's procgen onto
//! the UUID-keyed v2 prefab model. The app-side glue now drives [`generate_level`] with the
//! UUID-keyed [`PrefabRegistry2`] of [`Prefab2`](gdtf_battle_sim::level::Prefab2) fragments
//! (populated by the GTW-489 Load resolve from the GTW-490 migrated content), the
//! [`UuidThemeRegistry`] (the theme's default floor), the [`TerrainDefRegistry`] (classifying
//! each placed piece), and the authored situation's [`ThemeUuid`] theme directly — NO
//! [`LevelTheme`](gdtf_battle_sim::level::LevelTheme) shim, NO legacy
//! [`PrefabRegistry`](gdtf_battle_sim::level::PrefabRegistry). The GTW-491 (T07a) terrain shim
//! adapter is REMOVED: procgen now emits terrain referencing the migrated AUTHORED UUIDs, so
//! `setup_battle` resolves it against the real `TerrainDefRegistry` directly.
//!
//! # The one-way model boundary
//!
//! The sim ([`gdtf_battle_sim`]) OWNS terrain generation — [`generate_level`] is the
//! deterministic, render-free seed harness; this app-side glue only DRIVES it (it reads the
//! sim's `Result<Situation, PackingError>` and feeds the merged situation back into the sim
//! via [`SetupBattleRequested`]). The sim never reads the app: the dependency stays one-way
//! (`docs/decisions/0001-rust-bevy-rewrite.md`).
//!
//! # The seed (C2)
//!
//! Procgen draws from a [`ProcgenRng`] derived from the battle's [`BattleSeed`] via
//! [`ProcgenRng::from_root`] — the SAME deterministic, injected per-battle seed the rest of
//! the battle RNG is built from (resolved by `request_battle_setup` from a pre-injected
//! `Res<BattleSeed>` override or `resolve_root_seed`, NEVER `thread_rng` / `Instant`). So
//! the same seed always reproduces the same level (the replay property inherited from
//! GTW-431). This app-side [`ProcgenRng`] instance is independent of the one the sim's
//! `setup_battle_on_request` derives for in-battle draws (both seed identically from the
//! same root, so neither perturbs the other — see `foundation/rng/streams.rs`).
//!
//! # The merge (C3)
//!
//! [`generate_level`] returns a [`Situation`] whose TERRAIN is the assembled level but whose
//! `gangers` are empty (rosters are placed by the authored situation, not by the terrain
//! emit). [`merge_procgen_terrain`] takes that procgen terrain and the AUTHORED gangers /
//! `player_faction`, producing the situation the battle is actually built from:
//! `{authored gangers/spawn/player_faction} + {procgen terrain}`. The `theme` / `grid_size`
//! are identical on both (the authored values were what we generated against).
//!
//! # Fallback (C4)
//!
//! [`generate_level`] fails closed ([`PackingError`]) when the [`PrefabRegistry2`] has no
//! player / enemy prefab for the theme (e.g. an EMPTY registry, the headless deep-walk
//! case). On a failure — OR when no [`PrefabRegistry2`] is present at all — this returns the
//! AUTHORED situation UNCHANGED, so a battle with authored (or empty) terrain still sets up
//! and reaches `BattleRunning`. The real GUI path always has the loaded registries + a
//! theme+size situation, so procgen runs and produces a playable level; the fallback only
//! covers the no-content harnesses.

use bevy::prelude::warn;
use gdtf_battle_sim::{
    level::{PrefabRegistry2, UuidThemeRegistry},
    procgen::{ProcgenTuning, generate_level},
    rng::{BattleSeed, ProcgenRng},
    situation::Situation,
    terrain::def::TerrainDefRegistry,
};

/// The read-only content registries [`procgen_battle_situation`] resolves the generated level
/// against — grouped into one borrow-bundle so the function (and its caller) stay under
/// clippy's argument-count gate.
///
/// A named borrow-bundle (no-bare-types: the procgen resolution sources are a domain
/// grouping, not a bare tuple of refs), mirroring the sim's `BattleRegistries` precedent.
/// Each is `Option` because a setup could be requested before the `Load`-scene registries
/// are present (a no-content headless harness); any `None` ⇒ the authored situation is used
/// as-is (the C4 fallback — no terrain to generate against).
#[derive(Clone, Copy)]
pub(in crate::states::running::game::battlescape::generation::battle_sim) struct ProcgenRegistries<
    'a,
> {
    /// The UUID-keyed v2 prefab library (the GTW-489 Load resolve populates it from the
    /// migrated `maps/<theme>/<size>/*.prefab_v2.ron` content).
    pub prefabs: Option<&'a PrefabRegistry2>,
    /// The UUID-keyed theme registry (the theme's default floor — the seam-lattice floor).
    pub themes:  Option<&'a UuidThemeRegistry>,
    /// The UUID-keyed terrain-definition registry (classifies each placed piece's sim-kind).
    pub terrain: Option<&'a TerrainDefRegistry>,
}

/// Build the battle's situation by running procgen terrain over the authored situation's
/// gangers / spawn data (GTW-433 C2/C3; GTW-492 UUID model), or returning the authored
/// situation unchanged on a procgen failure / missing registry (C4 fallback).
///
/// Drives the sim-owned [`generate_level`] with the authored situation's `theme`
/// ([`ThemeUuid`](gdtf_battle_sim::level::ThemeUuid)) + `grid_size`, the UUID-keyed
/// registries (`prefabs` / `themes` / `terrain`), a [`ProcgenRng`] derived from `seed`
/// ([`ProcgenRng::from_root`] — the deterministic injected per-battle seed, C2), and the
/// RULED-default [`ProcgenTuning`]. On `Ok` it MERGES the generated terrain with the authored
/// gangers via [`merge_procgen_terrain`]; on `Err` (or any registry `None`) it logs and
/// returns `authored` unchanged.
#[must_use]
pub(in crate::states::running::game::battlescape::generation::battle_sim) fn procgen_battle_situation(
    authored: Situation,
    registries: ProcgenRegistries<'_>,
    seed: BattleSeed,
) -> Situation {
    // Any registry absent (a no-content harness) → nothing to generate against; use the
    // authored situation as-is (fail-open to the authored terrain, never panic).
    let (Some(prefabs), Some(themes), Some(terrain)) =
        (registries.prefabs, registries.themes, registries.terrain)
    else {
        return authored;
    };

    // The procgen RNG is derived from the SAME injected per-battle seed the rest of the
    // battle RNG uses (C2) — deterministic, reproducible, no wall-clock / thread_rng here.
    let mut rng = ProcgenRng::from_root(seed);
    // The RULED-default procgen knobs (the live tuning hot-reload was deferred with its
    // consumer; the defaults are the shipped values — see procgen `tuning.rs`).
    let tuning = ProcgenTuning::default();

    // GTW-492: generate against the authored situation's UUID-keyed theme DIRECTLY — no
    // LevelTheme shim. The migrated v2 prefabs author this theme's ThemeUuid, so the
    // theme-keyed candidate lookup resolves them; the emitted terrain references the migrated
    // AUTHORED terrain UUIDs, so setup resolves it against the real TerrainDefRegistry (no
    // legacy from_legacy_name shim adapter).
    match generate_level(
        prefabs,
        themes,
        terrain,
        authored.theme,
        authored.grid_size,
        &mut rng,
        &tuning,
    ) {
        Ok(generated) => merge_procgen_terrain(authored, generated),
        Err(err) => {
            // Fail closed to the authored terrain (C4): the headless deep-walk seeds EMPTY
            // registries, so procgen cannot assemble a level — the authored (often empty)
            // situation still sets up and reaches BattleRunning. The real GUI path always has
            // the loaded prefabs, so this never fires there.
            warn!(
                "procgen could not assemble a level ({err}); using the authored situation's \
                 terrain instead"
            );
            authored
        }
    }
}

/// Merge the procgen-`generated` situation's TERRAIN over the `authored` situation's
/// gangers / spawn data (GTW-433 C3).
///
/// The battle is built from `{authored gangers/spawn/player_faction} + {procgen terrain}`:
/// the generated situation supplies every terrain entry (`default_floor` / `walls` /
/// `scatter` / `slabs` / `floors` / `vertical_links`); the authored situation supplies the
/// `gangers` and `player_faction`. `theme` / `grid_size` are identical on both (the authored
/// values are what `generated` was assembled against — GTW-492: the emit sets
/// `generated.theme` to the same `ThemeUuid` we passed in), so they carry through unchanged
/// from `authored`.
fn merge_procgen_terrain(authored: Situation, generated: Situation) -> Situation {
    Situation {
        // Authored placement data — the rosters + spawn the situation file owns.
        gangers:        authored.gangers,
        player_faction: authored.player_faction,
        // GTW-492: `theme` is the UUID-keyed `ThemeUuid` authored metadata carried through
        // from the authored situation (it equals `generated.theme` now — procgen generated
        // against it directly — so either choice is identical; keep the authored source).
        theme:          authored.theme,
        grid_size:      authored.grid_size,
        // Procgen-generated terrain — the assembled level.
        default_floor:  generated.default_floor,
        walls:          generated.walls,
        scatter:        generated.scatter,
        slabs:          generated.slabs,
        floors:         generated.floors,
        vertical_links: generated.vertical_links,
    }
}
