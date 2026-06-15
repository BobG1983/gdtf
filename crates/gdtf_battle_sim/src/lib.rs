//! Authoritative, render-free combat simulation for GDTF's turn-based battle
//! system — **the MODEL**.
//!
//! This crate owns combat truth: deterministic, unit-testable with an injected
//! seeded RNG ([`rng::SimRng`]), and presentation-agnostic — it reasons in the
//! cubic-voxel sim metric ([`metric`]: cells on x/y, levels on z, over the
//! 60×60×8 grid), **never in pixels**. It depends on Bevy only for ECS plumbing
//! (resources, components, messages) — never a renderer, window, or asset-server.
//! The presenter (`gdtf_battle_presenter`) mirrors this state and owns all
//! sim→view projection; combat rules never live in the view, and the model never
//! reads the presenter.
//!
//! ## Module map
//!
//! - [`metric`] — the cubic-voxel coordinate system ([`metric::SimPos`],
//!   [`metric::Cell`] / [`metric::Level`] / [`metric::CellLevel`],
//!   [`metric::cell_center`] / [`metric::pos_to_cell`]).
//! - [`tuning`] — the [`tuning::CombatTuning`] resource: every balance
//!   coefficient, serde-loaded from `assets/combat/tuning.ron`.
//! - [`tu`] — the TU-economy primitives the E4 acts spend through:
//!   [`tu::can_spend_tu`] / [`tu::spend_tu`] (saturating) / [`tu::reset_tu`] over a
//!   ganger's [`ganger::Tu`] (current pool) and [`ganger::TuMax`] (round-start max —
//!   the denominator of GTW-38's `TU_left / TU_max` reaction ratio). Pure math, no
//!   `World` access; the COST magnitudes are tuning sourced by later E4 slices.
//! - [`posture`] — the E4.1 posture / orientation verbs over the landed ganger
//!   components: [`posture::set_aiming`] (a pure aim-flag setter — charges NO TU; the
//!   aim cost is the fire-time ×1.5 premium, resolution.md §1a), [`posture::set_stance`]
//!   (charges [`tuning::StanceChangeTu`] via [`tu::spend_tu`] on a real posture change),
//!   and [`posture::set_facing`] (charges [`tuning::TurnTu`] on a real turn). Each
//!   costed verb is a no-op — no charge — when the value is unchanged. Pure math, no
//!   `World` access.
//! - [`ganger`] — per-field ganger battle-state components (including the
//!   [`ganger::Stabilized`] bleed-out flag, owned here from E3.7); [`armor`] —
//!   armor stats + the battle-local [`armor::WornArmor`]; [`armor_wear`] —
//!   persisting a hit's [`resolve_hit::IntegrityWear`] onto the worn copy
//!   ([`armor_wear::wear_armor`]) + the [`armor_wear::ArmorBroken`] message on the
//!   protecting→broken crossing.
//! - [`bleed`] — the §9 bleed-out clock: [`bleed::tick_bleed`] drains a flat
//!   tuning [`tuning::BleedRate`] of [`ganger::Wounds`] per round from each
//!   un-stabilized [`ganger::LifeState::Downed`] ganger, emits the
//!   [`bleed::Bleeding`] message, and runs the once-only terminal gate to
//!   [`ganger::LifeState::Dead`] on depletion.
//! - [`downed_acts`] — the §9 from-Downed verbs + their shared faction-aware
//!   predicates: [`downed_acts::can_stabilize`] / [`downed_acts::stabilize_downed`]
//!   (an 8-adjacent ALIVE ally halts the bleed clock by setting [`ganger::Stabilized`],
//!   the ganger staying Downed) and [`downed_acts::can_execute`] /
//!   [`downed_acts::execute_downed`] (an 8-adjacent ALIVE enemy finishes a Downed
//!   ganger outright → [`ganger::LifeState::Dead`]), over the same-level Moore-8
//!   [`downed_acts::is_8_adjacent`] reach. Each act is a no-op exactly when its
//!   predicate is false (button ⇔ act share one guard); the flat TU costs
//!   ([`tuning::StabilizeTu`] / [`tuning::ExecuteTu`]) are READ, not debited (E4).
//! - Terrain & space: [`cover`] (the [`cover::CoverLedger`] + [`cover::HeightBand`]
//!   banding), [`surface`] (persistent floor/roof-slab + ground grid),
//!   [`occupancy`] + [`occupancy_sync`] (the coarse 3D occupancy grid and its
//!   change-driven in-place maintenance), [`vertical`] (the stair/ladder link graph).
//! - [`situation`] — the authored [`situation::Situation`] + [`situation::setup_battle`].
//! - [`rng`] — the model-owned seeded [`rng::SimRng`] (the single draw point).
//! - The shot pipeline ([`resolve_coarse::resolve_coarse`] composes it): [`weapon`]
//!   data → [`stability`] → [`cone`] (cone width) → [`central_axis`] (muzzle / aim /
//!   recoil-climb axis) → [`sample_cone`] (the in-cone shot vector) → [`clearance`] +
//!   [`march`] (the 3-axis voxel-DDA travel) → [`hit_location`] (the part roll) →
//!   a [`resolve_coarse::ShotOutcome`].
//! - [`matchup`] — the 7-type weapon×armor Paley-tournament lookup
//!   ([`matchup::matchup`] over [`weapon::DamageType`] / [`armor::ArmorType`]) and
//!   the punch-&-shred [`matchup::MatchupMultiplier`] it yields from tuning.
//! - [`resolve_hit`] — the per-hit damage/penetration formula
//!   ([`resolve_hit::resolve_hit`]) that resolves weapon damage stats vs one
//!   [`armor::ArmorPiece`] under a [`matchup::Matchup`] into a frozen
//!   [`resolve_hit::HitResult`] (penetrating / HP-loss / integrity-wear). Pure
//!   math — application of HP / Wounds / armor wear is a later E3 slice.
//! - [`severity`] — the §6 wound-severity roll ([`severity::roll_severity`]):
//!   the penetration-gated score → [`severity::Severity`] bucket in the
//!   floor-extend form (the defender's Luck extends the roll's floor down). Pure
//!   seeded-RNG math; the per-tier Wounds application is a later E3 slice.
//! - [`resolve_and_apply`] — the E3.9 capstone integrator
//!   ([`resolve_and_apply::resolve_and_apply`]): folds one [`resolve_coarse::ShotOutcome`]
//!   through matchup → [`resolve_hit`] → [`severity`] → [`apply_hit`] into ONE act
//!   and returns a FROZEN [`resolve_and_apply::HitReport`] for the presenter's FX
//!   (corpse-skip before any draw; every draw via the injected [`rng::SimRng`]; no
//!   pixel). Charging TU / looping the burst (`fire()`) is E4.
//!
//! Design canon: `docs/combat/` (notably `battle-space.md`, `resolution.md`) and
//! `docs/architecture.md` — the model/view split this crate sits inside.

pub mod apply_hit;
pub mod armor;
pub mod armor_wear;
pub mod bleed;
pub mod central_axis;
pub mod clearance;
pub mod cone;
pub mod cover;
pub mod downed_acts;
pub mod ganger;
pub mod hit_location;
pub mod march;
pub mod matchup;
pub mod metric;
pub mod occupancy;
pub mod occupancy_sync;
pub mod posture;
pub mod resolve_and_apply;
pub mod resolve_coarse;
pub mod resolve_hit;
pub mod rng;
pub mod sample_cone;
pub mod severity;
pub mod situation;
pub mod stability;
pub mod surface;
pub mod tu;
pub mod tuning;
pub mod vertical;
pub mod weapon;

pub use apply_hit::{GangerHitTarget, apply_hit};
pub use armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType, BodyPart,
    SourceArmor, WornArmor,
};
pub use armor_wear::{ArmorBroken, wear_armor};
pub use bleed::{Bleeding, tick_bleed};
pub use central_axis::{AimDir, climb_aim_dir, muzzle_position, target_aim_point};
pub use clearance::{Clearance, round_band_for_cell, round_band_fraction, round_clears_occupant};
pub use cone::{ConeAngle, PriorShots, RecoilFactor, aim_cone_mult, cone_angle, recoil_factor};
pub use cover::{
    BandFraction, CoverDamage, CoverEntry, CoverEvent, CoverHp, CoverLedger, Destroyed, HeightBand,
    band_for,
};
pub use downed_acts::{
    Actor, DownedTarget, can_execute, can_stabilize, execute_downed, is_8_adjacent,
    stabilize_downed,
};
pub use ganger::{
    Aiming, Direction, Facing, Faction, Hp, LifeState, Luck, Position, Shooting, Stabilized,
    Stance, StanceKind, Toughness, Tu, TuMax, Wounds,
};
pub use hit_location::roll_body_part;
pub use march::{MarchKind, MarchResult, march_vector};
pub use matchup::{Matchup, MatchupMultiplier, WheelNode, matchup, matchup_multiplier};
pub use metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos, cell_center, pos_to_cell};
pub use occupancy::{
    DestroyedCover, GRID_HEIGHT, GRID_WIDTH, OccupancyGrid, OccupancyInput, OccupancySlot,
    OccupantPlacement, TerrainKind, TerrainPlacement,
};
pub use occupancy_sync::{
    CoverDestroyed, OccupancyMaintenancePlugin, PrevSlot, sync_dead_gangers, sync_destroyed_cover,
    sync_moved_gangers,
};
pub use posture::{set_aiming, set_facing, set_stance};
pub use resolve_and_apply::{AppliedDamage, HitReport, TargetGanger, resolve_and_apply};
pub use resolve_coarse::{ShotInputs, ShotKind, ShotOutcome, resolve_coarse};
pub use resolve_hit::{HitResult, HpDamage, IntegrityWear, PenetratingDamage, resolve_hit};
pub use rng::{BattleSeed, SimRng};
pub use sample_cone::{ConcentrationP, ShotDir, concentration_p, sample_cone_vector};
pub use severity::{PartSeverityMod, Severity, SeverityInputs, part_severity_mod, roll_severity};
pub use situation::{
    BattleSetup, CoverSpawn, GangerSpawn, Situation, has_stacked_gangers, setup_battle,
};
pub use stability::{
    ConeMult, EmplacementStability, RecoilGrowth, StabilityScore, WeaponStability, stability,
};
pub use surface::{GroundDamage, SlabState, SurfaceGrid};
pub use tu::{can_spend_tu, reset_tu, spend_tu};
pub use tuning::{
    AimConeMult, AimHeightFrac, AimMode, AimTuPremium, BandEdge, BleedRate, BodyPartWeight,
    BodyPartWeights, BraceContribution, BraceMinHeight, CombatTuning, ConcentrationCoeff,
    ConcentrationCoeffs, ConeStabilityTuning, DefenderLuckScale, ExecuteTu, MatchupMultipliers,
    MuzzleForwardOffset, MuzzleHeight, MuzzleHeights, PenDamageScale, ProjectileBandEdges,
    RandomSpread, RecoilClimb, SeverityEdge, SeverityEdges, SeverityScaling, ShooterLuckScale,
    SilhouetteTop, SilhouetteTops, StabilityCurve, StabilityCurveCoord, StabilityCurvePoint,
    StabilityCurves, StabilizeTu, StanceChangeTu, StanceContribution, StanceStability,
    ToughnessMitigation, TurnTu, WoundCost, WoundCosts,
};
pub use vertical::{
    InvalidVerticalLink, LinkKind, OneWay, VerticalLink, VerticalLinkGraph,
    build_vertical_link_graph,
};
pub use weapon::{
    Accuracy, BaseSpread, DamageType, FatalBias, FireMode, FireModeSpec, Kickback, MagazineSize,
    ModeConeMult, ModeShots, ModeTuPercent, Weapon, WeaponDamage, WeaponDamageProfile, WeaponPunch,
    WeaponShred,
};
