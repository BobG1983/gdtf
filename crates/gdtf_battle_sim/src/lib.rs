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
//! - [`turn`] — the GTW-309 turn-cycle engine: the [`turn::ActiveFaction`] battle-lifetime
//!   resource (whose turn it is), the pure [`turn::regen_team_tu`] turn-start TU-regen
//!   helper (resets one team's [`ganger::Tu`] to [`ganger::TuMax`] via [`tu::reset_tu`]),
//!   and the [`turn::dispatch_end_turn`] system that drains the
//!   [`acts::EndTurnRequested`] signal and cycles the turn — handing off to the other team
//!   and (while the enemy has no AI) auto-passing the enemy turn back to the player.
//! - [`faced_cell`] — the E4.2 geometry helper [`faced_cell::faced_cell`]: shooter
//!   [`ganger::Position`] + [`ganger::Facing`] → the faced ([`metric::Cell`],
//!   [`metric::Level`]) one unit step along the facing (same storey). The cell whose
//!   cover the §1a brace gate (E4.3) reads; composes only the [`metric`] sim-unit
//!   conversions — render-free, no `World`, zero pixels.
//! - [`aim`] — the E4.3 §1a composers [`aim::stability_for`] / [`aim::cone_for`]:
//!   the shared HUD + `fire()` methods that compose the stability read and
//!   dispersion-cone WIDTH off a shooter's ganger state ([`aim::Shooter`] bundle)
//!   and the MODEL [`cover::CoverLedger`] (peeked at the E4.2 faced cell, never
//!   rebuilt). They WRAP the landed E2 [`stability`] / [`cone`] pipeline verbatim;
//!   the weapon's stability contribution is its [`weapon::Stable`] tag (`cone_for`
//!   takes the weapon stats as a [`weapon::WeaponStats`] borrow-view and threads the
//!   tag, an explicit param, to `stability_for`), which engages the §1a brace
//!   UNCONDITIONALLY — there is no weapon-points term. Angular / dimensionless,
//!   zero pixels.
//! - [`posture`] — the E4.1 posture / orientation verbs over the landed ganger
//!   components: [`posture::set_aiming`] (a pure aim-flag setter — charges NO TU; the
//!   aim cost is the fire-time ×1.5 premium, resolution.md §1a), [`posture::set_stance`]
//!   (charges [`tuning::StanceChangeTu`] via [`tu::spend_tu`] on a real posture change),
//!   and [`posture::set_facing`] (charges [`tuning::TurnTu`] on a real turn). Each
//!   costed verb is a no-op — no charge — when the value is unchanged. Pure math, no
//!   `World` access.
//! - [`move_acts`] — the GTW-355 committed walk [`move_acts::advance_walk`]: drives an
//!   accepted route ONE cell per tick, charging each entered cell its TERRAIN-DETERMINED
//!   TU cost (the cell's [`occupancy::TerrainKind`] movement cost from the per-terrain
//!   [`tuning::MoveCosts`] table, NOT a flat constant) atomically with the step. The route
//!   is planned + gated affordable up front by `dispatch_move`; each step bump-stops on
//!   live obstacles and halts on a reveal / reaction interrupt. It writes ONLY
//!   [`ganger::Position`] — the grid slot maintenance is the landed
//!   [`occupancy_sync::sync_moved_gangers`] `Changed<Position>` reactor.
//! - [`magazine`] — the ammo state + the shared firing guard: the
//!   [`magazine::Magazine`] GROUPING Component (GTW-275 — the [`weapon::MagazineSize`]
//!   capacity, the per-weapon [`magazine::ReloadTu`] reload cost, and the
//!   [`magazine::LoadedRounds`] live count, clamped to capacity at construction;
//!   saturating [`magazine::Magazine::spend_round`], the
//!   [`magazine::Magazine::refill`] reload, the [`magazine::clamp_burst`] burst
//!   primitive), the shared [`magazine::mode_tu_cost`] per-shot TU charge
//!   ([`ganger::TuMax`]-derived × [`weapon::ModeTuPercent`] × the aim premium when
//!   aiming — resolution.md §1 / §1a), and the
//!   [`magazine::can_fire`] guard set (over a [`magazine::FireActor`] bundle): alive +
//!   affords the mode TU + ≥1 round + [`magazine::in_bounds`] — NO LOS input (fog is
//!   presenter player policy). The TU-costed reload ACT is [`acts::dispatch_reload`]
//!   (it charges the magazine's own per-weapon `reload_tu`).
//! - [`ganger`] — per-field ganger battle-state components (including the
//!   [`ganger::Stabilized`] bleed-out flag, owned here from E3.7); [`armor`] —
//!   armor stats + the battle-local armor-piece entities related via
//!   [`armor::Wears`] (ADR-0004); [`armor_wear`] — persisting a hit's
//!   [`resolve_hit::IntegrityWear`] onto the struck piece entity's
//!   [`armor::ArmorIntegrity`] ([`armor_wear::wear_armor`]) + the
//!   [`armor_wear::ArmorBroken`] message on the protecting→broken crossing.
//! - [`bleed`] — the §9 bleed-out clock: [`bleed::tick_bleed`] drains a flat
//!   tuning [`tuning::BleedRate`] of [`ganger::Wounds`] per round from each
//!   un-stabilized [`ganger::LifeState::Downed`] ganger, emits the
//!   [`bleed::Bleeding`] message, and runs the once-only terminal gate to
//!   [`ganger::LifeState::Dead`] on depletion. The clock is wired into the live
//!   runtime by [`acts::SimActsPlugin`]: it runs `tick_bleed` once per full round
//!   at the enemy-phase start, gated on [`bleed::enemy_phase_started`] (GTW-336).
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
//! - [`weapon`] — the weapon as ECS components (GTW-200): a unit [`weapon::Weapon`]
//!   MARKER plus one `#[derive(Component)]` newtype per stat
//!   ([`weapon::WeaponName`] / [`weapon::BaseSpread`] / [`weapon::Accuracy`] /
//!   [`weapon::Kickback`] / [`weapon::FatalBias`] / [`weapon::WeaponDamage`] /
//!   [`weapon::WeaponPunch`] / [`weapon::WeaponShred`] / [`weapon::DamageType`] /
//!   the [`magazine::Magazine`] grouping (its [`weapon::MagazineSize`] /
//!   [`magazine::ReloadTu`] / [`magazine::LoadedRounds`] leaves, GTW-275) /
//!   [`weapon::FireMode`] / [`weapon::Stable`], where the
//!   [`weapon::FireMode`] selector is a list of [`weapon::FireModeSpec`]s each
//!   carrying its closed [`weapon::ModeKind`]) living as sibling
//!   components on the
//!   armed entity, spawned via the [`weapon::WeaponBundle`]. The §1/§6 readers take
//!   a transient [`weapon::WeaponStats`] borrow-view (refs assembled from the
//!   components — not a stored component); there is no packed `Weapon` data struct.
//! - The shot pipeline ([`resolve_coarse::resolve_coarse`] composes it): [`weapon`]
//!   stats → [`stability`] → [`cone`] (cone width) → [`central_axis`] (muzzle / aim /
//!   recoil-climb axis) → [`sample_cone`] (the in-cone shot vector) → [`clearance`] +
//!   [`march`] (the 3-axis voxel-DDA travel) → [`hit_location`] (the part roll) →
//!   a [`resolve_coarse::ShotOutcome`].
//! - [`los`] — the GTW-337 level-aware coarse-geometry sight probe
//!   [`los::has_los`]: can an [`los::Observer`] SEE a [`los::Target`]? It WRAPS the one
//!   geometry truth ([`march::march_vector`]) and REUSES the shot pipeline's
//!   z-anchoring — a **facing-neutral** eye (`cell_center` + per-stance
//!   [`tuning::MuzzleHeights`] z, no per-facing forward offset) and the EXACT
//!   `cover.peek().or_else(occupant_band)` aim band of
//!   [`fire`]'s `TargetGeometry::compose` fed into [`central_axis::target_aim_point`].
//!   It marches the eye→aim ray ONCE and reports a [`los::Sighted`] verdict (BLOCKED iff
//!   the march stops on a slab / cover / non-target ganger strictly before the target;
//!   corpses do not block). Asymmetric sight falls out of eye-vs-aim anchoring. Pure,
//!   render-free, RNG-free; leaf 1 of the GTW-13 FOV epic. [`los::can_see`] (leaf 3,
//!   GTW-339) is the single-observer engagement gate composing the conscious-observer
//!   gate ([`ganger::LifeState::is_active`]), the 2D Chebyshev range disc bound by
//!   [`tuning::ViewRange`], and [`los::has_los`] — faction-agnostic, consulted per
//!   observer/target pair by the AI engagement gate and reaction fire.
//! - [`visibility`] — the GTW-340 squad fog-of-war three-state model (leaf 4 of the
//!   GTW-13 FOV epic): the [`visibility::SquadVisibility`] resource (the VISIBLE /
//!   EXPLORED [`metric::CellLevel`] sets, UNSEEN the implicit complement; EXPLORED is
//!   monotone), the PURE read seams the GTW-11 fog gate / GTW-70 AI / GTW-38 reaction
//!   fire consume ([`visibility::SquadVisibility::is_cell_visible`] /
//!   [`visibility::SquadVisibility::is_cell_explored`] /
//!   [`visibility::SquadVisibility::visible_cells`] and
//!   [`visibility::is_ganger_visible`] over an explicit [`visibility::FactionRelation`]),
//!   and the pure value transforms GTW-341's recompute system calls
//!   ([`visibility::union_fov`] — the squad VISIBLE union over a disc-bounded
//!   authored/occupied candidate set, each candidate banded the shot-pipeline way —
//!   and [`visibility::accrue`], VISIBLE-replaces / EXPLORED-grows). Pure data +
//!   helpers; the recompute system wiring is GTW-341, not here.
//! - [`pathfinder`] — the GTW-12d multi-level route core (ADR-0005, visibility.md
//!   §48): the deterministic weighted search over `(cell, level)` nodes that serves
//!   BOTH of GTW-12's deliverables from ONE relaxation core. [`pathfinder::find_path`]
//!   is point-to-point A\* (Dijkstra + the admissible `chebyshev_xy × 4` heuristic)
//!   returning a typed [`pathfinder::Path`] (the ordered `start..=goal` cells + the
//!   total [`ganger::Tu`], the total == the summed per-step edge costs — the §48
//!   bit-identity) or a typed [`pathfinder::PathBlocked`]; [`pathfinder::reachable_within`]
//!   is the bounded Dijkstra distance-field FLOOD (`h ≡ 0`) yielding every
//!   `(cell, level)` within a [`ganger::Tu`] budget. Edges are the UNION of GTW-350
//!   planar [`occupancy::pathable_neighbors`] and GTW-351 cross-storey
//!   [`vertical::traversable_links`]; the frontier's `(cost, (z, y, x) cell_key)`
//!   tie-break + the pre-sorted edge enumeration make replays byte-equal (no RNG).
//!   PURE free functions over the borrowed grids/tuning snapshot — no `&mut World`, no
//!   per-query rebuild; it plans + totals, never charges TU.
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
//! - [`fire`] — the E4.5 capstone firing act ([`fire::fire`]): a query-based Bevy
//!   function (NO `&mut World`) over two disjoint queries ([`fire::ShooterQuery`] and
//!   [`fire::TargetQuery`]) that validates ([`magazine::can_fire`]) → charges TU once
//!   ([`tu::spend_tu`]) → clamps the burst to ammo ([`magazine::clamp_burst`]) →
//!   per-round composes the cone ([`aim::cone_for`]), [`resolve_coarse::resolve_coarse`],
//!   and [`resolve_and_apply::resolve_and_apply`] into a frozen `Vec<HitReport>`
//!   volley. Every draw via the injected [`rng::SimRng`]; no LOS/fog; no pixel.
//! - [`acts`] — the E10.2 message-driven INPUT CONTRACT + per-act dispatch + the public
//!   [`acts::SimActsPlugin`]: six `#[derive(Message)]` `*Requested` types
//!   ([`acts::FireRequested`] / [`acts::SetAimingRequested`] / [`acts::SetStanceRequested`]
//!   / [`acts::SetFacingRequested`] / [`acts::StabilizeDownedRequested`] /
//!   [`acts::ExecuteDownedRequested`]) carrying [`Entity`](bevy::prelude::Entity) actor
//!   ref(s) + owned payload, and one dispatch system per act that drains the buffered
//!   message and calls the ALREADY-LANDED verb ([`fire`] / [`posture`] / [`downed_acts`])
//!   once per message — every dispatch system `.in_set(occupancy_sync::SimSystems::Simulate)`
//!   in `Update`. No act logic is reimplemented; no `*Resolved`, no movement act (later).
//! - [`battle`] — the E10.5 sim-owned battle-lifecycle integration: the public
//!   [`battle::BattleSimPlugin`] (adding it wires the WHOLE sim runtime — it bundles
//!   [`occupancy_sync::OccupancyMaintenancePlugin`] + [`acts::SimActsPlugin`]) and the
//!   three `#[derive(Message)]` lifecycle types the app drives it with
//!   ([`battle::SetupBattleRequested`] carrying an owned [`situation::Situation`] +
//!   [`rng::BattleSeed`], [`battle::TeardownBattleRequested`], and the
//!   [`battle::BattleReady`] setup-complete signal). It also owns the
//!   [`battle::BattleInProgress`] marker resource (GTW-212): the explicit "a battle is
//!   active" witness the bundled `Simulate` band gates on — inserted on a successful
//!   setup, removed on teardown. The app SENDS the triggers (naming its own states,
//!   app-side) and the sim ACTS on them, naming NO `gdtf_app` type — the one-way
//!   model/view boundary expressed as messages. It also owns the GTW-237 roster-grounded
//!   outcome census: the [`battle::BattleRoster`] resource (the factions fielded at setup,
//!   captured/removed alongside [`battle::BattleInProgress`]) and the
//!   [`battle::check_outcome`] `Simulate`-band system, which emits the
//!   [`battle::BattleWon`] / [`battle::BattleLost`] signal messages (a sim SIGNAL only —
//!   the app-side consumer that ends the battle is GTW-239).
//!
//! Design canon: `docs/combat/` (notably `battle-space.md`, `resolution.md`) and
//! ADR-0001 (`docs/decisions/0001-rust-bevy-rewrite.md`) — the model/view split
//! this crate sits inside as the authoritative, render-free model.

pub mod acts;
pub mod aim;
pub mod apply_hit;
pub mod armor;
pub mod armor_wear;
pub mod battle;
pub mod bleed;
pub mod central_axis;
pub mod clearance;
pub mod cone;
pub mod cover;
pub mod downed_acts;
pub mod faced_cell;
pub mod fire;
pub mod firing_arc;
pub mod ganger;
pub mod hit_location;
pub mod inflicted_wound;
pub mod los;
pub mod magazine;
pub mod march;
pub mod matchup;
pub mod metric;
pub mod move_acts;
pub mod occupancy;
pub mod occupancy_sync;
pub mod pathfinder;
pub mod posture;
pub mod resolve_and_apply;
pub mod resolve_coarse;
pub mod resolve_hit;
pub mod rng;
pub mod sample_cone;
pub mod severity;
pub mod shot_fired;
pub mod situation;
pub mod stability;
pub mod surface;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod tu;
pub mod tuning;
pub mod turn;
pub mod vertical;
pub mod visibility;
pub mod weapon;

pub use acts::{
    FireDeclaration, MoveRejected, MoveRejection, MovementOccurred, ReloadOutcome, ReloadResult,
};
pub use aim::{Shooter, cone_for, stability_for};
pub use apply_hit::{GangerHitTarget, apply_hit};
pub use armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
    ArmorRegistry, ArmorSpec, ArmorType, BodyPart, PieceArmorMut, SourceArmor, Wears, WornBy,
};
pub use armor_wear::{ArmorBroken, ArmorWearOutcome, ArmorWorn, wear_armor};
pub use battle::{
    BattleInProgress, BattleLost, BattleReady, BattleRoster, BattleSimPlugin, BattleWon,
    PlayerFaction, SetupBattleRequested, TeardownBattleRequested, check_outcome,
    setup_battle_on_request, teardown_battle_on_request,
};
pub use bleed::{Bleeding, enemy_phase_started, tick_bleed};
pub use central_axis::{AimDir, climb_aim_dir, muzzle_position, target_aim_point};
pub use clearance::{
    Clearance, round_band_for_cell, round_band_fraction, round_clears_occupant, silhouette_band,
};
pub use cone::{ConeAngle, PriorShots, RecoilFactor, aim_cone_mult, cone_angle, recoil_factor};
pub use cover::{
    BandFraction, CoverDamage, CoverEntry, CoverEvent, CoverHp, CoverLedger, Destroyed, HeightBand,
    band_for,
};
pub use downed_acts::{
    Actor, DownedTarget, can_execute, can_stabilize, execute_downed, is_8_adjacent,
    stabilize_downed,
};
pub use faced_cell::faced_cell;
pub use fire::{
    BattleGrids, PieceQuery, ShooterQuery, TargetQuery, Volley, WeaponQuery, WearsQuery,
    WieldsQuery, fire,
};
pub use ganger::{
    Aiming, Direction, Facing, Faction, GangerName, Hp, HpMax, LifeState, Luck, Position, Shooting,
    Stabilized, Stance, StanceKind, Toughness, Tu, TuMax, Wounds, WoundsMax,
};
pub use hit_location::roll_body_part;
pub use inflicted_wound::{InflictedWound, InflictedWounds};
pub use los::{CanSee, Observer, Sighted, Target, can_see, has_los};
pub use magazine::{
    FireActor, LoadedRounds, Magazine, ReloadTu, can_fire, clamp_burst, in_bounds, mode_tu_cost,
};
pub use march::{MarchKind, MarchResult, march_vector};
pub use matchup::{Matchup, MatchupMultiplier, WheelNode, matchup, matchup_multiplier};
pub use metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos, cell_center, pos_to_cell};
pub use move_acts::{ReactionShotFired, WalkInProgress, advance_walk};
pub use occupancy::{
    DestroyedCover, GRID_HEIGHT, GRID_WIDTH, OccupancyGrid, OccupancyInput, OccupancySlot,
    OccupantPlacement, TerrainKind, TerrainPlacement, pathable_neighbors,
};
pub use occupancy_sync::{
    CoverDestroyed, OccupancyMaintenancePlugin, PrevSlot, sync_dead_gangers, sync_destroyed_cover,
    sync_moved_gangers,
};
pub use pathfinder::{Path, PathBlocked, PathCost, PlanningView, find_path, reachable_within};
pub use posture::{set_aiming, set_facing, set_stance};
pub use resolve_and_apply::{AppliedDamage, HitReport, TargetGanger, resolve_and_apply};
pub use resolve_coarse::{ShotInputs, ShotKind, ShotOutcome, resolve_coarse};
pub use resolve_hit::{HitResult, HpDamage, IntegrityWear, PenetratingDamage, resolve_hit};
pub use rng::{BattleSeed, SimRng};
pub use sample_cone::{ConcentrationP, ShotDir, concentration_p, sample_cone_vector};
pub use severity::{PartSeverityMod, Severity, SeverityInputs, part_severity_mod, roll_severity};
pub use shot_fired::ShotFired;
pub use situation::{
    BattleSetup, BattleSetupError, CoverSpawn, GangerSpawn, Situation, has_stacked_gangers,
    setup_battle,
};
pub use stability::{ConeMult, EmplacementStability, RecoilGrowth, StabilityScore, stability};
pub use surface::{GroundDamage, SlabState, SurfaceGrid};
pub use tu::{can_spend_tu, reset_tu, spend_tu};
pub use tuning::{
    AimConeMult, AimHeightFrac, AimMode, AimTuPremium, BandEdge, BleedRate, BodyPartWeight,
    BodyPartWeights, BraceContribution, BraceMinHeight, CombatTuning, ConcentrationCoeff,
    ConcentrationCoeffs, ConeStabilityTuning, DefenderLuckScale, ExecuteTu, MatchupMultipliers,
    MoveCost, MoveCosts, MuzzleForwardOffset, MuzzleHeight, MuzzleHeights, PenDamageScale,
    ProjectileBandEdges, RandomSpread, RecoilClimb, SeverityEdge, SeverityEdges, SeverityScaling,
    ShooterLuckScale, SilhouetteTop, SilhouetteTops, StabilityCurve, StabilityCurveCoord,
    StabilityCurvePoint, StabilityCurves, StabilizeTu, StanceChangeTu, StanceContribution,
    StanceStability, ToughnessMitigation, TurnTu, WoundCost, WoundCosts,
};
pub use turn::{ActiveFaction, TurnStarted, dispatch_end_turn, regen_team_tu};
pub use vertical::{
    InvalidVerticalLink, LinkKind, OneWay, VerticalLink, VerticalLinkGraph,
    build_vertical_link_graph,
};
pub use visibility::{
    FactionRelation, FovObserver, SquadVisibility, accrue, is_ganger_visible, recompute_visibility,
    should_recompute_visibility, union_fov,
};
pub use weapon::{
    Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
    HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
    Stable, Weapon, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch, WeaponRegistry,
    WeaponShred, WeaponSpec, WeaponStats, WieldedBy, Wields,
};
