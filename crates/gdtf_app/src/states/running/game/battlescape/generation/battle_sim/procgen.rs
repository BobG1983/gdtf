//! The app-side **procgen trigger** (GTW-433): build a battle's terrain by running the
//! sim's space-packing pipeline at `OnEnter(BattleScapeState::Generation)`, then merge that
//! terrain with the authored situation's gangers / spawn data.
//!
//! This is the LIVE trigger that makes procgen drive real battles. The authored
//! [`Situation`] now carries only `theme` + `grid_size` + `rosters` (GTW-433 C1 +
//! GTW-744: `skirmish.ron` authors NO inline terrain AND NO placement cells); the terrain is
//! GENERATED here by calling the sim-owned [`generate_level`], and each roster member is then
//! DEPLOYED onto the generated map's deployment zones ([`deploy_rosters`], deterministic in the
//! battle seed) and appended to the situation's gangers before setup.
//!
//! # The UUID model (GTW-492)
//!
//! GTW-492 (child T07b of the GTW-476 data-model refactor) switched the sim's procgen onto
//! the UUID-keyed v2 prefab model. The app-side glue now drives [`generate_level`] with the
//! UUID-keyed [`PrefabRegistry`] of [`Prefab`](gdtf_battle_sim::level::Prefab) fragments
//! (populated by the GTW-489 Load resolve from the GTW-490 migrated content), the
//! [`UuidThemeRegistry`] (the theme's default floor), the [`TerrainDefRegistry`] (classifying
//! each placed piece), and the authored situation's [`ThemeUuid`] theme directly — no
//! legacy theme-enum or prefab-registry shim. The GTW-491 (T07a) terrain shim
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
//! [`generate_level`] fails closed ([`PackingError`]) when the [`PrefabRegistry`] has no
//! player / enemy prefab for the theme (e.g. an EMPTY registry, the headless deep-walk
//! case). On a failure — OR when no [`PrefabRegistry`] is present at all — this returns the
//! AUTHORED situation UNCHANGED, so a battle with authored (or empty) terrain still sets up
//! and reaches `BattleRunning`. The real GUI path always has the loaded registries + a
//! theme+size situation, so procgen runs and produces a playable level; the fallback only
//! covers the no-content harnesses.

use bevy::prelude::warn;
use gdtf_assets::{ContentFinding, FindingDetail, FindingReferrer};
use gdtf_battle_sim::{
    level::{PrefabRegistry, UuidThemeRegistry},
    procgen::{EmittedLevel, PackingError, ProcgenFinding, ProcgenTuning, generate_level},
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
    /// The UUID-keyed prefab library (the GTW-489 Load resolve populates it from the
    /// `maps/<theme>/<size>/*.prefab.ron` content).
    pub prefabs: Option<&'a PrefabRegistry>,
    /// The UUID-keyed theme registry (the theme's default floor — the seam-lattice floor).
    pub themes:  Option<&'a UuidThemeRegistry>,
    /// The UUID-keyed terrain-definition registry (classifies each placed piece's sim-kind).
    pub terrain: Option<&'a TerrainDefRegistry>,
    /// The LIVE procgen fill tuning (GTW-533) — the OQ-6 fill density / large-prefab
    /// threshold / dead-rect scatter cap. Loaded + hot-reloaded from
    /// `core_tuning/procgen.tuning.ron` by the Load resolve. `None` on a no-content headless
    /// harness (no resource inserted) ⇒ the const RULED [`ProcgenTuning::default`] is used.
    pub tuning:  Option<&'a ProcgenTuning>,
}

/// The procgen driver's full outcome: the situation the battle is built from
/// plus every GTW-582 content-integrity finding the generation surfaced (a
/// named result struct per the no-bare-types rule, the sim's `EmittedLevel`
/// mirror). `findings` is EMPTY on a fully-resolved generation; the caller
/// (`request_battle_setup`, or the GTW-655 dev-tools stepper once its staged
/// drive completes) records each into the
/// [`ContentIntegrityReport`](gdtf_assets::ContentIntegrityReport).
///
/// `pub(crate)`, not scoped to this module's parent: the GTW-655 dev-tools stepper
/// (`crate::dev::procgen_stepper`) is a SECOND caller of [`outcome_from_emitted`] once its
/// staged drive reaches the emit stage, so it must name this type too.
pub(crate) struct ProcgenOutcome {
    /// The situation the battle is built from (generated terrain merged over
    /// the authored gangers, or the authored situation on the C4 fallback).
    pub situation:        Situation,
    /// The degraded resolutions / last-resort fallbacks the generation took
    /// (GTW-582 C3(d)/C5) — already `warn!`ed here; the caller reports them.
    pub findings:         Vec<ContentFinding>,
    /// The typed roster-DEPLOYMENT failure (GTW-744), when a deployment zone could not
    /// stand its whole roster — `None` on success (incl. the terrain-only stepper path and
    /// the C4 fallbacks, which never deploy). The caller
    /// (`request_battle_setup`) FAILS CLOSED on `Some`: it writes NO `SetupBattleRequested`,
    /// so the machine stays in Generation rather than starting an under-populated battle.
    pub deployment_error: Option<PackingError>,
}

/// Turn a successfully-[`generate_level`]d [`EmittedLevel`](gdtf_battle_sim::procgen::EmittedLevel)
/// into the [`ProcgenOutcome`] the battle is built from: merge its terrain over `authored`'s
/// gangers ([`merge_procgen_terrain`]) and convert every GTW-582 finding it carried, `warn!`ing
/// each here (the generation site — the sim is render-free and cannot write the report itself).
///
/// Extracted from [`procgen_battle_situation`]'s `Ok` arm (GTW-655) so the dev-tools stepper's
/// staged drive — which reaches an [`EmittedLevel`](gdtf_battle_sim::procgen::EmittedLevel) via
/// [`StagedProcgen::advance`](gdtf_battle_sim::procgen::StagedProcgen::advance) one stage at a
/// time instead of one [`generate_level`] call — can finish through the EXACT SAME merge +
/// finding-conversion logic as the normal path, rather than a second reimplementation that could
/// drift from it. [`procgen_battle_situation`] itself is UNCHANGED behaviorally by this split —
/// it now simply calls this function from its `Ok` arm.
#[must_use]
pub(crate) fn outcome_from_emitted(authored: Situation, emitted: EmittedLevel) -> ProcgenOutcome {
    // GTW-582 C3(d): every degraded resolution the emit took is loud — warn! here (the
    // generation site) and hand the converted findings to the caller for the
    // ContentIntegrityReport.
    let findings = emitted
        .findings
        .iter()
        .map(|finding| convert_procgen_finding(*finding))
        .collect();
    ProcgenOutcome {
        situation: merge_procgen_terrain(authored, emitted.situation),
        findings,
        deployment_error: None,
    }
}

/// Build the battle's situation by running procgen terrain over the authored situation's
/// gangers / spawn data (GTW-433 C2/C3; GTW-492 UUID model), or returning the authored
/// situation unchanged on a procgen failure / missing registry (C4 fallback).
///
/// Drives the sim-owned [`generate_level`] with the authored situation's `theme`
/// ([`ThemeUuid`](gdtf_battle_sim::level::ThemeUuid)) + `grid_size`, the UUID-keyed
/// registries (`prefabs` / `themes` / `terrain`), a [`ProcgenRng`] derived from `seed`
/// ([`ProcgenRng::from_root`] — the deterministic injected per-battle seed, C2), and the LIVE
/// [`ProcgenTuning`] from `registries.tuning` (GTW-533: the hot-reloaded resource, falling
/// back to [`ProcgenTuning::default`] only on a no-content harness). On `Ok` it MERGES the
/// generated terrain with the authored
/// gangers via [`merge_procgen_terrain`]; on `Err` (or any registry `None`) it logs and
/// returns `authored` unchanged.
///
/// GTW-582: every degraded path is LOUD and lands in [`ProcgenOutcome::findings`]:
///
/// - the sim's [`ProcgenFinding`]s (a theme with no registry default floor — the
///   nil-sentinel pour; an unresolvable placed piece — the fail-open pour) are
///   `warn!`ed here and converted to report findings (C3(d));
/// - the `Err` empty-board fallback (the authored — often empty — terrain is used
///   as-is) survives SOLELY as this last resort: it `warn!`s (as before) AND
///   records a `DegradedFallback` finding, never silently (C5).
///
/// The missing-REGISTRY early-out stays a silent fallback by design: it is the
/// no-content HARNESS path (a load-ordering guard, not an authored dangling
/// reference — the C1 catalog keeps the two distinct).
#[must_use]
pub(in crate::states::running::game::battlescape::generation::battle_sim) fn procgen_battle_situation(
    authored: Situation,
    registries: ProcgenRegistries<'_>,
    seed: BattleSeed,
) -> ProcgenOutcome {
    // Any registry absent (a no-content harness) → nothing to generate against; use the
    // authored situation as-is (fail-open to the authored terrain, never panic).
    let (Some(prefabs), Some(themes), Some(terrain)) =
        (registries.prefabs, registries.themes, registries.terrain)
    else {
        return ProcgenOutcome {
            situation:        authored,
            findings:         Vec::new(),
            deployment_error: None,
        };
    };

    // The procgen RNG is derived from the SAME injected per-battle seed the rest of the
    // battle RNG uses (C2) — deterministic, reproducible, no wall-clock / thread_rng here.
    let mut rng = ProcgenRng::from_root(seed);
    // GTW-533: the LIVE procgen fill knobs. Use the resident `ProcgenTuning` resource (loaded
    // + hot-reloaded from `core_tuning/procgen.tuning.ron` by the Load resolve) when present,
    // else the const RULED default (the no-content headless harness, which inserts no
    // resource — same value the pass used before GTW-533 wired the load).
    let tuning = registries.tuning.copied().unwrap_or_default();

    // GTW-492: generate against the authored situation's UUID-keyed theme DIRECTLY — no
    // theme-enum shim. The migrated v2 prefabs author this theme's ThemeUuid, so the
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
        Ok(emitted) => super::deploy::deploy_over_generated(authored, emitted, seed),
        Err(err) => outcome_from_packing_error(authored, &err),
    }
}

/// Fail closed to the authored terrain on a [`PackingError`](gdtf_battle_sim::procgen::PackingError)
/// — the LAST-RESORT empty-board fallback (GTW-582 C5): the authored (often empty) situation is
/// used as-is, `warn!`ed and recorded as a `DegradedFallback` finding, never silently.
///
/// Extracted from [`procgen_battle_situation`]'s `Err` arm (GTW-655) for the SAME reason as
/// [`outcome_from_emitted`]: the dev-tools stepper's staged drive can also fail closed on the
/// assemble or fill stage, and finishes through this EXACT same fallback logic rather than a
/// second reimplementation.
#[must_use]
pub(crate) fn outcome_from_packing_error(
    authored: Situation,
    err: &PackingError,
) -> ProcgenOutcome {
    warn!(
        "procgen could not assemble a level ({err}); using the authored situation's terrain \
         instead"
    );
    let finding = ContentFinding::DegradedFallback {
        context: FindingReferrer::new(format!(
            "procgen for theme {} (empty-board fallback)",
            *authored.theme,
        )),
        detail:  FindingDetail::new(format!(
            "could not assemble a level ({err}); the authored situation's terrain was used \
             instead"
        )),
    };
    ProcgenOutcome {
        situation:        authored,
        findings:         vec![finding],
        deployment_error: None,
    }
}

/// Convert one sim-side [`ProcgenFinding`] into its report record, `warn!`ing
/// it at this (the generation) site — the sim is render-free and cannot write
/// the report itself, so the app-side driver is where the degradation gets
/// loud (GTW-582 C3(d)/C5).
fn convert_procgen_finding(finding: ProcgenFinding) -> ContentFinding {
    match finding {
        ProcgenFinding::MissingThemeDefaultFloor { theme } => {
            warn!(
                "procgen: theme {} resolves no default floor in the UuidThemeRegistry; the \
                 level was poured with the nil-sentinel floor (playable but degraded)",
                *theme,
            );
            ContentFinding::DegradedFallback {
                context: FindingReferrer::new(format!("procgen for theme {}", *theme)),
                detail:  FindingDetail::new(
                    "the theme resolves no default floor; the level was poured with the \
                     nil-sentinel floor"
                        .to_owned(),
                ),
            }
        }
        ProcgenFinding::UnresolvedTerrainPiece { piece } => {
            warn!(
                "procgen: placed terrain piece {} resolves no TerrainDef; poured fail-open \
                 into the walls list (it will surface as TerrainNotFound at setup)",
                *piece,
            );
            ContentFinding::DegradedFallback {
                context: FindingReferrer::new(format!("procgen placed terrain piece {}", *piece)),
                detail:  FindingDetail::new(
                    "the piece resolves no TerrainDef; poured fail-open into the walls list \
                     (it will surface as TerrainNotFound at setup)"
                        .to_owned(),
                ),
            }
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
        // Authored placement data — the gangers + spawn the situation file owns. The
        // GTW-744 `rosters` are deployed into `gangers` by `deploy_over_generated` AFTER this
        // merge, so the merged situation carries an empty `rosters` (setup reads `gangers`).
        gangers:        authored.gangers,
        rosters:        Vec::new(),
        player_faction: authored.player_faction,
        // GTW-545: authored area-damage-field placements are situation-owned placement data
        // (a seeded hazard, like a ganger), so they carry through the procgen merge unchanged.
        fields:         authored.fields,
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
