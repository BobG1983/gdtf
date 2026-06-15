//! Authoritative, render-free combat simulation for GDTF's turn-based battle
//! system — the MODEL.
//!
//! This crate owns combat truth: it is deterministic and unit-testable with an
//! injected seeded RNG, and it depends on Bevy only for ECS plumbing
//! (MinimalPlugins-compatible) — never on a renderer, window, or asset-server.
//! The presenter (`gdtf_battle_presenter`) mirrors this state; combat rules
//! never live in the view.
//!
//! E1.1 lays the foundation: the [`metric`] cubic-voxel battle-space coordinate
//! system (newtypes + [`metric::MAX_LEVELS`] + the [`metric::cell_center`] /
//! [`metric::pos_to_cell`] conversions) and the [`tuning`] combat-tuning
//! resource. See `docs/combat/battle-space.md` and `docs/combat/resolution.md`.
//!
//! E1.2 ([`ganger`]) decomposes ganger battle state into nine **separate**
//! per-field ECS components so a system can query any subset independently — see
//! `docs/combat/combat.md`, `wounds-and-roster.md`, and `stats.md`.
//!
//! E1.3 ([`armor`]) adds the four armor-stat newtypes, an [`armor::ArmorPiece`]
//! per body location, the read-only roster [`armor::SourceArmor`] record, and the
//! battle-local [`armor::WornArmor`] component seeded by value from it — the sim's
//! only mutable armor surface during a battle. See
//! `docs/combat/weapons-and-armor.md` and `docs/architecture.md`.
//!
//! E1.4 ([`cover`]) adds the [`cover::CoverLedger`] resource — the single
//! authoritative store of cover structural HP, ONE unified map keyed
//! `(cell, level)` for BOTH walls and props, lazily seeded to `max_hp` on first
//! access. [`cover::CoverLedger::deplete_cover`] spends HP and emits a
//! [`cover::CoverEvent::Destroyed`] marker at zero (the occupancy/prop
//! consequences are deferred to GTW-35). Cover reuses the GTW-153 armor newtypes,
//! and band thresholds come from [`tuning::CombatTuning`] via [`cover::band_for`].
//! See `docs/combat/resolution.md` §3 and `docs/architecture.md`.
//!
//! E1.5 ([`surface`]) adds the [`surface::SurfaceGrid`] resource — the persistent
//! store of floor/roof slab existence ([`surface::SlabState`] per [`metric::CellLevel`])
//! and per-[`metric::Cell`] ground damage ([`surface::GroundDamage`]), mutated **in
//! place** so it survives every occupancy rebuild. A destroyed slab stays destroyed
//! ([`surface::SurfaceGrid::destroy_slab`] is terminal) and ground damage only
//! accrues ([`surface::SurfaceGrid::accrue_ground_damage`] is monotonic). This is a
//! SEPARATE resource from the coarse occupancy (E1.6 / GTW-156). See
//! `docs/architecture.md`'s "persistent surface grid" + the surface/ground-hit verbs.
//!
//! E1.6 ([`occupancy`]) adds the [`occupancy::OccupancyGrid`] resource — the coarse
//! 3D collision/query surface ([`occupancy::GRID_WIDTH`] × [`occupancy::GRID_HEIGHT`]
//! × [`metric::MAX_LEVELS`] = 60×60×8). Each `(cell, level)`
//! [`occupancy::OccupancySlot`] carries a [`occupancy::TerrainKind`] static-terrain
//! marker (wall / cover, blocking vs not) and an occupant
//! `Option<`[`bevy::prelude::Entity`]`>` (a Bevy `Entity` handle, NEVER a numeric id —
//! GTW-10 / GTW-12). [`occupancy::OccupancyGrid::build_from_occupancy_input`] pours a
//! grid-relevant [`occupancy::OccupancyInput`] (terrain + occupant placements) into a
//! fresh grid, and the append-only [`occupancy::DestroyedCover`] set excludes smashed
//! cover from [`occupancy::OccupancyGrid::is_blocked`]. This is the occupancy grid's
//! OWN exclusion set, distinct from [`cover::CoverLedger`] (GTW-157 syncs them). See
//! `docs/architecture.md`'s "coarse occupancy" + `battle-space.md`.
//!
//! E1.7 ([`occupancy_sync`]) adds the **change-driven** maintenance layer for the
//! [`occupancy::OccupancyGrid`]: three focused Bevy systems that edit the grid IN
//! PLACE (never a per-shot rebuild — `docs/architecture.md`'s "change-driven grid
//! maintenance", the GTW-6 / GTW-12 ruling). [`occupancy_sync::sync_moved_gangers`]
//! reacts to `Changed<`[`ganger::Position`]`>` (tracking the prior slot in a
//! [`occupancy_sync::PrevSlot`] component) to clear the OLD slot and mark the NEW;
//! [`occupancy_sync::sync_dead_gangers`] reacts to `Changed<`[`ganger::LifeState`]`>`
//! to free a downed / dead ganger's slot; [`occupancy_sync::sync_destroyed_cover`]
//! reads the buffered [`occupancy_sync::CoverDestroyed`] **message** (Bevy 0.18
//! messages, not the observer `Event` API) into the grid's destroyed-cover set.
//! [`occupancy_sync::OccupancyMaintenancePlugin`] is the registration unit (the
//! three chained systems + the message buffer); the app adds it when the sim is
//! wired into the runtime (E1.8 / E5), which is out of scope here.
//!
//! E1.9 ([`rng`]) adds the [`rng::SimRng`] resource — the model's single,
//! deterministic draw point. It wraps a private seeded `StdRng` (constructed
//! from a [`rng::BattleSeed`] via `seed_from_u64`, so the concrete RNG type
//! never escapes), and exposes a thin draw surface plus an `&mut impl rand::Rng`
//! handle for the `fn(.., rng: &mut impl Rng)` combat-math shape. There is NO
//! global/thread RNG anywhere in the sim — `docs/combat/resolution.md`'s "every
//! draw comes from the model-owned seeded RNG, injected once at setup", pinned
//! by `docs/testing.md`'s same-seed-same-stream property (and a source scan).
//!
//! E1.10 ([`vertical`]) adds the [`vertical::VerticalLinkGraph`] resource — the
//! validated index of authored stair / ladder [`vertical::VerticalLink`]s, the
//! ONLY way a ganger changes storey (`docs/combat/combat.md`). The authored list
//! lives on the canonical [`situation::Situation::vertical_links`], and
//! [`vertical::build_vertical_link_graph`] validates each authored link at setup
//! (level in `0..`[`metric::MAX_LEVELS`]; no dangling endpoint cell — checked
//! against [`situation::Situation::authored_cells`]; the two endpoints on different
//! storeys) — returning a typed [`vertical::InvalidVerticalLink`], NEVER a panic —
//! before indexing each valid link by departure `(cell, level)` (both directions
//! unless [`one-way`](vertical::LinkKind::is_one_way)). Graph + validation ONLY: no
//! traversal / pathfinding / movement cost (GTW-12). See `docs/architecture.md`'s
//! "vertical-link graph".
//!
//! E1.8 ([`situation`]) defines the **canonical authored** [`situation::Situation`]
//! — the full battlefield (gangers, walls, scatter, slabs, vertical links) — and
//! [`situation::setup_battle`], which reads it and builds the battle in the ECS
//! world: it `spawn`s each ganger with ALL the E1.2 components plus the E1.3
//! [`armor::WornArmor`] (seeded from the ganger's [`armor::SourceArmor`]), keeping
//! the returned Bevy [`bevy::prelude::Entity`] handle (NEVER a numeric id — GTW-10 /
//! GTW-12); seeds the [`cover::CoverLedger`] (E1.4) from walls + scatter, the
//! [`surface::SurfaceGrid`] (E1.5) from the slabs, and the
//! [`occupancy::OccupancyGrid`] (E1.6) from the authored terrain + the SPAWNED
//! occupant entities; and validates + inserts the [`vertical::VerticalLinkGraph`]
//! (E1.10). The [`situation::Situation`] type SUPERSEDES the GTW-156 placeholder
//! (renamed to [`occupancy::OccupancyInput`]). See `docs/architecture.md`'s setup
//! systems.
//!
//! E2.1 ([`weapon`] + the [`tuning::ConeStabilityTuning`] extension) lays the §1
//! cone/stability/recoil/aim **data substrate** the rest of E2 reads — types +
//! serde only, no math. [`weapon::Weapon`] carries the per-weapon NUMBERS
//! (`base_spread`, `accuracy`, `kickback`, `fatal_bias` [carried, consumed by E3],
//! `magazine_size`) and a [`weapon::FireMode`] selector (single / single+burst /
//! single+burst+full-auto, each mode a [`weapon::FireModeSpec`] of cone-mult / TU%
//! / shots); [`tuning::ConeStabilityTuning`] carries the universal COEFFICIENTS
//! (the stance + auto-brace stability contributions, the per-stance brace
//! min-height gate, the two stability curves, the aim-mode cone mult + TU premium,
//! the recoil-climb coefficient, the concentration-p coefficients, and the de-pxed
//! muzzle/aim geometry — [`tuning::AimHeightFrac`], [`tuning::MuzzleForwardOffset`]
//! [a cell-fraction], and the per-stance muzzle + silhouette-top level-fractions).
//! Weapon numbers live on the weapon; coefficients live in tuning
//! (`docs/combat/resolution.md` §1 + §"Coefficients live in the combat-tuning
//! data"; `docs/combat/battle-space.md` §"Stance / cover / muzzle / aim heights").
//!
//! E2.2 ([`stability`]) builds the §1a **stability layer** on the E2.1 substrate:
//! [`stability::stability`] sums the weapon-intrinsic, per-stance, automatic-brace,
//! and emplacement contributions into one **continuous 0–100 score**
//! (clamped/normalised over 100), then reads BOTH [`tuning::StabilityCurves`] at that
//! score, returning the named [`stability::ConeMult`] (steadier → narrower) and
//! [`stability::RecoilGrowth`] (steadier → climbs strictly less) pair. The auto-brace
//! is granted EXACTLY when the faced cell's [`cover::CoverEntry::height_band`] (read
//! directly off the entry — NOT via [`cover::band_for`]) satisfies the per-stance
//! [`tuning::BraceMinHeight`] gate (prone↔LOW+, kneel↔MID+, stand↔HIGH). Every
//! coefficient and both curves come from [`tuning::ConeStabilityTuning`]; angular /
//! dimensionless — zero pixels. See `docs/combat/resolution.md` §1a + "What's pure
//! math vs sim" line 148, and `docs/combat/battle-space.md` §"Banding".
//!
//! E2.3 ([`cone`]) builds the §1a **cone-size** calculation on the E2.1/E2.2
//! substrate: [`cone::cone_angle`] returns the named [`cone::ConeAngle`] (radians)
//! as the PRODUCT of the five multiplicative factors `base_spread × stability ×
//! aim × firemode × recoil` (resolution.md §1a; "What's pure math vs sim" line
//! 147). `stability` is the E2.2 [`stability::ConeMult`] (steadier < 1); `aim` is
//! the Aim-Mode multiplier ([`tuning::AimConeMult`], ×0.6 aimed / 1 hip-fired)
//! selected from [`tuning::AimMode`] by the ganger's [`ganger::Aiming`] flag via
//! [`cone::aim_cone_mult`]; `firemode` is the per-mode [`weapon::ModeConeMult`]
//! read off the weapon's [`weapon::FireMode`]; and `recoil = 1 +
//! `[`cone::PriorShots`]` × kickback` (the first round → ×1) via
//! [`cone::recoil_factor`], with `kickback` from the weapon. This is the cone
//! WIDTH only (the §1b in-cone sample is E2.5). Because the factors multiply,
//! bracing tightens PROPORTIONALLY — a steadier stability shrinks a sloppy weapon
//! by more absolute angle than a tight one. Angular / dimensionless — zero pixels;
//! no cone-factor magnitude is hardcoded (every term reads from weapon / tuning
//! data). See `docs/combat/resolution.md` §1a.
//!
//! E2.4 ([`central_axis`]) builds the §1 **central-axis geometry** in sim units /
//! level-fractions on the E2.1/E2.2 substrate (and extends [`ganger::Direction`]
//! with [`ganger::Direction::forward_step`], each facing's ground-plane unit step).
//! [`central_axis::muzzle_position`] returns a [`metric::SimPos`] at the shooter's
//! [`metric::cell_center`] plus the per-facing forward offset
//! ([`tuning::MuzzleForwardOffset`] cell-fraction along the facing's `forward_step`,
//! CLAMPED within the shooter's cell), with `z = level + the per-stance muzzle
//! level-fraction` ([`tuning::MuzzleHeights`]). [`central_axis::target_aim_point`]
//! returns the target's `cell_center` with `z` derived from
//! [`tuning::ProjectileBandEdges`]: a ganger target pins its stance silhouette-top
//! band-top fraction times [`tuning::AimHeightFrac`], a cover-occupied cell pins the
//! cover [`cover::HeightBand`]'s midpoint level-fraction (both from the band edges,
//! never a literal). [`central_axis::climb_aim_dir`] tilts the unit muzzle→aim axis
//! UP (`+z`) by `prior_shots × recoil_climb × recoil_growth` radians (zero prior
//! shots → the untilted axis exactly), returning the unit-direction newtype
//! [`central_axis::AimDir`]. All vertical/sub-cell datums are
//! level-fractions/cell-fractions — zero pixels. See `docs/combat/resolution.md` §1
//! and "What's pure math vs sim" line 149, plus `docs/combat/battle-space.md`
//! §"Stance / cover / muzzle / aim heights" and §"Sub-cell precision on the ground
//! plane".
//!
//! E2.8 ([`hit_location`]) builds the §4 **weighted hit-location roll**:
//! [`hit_location::roll_body_part`] picks one of the six [`armor::BodyPart`]s by a
//! weighted draw over the EXISTING tuning [`tuning::BodyPartWeights`] (consumed,
//! not added) — no per-part geometry, the coarse march's band clearance already
//! decided *which* ganger and this chance roll decides *where* (the retired
//! on-silhouette / exposure model). The single draw comes from the injected
//! `&mut impl rand::Rng` (the [`rng::SimRng`] handle) so it is deterministic and
//! seed-replayable; an all-six-zero weights total falls back to
//! [`armor::BodyPart::Torso`] without panicking. See `docs/combat/resolution.md`
//! §4 + "What's pure math vs sim" line 152, and `docs/testing.md`'s seeded
//! distribution-test discipline (ORDERING, never magnitudes).

pub mod armor;
pub mod central_axis;
pub mod cone;
pub mod cover;
pub mod ganger;
pub mod hit_location;
pub mod metric;
pub mod occupancy;
pub mod occupancy_sync;
pub mod rng;
pub mod situation;
pub mod stability;
pub mod surface;
pub mod tuning;
pub mod vertical;
pub mod weapon;

pub use armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, BodyPart, SourceArmor,
    WornArmor,
};
pub use central_axis::{AimDir, climb_aim_dir, muzzle_position, target_aim_point};
pub use cone::{ConeAngle, PriorShots, RecoilFactor, aim_cone_mult, cone_angle, recoil_factor};
pub use cover::{
    BandFraction, CoverDamage, CoverEntry, CoverEvent, CoverHp, CoverLedger, Destroyed, HeightBand,
    band_for,
};
pub use ganger::{
    Aiming, Direction, Facing, Faction, Hp, LifeState, Position, Stance, StanceKind, Tu, Wounds,
};
pub use hit_location::roll_body_part;
pub use metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos, cell_center, pos_to_cell};
pub use occupancy::{
    DestroyedCover, GRID_HEIGHT, GRID_WIDTH, OccupancyGrid, OccupancyInput, OccupancySlot,
    OccupantPlacement, TerrainKind, TerrainPlacement,
};
pub use occupancy_sync::{
    CoverDestroyed, OccupancyMaintenancePlugin, PrevSlot, sync_dead_gangers, sync_destroyed_cover,
    sync_moved_gangers,
};
pub use rng::{BattleSeed, SimRng};
pub use situation::{
    BattleSetup, CoverSpawn, GangerSpawn, Situation, has_stacked_gangers, setup_battle,
};
pub use stability::{
    ConeMult, EmplacementStability, RecoilGrowth, StabilityScore, WeaponStability, stability,
};
pub use surface::{GroundDamage, SlabState, SurfaceGrid};
pub use tuning::{
    AimConeMult, AimHeightFrac, AimMode, AimTuPremium, BandEdge, BodyPartWeight, BodyPartWeights,
    BraceContribution, BraceMinHeight, CombatTuning, ConcentrationCoeff, ConcentrationCoeffs,
    ConeStabilityTuning, DefenderLuckSpreadCap, MuzzleForwardOffset, MuzzleHeight, MuzzleHeights,
    PenDamageScale, ProjectileBandEdges, RandomSpread, RandomSpreadMin, RecoilClimb,
    SeverityScaling, ShooterLuckScale, SilhouetteTop, SilhouetteTops, StabilityCurve,
    StabilityCurveCoord, StabilityCurvePoint, StabilityCurves, StanceContribution, StanceStability,
    ToughnessMitigation,
};
pub use vertical::{
    InvalidVerticalLink, LinkKind, OneWay, VerticalLink, VerticalLinkGraph,
    build_vertical_link_graph,
};
pub use weapon::{
    Accuracy, BaseSpread, FatalBias, FireMode, FireModeSpec, Kickback, MagazineSize, ModeConeMult,
    ModeShots, ModeTuPercent, Weapon,
};

#[cfg(test)]
mod pixel_scan_tests {
    //! GTW-174 AC #1: a source scan confirming **zero residual pixel concept** in
    //! the sim crate. The metric is cubic-voxel sim units; the presenter owns all
    //! sim→view scaling, so no pixel token may appear anywhere in `src/`.

    use std::{fs, path::Path};

    /// Recursively collect every `.rs` file under `dir` into `out`.
    fn collect_rs(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(read) = fs::read_dir(dir) else {
            return;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rs(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// No pixel concept survives anywhere in the sim source: not the two-letter
    /// pixel token, not the deleted coordinate constants, not the retired position
    /// newtype. Scans every `src/**/*.rs` (this file included) as text with
    /// comments stripped, so an honest doc-comment is never a false hit and a real
    /// re-introduction in code is caught.
    #[test]
    fn sim_source_has_no_pixel_concept() {
        // Forbidden strings built from FRAGMENTS so the literal token never appears
        // in this file's *code* (the scan would otherwise self-match — see the
        // engineer memory on source-scan self-match).
        let two_letter = ["p", "x"].concat();
        let deleted_pitch = ["CELL", "PITCH", &two_letter.to_uppercase()].join("_");
        let deleted_height = ["Z", "LEVEL", "HEIGHT"].join("_");
        let retired_pos = ["Battle", &two_letter[..1].to_uppercase(), &two_letter[1..]].concat();
        let forbidden = [
            two_letter.as_str(),
            deleted_pitch.as_str(),
            deleted_height.as_str(),
            retired_pos.as_str(),
        ];

        let manifest = env!("CARGO_MANIFEST_DIR");
        let src_root = Path::new(manifest).join("src");
        let mut files = Vec::new();
        collect_rs(&src_root, &mut files);
        assert!(!files.is_empty(), "scan must find source files under src/");

        let mut hits = Vec::new();
        for file in &files {
            let Ok(body) = fs::read_to_string(file) else {
                continue;
            };
            for (lineno, raw) in body.lines().enumerate() {
                // Strip the line from its first comment marker so a doc/inline
                // comment mentioning the concept is not a hit — only code counts.
                let marker = ["/", "/"].concat();
                let code = raw.split(&marker).next().unwrap_or(raw);
                let lower = code.to_ascii_lowercase();
                for token in &forbidden {
                    if lower.contains(&token.to_ascii_lowercase()) {
                        hits.push(format!("{}:{} :: {token}", file.display(), lineno + 1));
                    }
                }
            }
        }

        assert!(
            hits.is_empty(),
            "residual pixel concept in gdtf_battle_sim source (must be zero):\n{}",
            hits.join("\n"),
        );
    }
}
