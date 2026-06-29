//! The app-side **procgen trigger** (GTW-433): build a battle's terrain by running the
//! sim's GTW-431 space-packing pipeline at `OnEnter(BattleScapeState::Generation)`, then
//! merge that terrain with the authored situation's gangers / spawn data.
//!
//! This is the LIVE trigger that makes procgen drive real battles. The authored
//! [`Situation`] now carries only `theme` + `grid_size` + the placed gangers (GTW-433 C1:
//! `skirmish.ron` no longer authors inline terrain); the terrain is GENERATED here by
//! calling the sim-owned [`generate_level`] and pouring its result back over the authored
//! gangers.
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
//! emit). [`procgen_battle_situation`] takes that procgen terrain and the AUTHORED gangers /
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
//! and reaches `BattleRunning`. The real GUI path always has the loaded registry + a
//! theme+size situation, so procgen runs and produces a playable level; the fallback only
//! covers the no-content harnesses.

use bevy::prelude::warn;
use gdtf_battle_sim::{
    level::{LevelTheme, PrefabRegistry},
    procgen::{ProcgenTuning, generate_level},
    rng::{BattleSeed, ProcgenRng},
    situation::Situation,
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        piece::{TerrainKindSpec, TerrainName, TerrainRegistry, TerrainSpec},
    },
};

/// Build the battle's situation by running procgen terrain over the authored situation's
/// gangers / spawn data (GTW-433 C2/C3), or returning the authored situation unchanged on a
/// procgen failure / missing registry (C4 fallback).
///
/// Drives the sim-owned [`generate_level`] with the authored situation's `theme` +
/// `grid_size`, a [`ProcgenRng`] derived from `seed` ([`ProcgenRng::from_root`] — the
/// deterministic injected per-battle seed, C2), and the RULED-default [`ProcgenTuning`]
/// (the live hot-reload wiring of the procgen tuning was deferred with its consumer; the
/// defaults are the shipped knobs). On `Ok` it MERGES the generated terrain with the
/// authored gangers via [`merge_procgen_terrain`]; on `Err` (or `registry: None`) it logs
/// and returns `authored` unchanged.
///
/// `registry` is `Option` because a setup could be requested before the `Load`-scene
/// [`PrefabRegistry`] is present (a no-content headless harness); `None` ⇒ the authored
/// situation is used as-is (no terrain to generate against).
#[must_use]
pub(in crate::states::running::game::battlescape::generation::battle_sim) fn procgen_battle_situation(
    authored: Situation,
    registry: Option<&PrefabRegistry>,
    seed: BattleSeed,
) -> Situation {
    // No registry loaded (a no-content harness) → nothing to generate against; use the
    // authored situation as-is (fail-open to the authored terrain, never panic).
    let Some(registry) = registry else {
        return authored;
    };

    // The procgen RNG is derived from the SAME injected per-battle seed the rest of the
    // battle RNG uses (C2) — deterministic, reproducible, no wall-clock / thread_rng here.
    let mut rng = ProcgenRng::from_root(seed);
    // The RULED-default procgen knobs (the live tuning hot-reload was deferred with its
    // consumer; the defaults are the shipped values — see procgen `tuning.rs`).
    let tuning = ProcgenTuning::default();

    // GTW-491 SHIM: the legacy `generate_level` is still keyed by the closed `LevelTheme`
    // enum, but `Situation.theme` switched to a UUID-keyed `ThemeUuid` (T07a). Until the
    // procgen switch onto UUID-keyed v2 prefabs (GTW-492), this app-side glue drives procgen
    // with the default `LevelTheme` (IndustrialHive — the only shipped prefab family). The
    // real GUI path's situation theme resolves to that family; the headless harnesses seed an
    // EMPTY registry and take the C4 fallback below, so the shimmed theme never matters there.
    let procgen_theme = LevelTheme::default();

    match generate_level(
        registry,
        procgen_theme,
        authored.grid_size,
        &mut rng,
        &tuning,
    ) {
        Ok(generated) => merge_procgen_terrain(authored, generated),
        Err(err) => {
            // Fail closed to the authored terrain (C4): the headless deep-walk seeds an
            // EMPTY registry, so procgen cannot assemble a level — the authored (often
            // empty) situation still sets up and reaches BattleRunning. The real GUI path
            // always has loaded prefabs, so this never fires there.
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
/// values are what `generated` was assembled against), so they carry through unchanged from
/// `authored`.
fn merge_procgen_terrain(authored: Situation, generated: Situation) -> Situation {
    Situation {
        // Authored placement data — the rosters + spawn the situation file owns.
        gangers:        authored.gangers,
        player_faction: authored.player_faction,
        // GTW-491: `theme` is now the UUID-keyed `ThemeUuid` — authored metadata that carries
        // through from the authored situation; procgen does not generate it. (The emit-side
        // `generated.theme` is the SHIM-bridged default theme UUID, deliberately discarded
        // here in favour of the authored one.)
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

/// GTW-491 procgen SHIM ADAPTER: re-key every legacy [`TerrainRegistry`] piece into a
/// UUID-keyed [`TerrainDef`] under [`TerrainUuid::from_legacy_name`], inserting any not already
/// present into the live [`TerrainDefRegistry`].
///
/// WHY this exists (logged for `/gate`): GTW-491 (T07a) switched the sim's terrain RESOLUTION
/// onto the UUID-keyed [`TerrainDefRegistry`], but procgen ([`generate_level`] / `emit_level`)
/// still reads the legacy `TerrainName`-keyed prefab fragments — so it emits terrain references
/// minted via [`TerrainUuid::from_legacy_name`] (the same deterministic bridge). Those synthetic
/// UUIDs are NOT in the migrated (GTW-490) `TerrainDefRegistry` (which is keyed by AUTHORED
/// UUIDs), so without this adapter `setup_battle` would fail closed with `TerrainNotFound` on
/// every procgen-driven battle. This adapter mirrors the emit shim exactly (the SAME
/// `from_legacy_name` keying) so the procgen-emitted terrain resolves. GTW-492 (T07b) switches
/// procgen onto UUID-keyed v2 prefabs and REMOVES both this adapter and the emit shim.
///
/// Only INSERTS missing keys — never overwrites a migrated def already in the registry — so a
/// real UUID-authored def always wins. Per-piece mapping: a legacy `Wall` → `TerrainSimKind::Wall`,
/// `Cover`/`Scatter` → `Cover`, `Slab`/`Floor` → `Slab` (the new model has no `Floor` sim-kind —
/// a walkable floor is a slab). The presenter graphic comes from the legacy `graphic`; only a
/// `Slab` carries the legacy `footfall`.
pub(in crate::states::running::game::battlescape::generation::battle_sim) fn shim_legacy_terrain_into_def_registry(
    legacy: &TerrainRegistry,
    def_registry: &mut TerrainDefRegistry,
) {
    for (name, spec) in legacy.iter() {
        let key = TerrainUuid::from_legacy_name(name);
        if def_registry.def(&key).is_some() {
            continue;
        }
        def_registry.insert(key, legacy_spec_to_def(key, name, spec));
    }
}

/// Build a shim [`TerrainDef`] from a legacy [`TerrainSpec`] under `key` — the GTW-491 mapping
/// (see [`shim_legacy_terrain_into_def_registry`]).
fn legacy_spec_to_def(key: TerrainUuid, name: &TerrainName, spec: &TerrainSpec) -> TerrainDef {
    use gdtf_battle_sim::slab::SlabHp;

    let graphic = spec.graphic.clone();
    let (sim_kind, presenter_kind) = match &spec.kind {
        TerrainKindSpec::Wall(s) => (
            TerrainSimKind::Wall {
                hp:               s.max_hp,
                armor_protection: s.armor_protection,
                armor_hardness:   s.armor_hardness,
                height_band:      s.height_band,
            },
            TerrainPresenterKind::Wall {
                graphic_name: graphic,
            },
        ),
        TerrainKindSpec::Cover(s) | TerrainKindSpec::Scatter(s) => (
            TerrainSimKind::Cover {
                hp:               s.max_hp,
                armor_protection: s.armor_protection,
                armor_hardness:   s.armor_hardness,
                height_band:      s.height_band,
            },
            TerrainPresenterKind::Cover {
                graphic_name: graphic,
            },
        ),
        TerrainKindSpec::Slab(s) => (
            TerrainSimKind::Slab {
                hp:               s.max_hp,
                armor_protection: s.armor_protection,
                armor_hardness:   s.armor_hardness,
            },
            TerrainPresenterKind::Slab {
                graphic_name: graphic,
                footfall:     Some(spec.footfall.clone()),
            },
        ),
        TerrainKindSpec::Floor(_) => (
            // The new model has no Floor sim-kind: a walkable floor is a Slab def. The legacy
            // floor authored only a move cost (no HP/armor), so the shim slab uses neutral
            // structural stats — the floor's move cost is consumed via the fallback path
            // (GTW-482 owns the per-floor move-cost seam).
            TerrainSimKind::Slab {
                hp:               SlabHp::new(60),
                armor_protection: gdtf_battle_sim::ArmorProtection::new(0),
                armor_hardness:   gdtf_battle_sim::ArmorHardness::new(0),
            },
            TerrainPresenterKind::Slab {
                graphic_name: graphic,
                footfall:     Some(spec.footfall.clone()),
            },
        ),
    };
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new((**name).clone()),
        sim_kind,
        presenter_kind,
        tags: Vec::new(),
    }
}
