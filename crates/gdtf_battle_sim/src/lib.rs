//! Authoritative, render-free combat simulation for GDTF's turn-based battle
//! system — **the MODEL**.
//!
//! This crate owns combat truth: deterministic, unit-testable with an injected
//! seeded RNG streams ([`rng::ShotRng`] / [`rng::SeverityRng`] / [`rng::LootRng`] / [`rng::InjuryRng`] / [`rng::ProcgenRng`] / [`rng::ReactionRng`]), and presentation-agnostic — it reasons in the
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
//!   coefficient, serde-loaded from `assets/core_tuning/combat.tuning.ron`.
//! - [`level`] — the GTW-409 map-editor / procgen foundations: the
//!   [`level::GridSize`] dimension newtypes (each axis validated against
//!   [`metric::MAX_LEVELS`] / [`level::MAX_GRID_SPAN`]), the UUID-keyed theme model
//!   ([`level::UuidThemeDef`] keyed in [`level::UuidThemeRegistry`]), and the UUID-keyed
//!   level-fragment model ([`level::PrefabSpecV2`] / [`level::PrefabRegistry2`]) the
//!   app's `Load` flow builds. Render-free.
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
//!   and STOPPING there (GTW-70 removed the auto-pass; the enemy turn is driven by the
//!   [`ai::enemy_ai_turn`] brain, which ends it back to the player).
//! - [`mod@faced_cell`] — the E4.2 geometry helper [`faced_cell::faced_cell`]: shooter
//!   [`ganger::Position`] + [`ganger::Facing`] → the faced ([`metric::Cell`],
//!   [`metric::Level`]) one unit step along the facing (same storey). The cell whose
//!   cover the §1a brace gate (E4.3) reads; composes only the [`metric`] sim-unit
//!   conversions — render-free, no `World`, zero pixels.
//! - [`aim`] — the E4.3 §1a composers [`aim::stability_for`] / [`aim::cone_for`]:
//!   the shared HUD + `fire()` methods that compose the stability read and
//!   dispersion-cone WIDTH off a shooter's ganger state ([`aim::Shooter`] bundle)
//!   and the MODEL [`cover::CoverLedger`] (peeked at the E4.2 faced cell, never
//!   rebuilt). They WRAP the landed E2 [`mod@stability`] / [`cone`] pipeline verbatim;
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
//!   change-driven in-place maintenance), [`vertical`] (the stair/ladder link graph),
//!   [`terrain::piece`] (the shared terrain-piece identity newtypes — [`piece::TerrainName`]
//!   / [`piece::TerrainGraphicKey`] / [`piece::FootfallSound`] — reused by the UUID terrain
//!   model and the presenter).
//! - [`situation`] — the authored [`situation::Situation`] + [`situation::setup_battle`].
//! - [`rng`] — the model-owned seeded RNG streams (GTW-14: five per-subsystem streams).
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
//!   stats → [`mod@stability`] → [`cone`] (cone width) → [`central_axis`] (muzzle / aim /
//!   recoil-climb axis) → [`sample_cone`] (the in-cone shot vector) → [`clearance`] +
//!   [`march`] (the 3-axis voxel-DDA travel) → [`hit_location`] (the part roll) →
//!   a [`resolve_coarse::ShotOutcome`].
//! - [`los`] — the GTW-337 level-aware coarse-geometry sight probe
//!   [`los::has_los`]: can an [`los::Observer`] SEE a [`los::Target`]? It WRAPS the one
//!   geometry truth ([`march::march_vector`]) and REUSES the shot pipeline's
//!   z-anchoring — a **facing-neutral** eye (`cell_center` + per-stance
//!   [`tuning::MuzzleHeights`] z, no per-facing forward offset) and the EXACT
//!   `cover.peek().or_else(occupant_band)` aim band of
//!   [`mod@fire`]'s `TargetGeometry::compose` fed into [`central_axis::target_aim_point`].
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
//! - [`mod@matchup`] — the 7-type weapon×armor Paley-tournament lookup
//!   ([`matchup::matchup`] over [`weapon::DamageType`] / [`armor::ArmorType`]) and
//!   the punch-&-shred [`matchup::MatchupMultiplier`] it yields from tuning.
//! - [`mod@resolve_hit`] — the per-hit damage/penetration formula
//!   ([`resolve_hit::resolve_hit`]) that resolves weapon damage stats vs one
//!   [`armor::ArmorPiece`] under a [`matchup::Matchup`] into a frozen
//!   [`resolve_hit::HitResult`] (penetrating / HP-loss / integrity-wear). Pure
//!   math — application of HP / Wounds / armor wear is a later E3 slice.
//! - [`severity`] — the §6 wound-severity roll ([`severity::roll_severity`]):
//!   the penetration-gated score → [`severity::Severity`] bucket in the
//!   floor-extend form (the defender's Luck extends the roll's floor down). Pure
//!   seeded-RNG math; the per-tier Wounds application is a later E3 slice.
//! - [`mod@resolve_and_apply`] — the E3.9 capstone integrator
//!   ([`resolve_and_apply::resolve_and_apply`]): folds one [`resolve_coarse::ShotOutcome`]
//!   through matchup → [`mod@resolve_hit`] → [`severity`] → [`mod@apply_hit`] into ONE act
//!   and returns a FROZEN [`resolve_and_apply::HitReport`] for the presenter's FX
//!   (corpse-skip before any draw; every draw via the injected [`rng::ShotRng`] or [`rng::SeverityRng`]; no
//!   pixel). Charging TU / looping the burst (`fire()`) is E4.
//! - [`mod@melee`] — the §7 opposed-Fight resolution CORE (GTW-506):
//!   [`melee::opposed_fight`] (two [`rng::FightRng`] draws → a [`melee::FightOutcome`]
//!   of `connect` + [`melee::FightMargin`]), [`melee::melee_damage_mult`] (the
//!   margin → clamped [`melee::MeleeDamageMult`] curve), and [`melee::apply_melee_multiplier`]
//!   (the PURE seam scaling a [`resolve_hit::HitResult`] by the multiplier between
//!   [`mod@resolve_hit`] and the §6 wound step). Pure functions + a dedicated seeded
//!   stream; the live melee ACT is GTW-507.
//! - [`mod@fire`] — the E4.5 capstone firing act ([`fire::fire`]): a query-based Bevy
//!   function (NO `&mut World`) over two disjoint queries ([`fire::ShooterQuery`] and
//!   [`fire::TargetQuery`]) that validates ([`magazine::can_fire`]) → charges TU once
//!   ([`tu::spend_tu`]) → clamps the burst to ammo ([`magazine::clamp_burst`]) →
//!   per-round composes the cone ([`aim::cone_for`]), [`resolve_coarse::resolve_coarse`],
//!   and [`resolve_and_apply::resolve_and_apply`] into a frozen `Vec<HitReport>`
//!   volley. Every draw via the injected [`rng::ShotRng`] or [`rng::SeverityRng`]; no LOS/fog; no pixel.
//! - [`acts`] — the E10.2 message-driven INPUT CONTRACT + per-act dispatch + the public
//!   [`acts::SimActsPlugin`]: six `#[derive(Message)]` `*Requested` types
//!   ([`acts::FireRequested`] / [`acts::SetAimingRequested`] / [`acts::SetStanceRequested`]
//!   / [`acts::SetFacingRequested`] / [`acts::StabilizeDownedRequested`] /
//!   [`acts::ExecuteDownedRequested`]) carrying [`Entity`](bevy::prelude::Entity) actor
//!   ref(s) + owned payload, and one dispatch system per act that drains the buffered
//!   message and calls the ALREADY-LANDED verb ([`mod@fire`] / [`posture`] / [`downed_acts`])
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

// ── Parent concern modules ───────────────────────────────────────────────────
pub mod acts_runtime;
pub mod combatants;
pub mod damage_resolution;
pub mod equipment;
pub mod falls;
pub mod foundation;
pub mod level;
pub mod lifecycle;
pub mod melee;
pub mod perception;
pub mod shot_pipeline;
pub mod terrain;

// ── Root modules (not moved) ─────────────────────────────────────────────────
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod tuning;

// ── Module re-exports: preserve `gdtf_battle_sim::<child>::Item` call paths ──
// External crates use sub-paths like `gdtf_battle_sim::acts::SimActsPlugin`;
// these re-exports keep those paths resolving without any caller edits.
// ── Flat item re-exports: preserve `gdtf_battle_sim::TypeName` paths ─────────
pub use acts_runtime::{
    acts,
    acts::{
        FireArcDecision, FireDeclaration, InjuryInflicted, MeleeRequested, MeleeResolved,
        MeleeTarget, MoveRejected, MoveRejection, MovementOccurred, OpenDoorRequested,
        ReloadOutcome, ReloadResult, ShoveOutcome, ShoveRequested, ShoveSource,
        ThrowGrenadeRequested, ThrowResolved, apply_injury, can_engage, decide_fire_arc,
        dispatch_melee, dispatch_open_door, dispatch_shove, dispatch_throw_grenade, resolve_shove,
    },
    ai,
    ai::{
        ActCadence, ActPacing, AiTarget, EnemyActCooldown, enemy_ai_turn, pick_nearest,
        plan_advance,
    },
    attachments,
    attachments::apply_pending_attachments,
    bleed,
    bleed::{Bleeding, enemy_phase_started, tick_bleed},
    dot,
    dot::{DotApplied, DotTicked, apply_dot, tick_dot},
    downed_acts,
    downed_acts::{
        Actor, DownedTarget, can_execute, can_stabilize, execute_downed, is_8_adjacent,
        stabilize_downed,
    },
    fields,
    fields::{
        FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, FieldRegistry,
        FieldTicked, FieldTurns, ImmuneArmorTypes, PlacedField, tick_fields,
    },
    firing_arc, move_acts,
    move_acts::{ReactionShotFired, WalkInProgress, advance_walk},
    on_death,
    on_death::{
        CoverOnDeathRegistry, ExplodeDamage, OnDeath, OnDeathEffect, OnDeathOccurred,
        resolve_on_death,
    },
    reaction,
    reaction::{reaction_trigger, reset_reactions_used},
    suppression,
    suppression::{
        SuppressionApplied, apply_suppression, reset_suppression, stance_for_cover_band,
        suppression_auto_stance,
    },
    turn,
    turn::{ActiveFaction, TurnStarted, dispatch_end_turn, regen_team_tu},
};
pub use combatants::{
    faced_cell,
    faced_cell::faced_cell,
    ganger,
    ganger::{
        Aim, Aiming, Bottle, Cool, DerivedStats, Direction, Facing, Faction, Fight, GangMember,
        GangName, GangRegistry, GangRoster, GangerAttributes, GangerName, Grit, Hp, HpMax,
        LifeState, Luck, Morale, Position, Reactions, Reflexes, Shooting, Speed, Stabilized,
        Stance, StanceKind, Strength, Suppressed, SuppressorCell, Toughness, Tu, TuMax, Wounds,
        WoundsMax, derive_stats,
    },
    posture,
    posture::{set_aiming, set_facing, set_stance},
    tu,
    tu::{can_spend_tu, reset_tu, spend_tu},
};
pub use damage_resolution::{
    apply_hit,
    apply_hit::{GangerHitTarget, apply_hit},
    hit_location,
    hit_location::roll_body_part,
    inflicted_wound,
    inflicted_wound::{InflictedWound, InflictedWounds},
    injuries,
    injuries::{
        BleedAfflicted, BleedAmount, GainedInjury, HandsAvailable, InflictedInjuries, InjuryDef,
        InjuryEffect, InjuryName, InjuryRegistry, InjuryTables, InjuryWeight, InjuryWeighting,
        InspectText, LogText, MovementCostFactor, PopupText, PostHeal, RolledInjury, StatDelta,
        StatDeltaLedger, StatDeltaSum, StatKind, StatTarget, WeightedInjuryEntry,
        WeightedInjuryTable, roll_injury,
    },
    matchup,
    matchup::{Matchup, MatchupMultiplier, WheelNode, matchup, matchup_multiplier},
    resolve_and_apply,
    resolve_and_apply::{
        AppliedDamage, GroundAccrual, HitReport, StruckSurfaces, TargetGanger, resolve_and_apply,
    },
    resolve_hit,
    resolve_hit::{HitResult, HpDamage, IntegrityWear, PenetratingDamage, resolve_hit},
    severity,
    severity::{PartSeverityMod, Severity, SeverityInputs, part_severity_mod, roll_severity},
};
pub use equipment::{
    armor,
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
        ArmorRegistry, ArmorSpec, ArmorType, BodyPart, InjuryCategory, PieceArmorMut, SourceArmor,
        Wears, WornBy,
    },
    armor_wear,
    armor_wear::{ArmorBroken, ArmorWearOutcome, ArmorWorn, wear_armor},
    magazine,
    magazine::{
        FireActor, LoadedRounds, Magazine, ReloadTu, can_fire, clamp_burst, in_bounds, mode_tu_cost,
    },
    weapon,
    weapon::{
        Accuracy, AimDelta, AoeRange, ApplyAim, ApplyAttachmentEffect, ApplyBrace, ApplyDamage,
        ApplyDamageTypeOverride, ApplyExtraAmmo, ApplyFastReload, ApplyFatalBias,
        ApplyGainFireMode, ApplyPenetration, ApplyShove, ApplyShred, ApplySilence, ApplyStability,
        AttachToWeaponExt, AttachmentEffect, AttachmentName, AttachmentRegistry, AttachmentSpec,
        BaseSpread, BlastRadius, ConeHalfAngle, DamageProfile, DamageType, Dot, DotDamage,
        DotProfile, DotTurns, FISTS_KEY, FatalBias, FightMode, FightModeKind, FightModeSpec,
        FireMode, FireModeSpec, Handedness, HandlingProfile, HitType, Kickback, MagazineSize,
        MeleeDamageProfile, MeleeWeapon, MeleeWeaponBundle, MeleeWeaponRegistry, MeleeWeaponSpec,
        ModeConeMult, ModeKind, ModeShots, ModeTuPercent, MountedWeapon, PendingAttachments, Reach,
        ReloadScale, Shove, Silenced, Stable, Strikes, TrajectoryStyle, TuCost, Weapon,
        WeaponBraceBonus, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch, WeaponRegistry,
        WeaponShred, WeaponSpawnSiblings, WeaponSpec, WeaponStats, WieldedBy, Wields,
        shooter_weapon_silenced,
    },
};
pub use falls::{DropLanding, FallOccurred, FallsPlugin, StoreysFallen, apply_falls, resolve_drop};
pub use foundation::{
    metric,
    metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos, cell_center, pos_to_cell},
    rng,
    rng::{
        BattleSeed, FightRng, InjuryRng, LootRng, ProcgenRng, ReactionRng, SeverityRng, ShotRng,
    },
};
pub use level::{
    GridHeight, GridLevels, GridSize, GridSizeError, GridWidth, MAX_GRID_SPAN, Prefab2, PrefabKey2,
    PrefabName, PrefabRegistry2, PrefabSpecV2, SpawnRole, TerrainPlacementEntry, ThemeDisplayName,
    ThemeUuid, UuidThemeDef, UuidThemeRegistry,
};
pub use lifecycle::{
    battle,
    battle::{
        BattleInProgress, BattleLost, BattleReady, BattleRoster, BattleSimPlugin, BattleWon,
        PlayerFaction, SetupBattleRequested, TeardownBattleRequested, check_outcome,
        setup_battle_on_request, teardown_battle_on_request,
    },
    procgen,
    procgen::{
        Anchor, DeadRectScatterCount, FilledPlacement, Footprint, LargePrefabAreaThreshold, Margin,
        MaxRectsPacker, MinDensityFloor, MinPlayerSide, PackingError, PlacedPrefab,
        Placement as PrefabPlacement, ProcgenTuning, RegionRect, SplitMode, assemble_placement,
        assemble_placement_with, emit_level, fill_placement, fill_placement_with, generate_level,
    },
    situation,
    situation::{
        BattleRegistries, BattleSetup, BattleSetupError, CoverSpawn, FieldSpawn, FloorSpawn,
        GangerSpawn, PlacedGanger, Placement, Situation, SlabSpawn, has_stacked_gangers,
        setup_battle,
    },
};
pub use melee::{
    Combatants, FightMargin, FightOutcome, MeleeDamageMult, MeleeStrike, MeleeWeaponHit,
    StructuralMult, apply_melee_multiplier, melee_damage_mult, opposed_fight, resolve_melee_strike,
    resolve_structural_melee,
};
pub use perception::{
    los,
    los::{CanSee, Observer, PeekOffset, Sighted, Target, can_see, has_los, has_los_peeking},
    pathfinder,
    pathfinder::{
        MIN_MOVE_COST, Path, PathBlocked, PathCost, PlanningView, find_path, reachable_within,
    },
    peek_sync,
    peek_sync::{peek_population_needed, sync_peek_offsets},
    visibility,
    visibility::{
        FactionRelation, FovObserver, OmniscientFog, SquadVisibility, accrue, is_ganger_visible,
        move_fog, recompute_visibility, should_recompute_visibility, union_fov,
    },
};
pub use shot_pipeline::{
    aim,
    aim::{Shooter, cone_for, stability_for},
    aoe,
    aoe::aoe_affected,
    central_axis,
    central_axis::{AimDir, climb_aim_dir, muzzle_position, target_aim_point},
    clearance,
    clearance::{
        Clearance, round_band_for_cell, round_band_fraction, round_clears_occupant, silhouette_band,
    },
    cone,
    cone::{ConeAngle, PriorShots, RecoilFactor, aim_cone_mult, cone_angle, recoil_factor},
    fire,
    fire::{
        BattleGrids, MeleeQuery, MountedQuery, PieceQuery, ShooterQuery, TargetQuery, Volley,
        WeaponQuery, WearsQuery, WieldsQuery, fire, resolve_blast,
    },
    march,
    march::{MarchKind, MarchResult, march_arc, march_vector},
    resolve_coarse,
    resolve_coarse::{ShotInputs, ShotKind, ShotOutcome, resolve_coarse},
    sample_cone,
    sample_cone::{ConcentrationP, ShotDir, concentration_p, sample_cone_vector},
    shot_fired,
    shot_fired::ShotFired,
    stability,
    stability::{
        ConeMult, EmplacementStability, RecoilGrowth, StabilityScore, SuppressionStability,
        TerrainBraced, stability,
    },
};
pub use terrain::{
    cover,
    cover::{
        BandFraction, CoverDamage, CoverEntry, CoverEvent, CoverHp, CoverLedger, Destroyed,
        HeightBand, band_for,
    },
    def,
    def::{
        TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind,
        TerrainTag, TerrainUuid, closed_openable_vision_band, is_openable,
    },
    emplacement,
    emplacement::{
        EmplacementOccupant, EmplacementState, EmplacementTogglePlugin, MountedWeaponEntity,
        MountedWeaponKey, SetEmplacement, apply_emplacement_toggle,
    },
    entity,
    entity::{
        BlocksVision, TerrainBrace, TerrainCell, TerrainIndex, TerrainIndexKey, TerrainPieceKind,
    },
    floor,
    floor::FloorCostGrid,
    occupancy,
    occupancy::{
        DestroyedCover, GRID_HEIGHT, GRID_WIDTH, OccupancyGrid, OccupancyInput, OccupancySlot,
        OccupantPlacement, StairEyeOffset, TerrainKind, TerrainPlacement, VisionBlocking,
        pathable_neighbors, project_vision_blocking,
    },
    occupancy_sync,
    occupancy_sync::{
        CoverDestroyed, GroundAccrued, OccupancyMaintenancePlugin, PrevSlot, SlabDestroyed,
        sync_accrued_ground, sync_dead_gangers, sync_destroyed_cover, sync_destroyed_slab,
        sync_moved_gangers,
    },
    openable,
    openable::{
        OpenState, OpenableBlocking, OpenableTogglePlugin, SetOpenable, apply_openable_toggle,
    },
    piece,
    piece::{FootfallSound, TerrainGraphicKey, TerrainName},
    slab,
    slab::{
        BraceStairCells, SlabDamage, SlabDestroyedFlag, SlabEntry, SlabEvent, SlabHp, SlabLedger,
    },
    surface,
    surface::{GroundDamage, SlabState, SurfaceGrid},
    vertical,
    vertical::{
        InvalidVerticalLink, LinkKind, OneWay, VerticalLink, VerticalLinkGraph,
        build_vertical_link_graph,
    },
};
pub use tuning::{
    AimConeMult, AimHeightFrac, AimMode, AimTuPremium, BandEdge, BleedRate, BodyPartWeight,
    BodyPartWeights, BottlePerMorale, BraceContribution, BraceMinHeight, CombatTuning,
    ConcentrationCoeff, ConcentrationCoeffs, ConeStabilityTuning, DefenderLuckScale,
    EmplacementStabilityBonus, ExecuteTu, FightVariance, FightWeights, GangerStatTuning, HpWeights,
    MatchupMultipliers, MeleeKMargin, MeleeMultMax, MeleeMultMin, MeleeTuning, MoraleWeights,
    MoveCost, MoveCosts, MuzzleForwardOffset, MuzzleHeight, MuzzleHeights, PenDamageScale,
    PerStoreyDamage, ProjectileBandEdges, RandomSpread, ReactionCapBase, ReactionCapPerReactions,
    ReactionPMax, ReactionPMin, ReactionProbability, ReactionScore, ReactionTuning, ReactionsUsed,
    ReactionsWeights, RecoilClimb, SeverityEdge, SeverityEdges, SeverityScaling, ShooterLuckScale,
    ShootingWeights, ShoveTu, SilhouetteTop, SilhouetteTops, SlabDefaultHp, SlabDefaults,
    StabilityCurve, StabilityCurveCoord, StabilityCurvePoint, StabilityCurves, StabilizeTu,
    StanceChangeTu, StanceContribution, StanceStability, StatWeight, SuppressionRadius,
    SuppressionStabilityPenalty, ToughnessMitigation, TuBase, TuPerSpeed, TurnTu, WoundCost,
    WoundCosts, WoundsPerHp, clamp_probability, interrupt_probability, may_interrupt, reaction_cap,
    reaction_score, rolls_interrupt,
};
