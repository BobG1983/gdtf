//! [`setup_battle`] — pour a [`Situation`](crate::situation::Situation) into the
//! battle ECS — plus its [`BattleSetup`] result and the
//! [`has_stacked_gangers`] sanity helper.

use bevy::{
    platform::collections::HashSet,
    prelude::Commands,
    scene::{
        CommandsSceneExt, EntityCommandsSceneExt, Scene, SceneList, bsn, bsn_list, template_value,
    },
};

use super::terrain_resolve::{
    ResolvedCoverPiece, ResolvedSlabPiece, resolve_cover_spec, resolve_floor_costs,
    resolve_slab_spec, resolve_terrain_or_err,
};
use crate::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorRegistry, ArmorSpec,
        BodyPart, Wears,
    },
    clearance::silhouette_band,
    cover::{CoverEntry, CoverLedger},
    ganger::{
        Aim, Aiming, Bottle, Cool, Facing, Faction, Fight, GangMember, GangRegistry, GangerName,
        Grit, Hp, HpMax, Luck, Morale, Position, Reactions, Reflexes, Shooting, Speed, Stance,
        Strength, Toughness, Tu, TuMax, Wounds, WoundsMax, derive_stats,
    },
    inflicted_wound::InflictedWounds,
    injuries::{BleedAfflicted, InflictedInjuries},
    los::PeekOffset,
    occupancy::{OccupancyGrid, OccupancyInput, OccupantPlacement, TerrainKind, TerrainPlacement},
    situation::{BattleSetupError, PlacedGanger, Situation},
    slab::{BraceStairCells, SlabEntry, SlabLedger},
    surface::{SlabState, SurfaceGrid},
    terrain::{
        entity::{TerrainBrace, TerrainCell, TerrainIndex, TerrainIndexKey, TerrainPieceKind},
        floor::FloorCostGrid,
        piece::TerrainRegistry,
    },
    tuning::{GangerStatTuning, MoveCost},
    vertical::{LinkKind, build_vertical_link_graph},
    weapon::{
        Accuracy, BaseSpread, FatalBias, FireMode, Kickback, Stable, Weapon, WeaponBundle,
        WeaponDamage, WeaponName, WeaponPunch, WeaponRegistry, WeaponShred, Wields,
    },
};

/// The result of [`setup_battle`] — the spawned ganger placements, so the caller
/// can map each authored ganger to its newly-spawned Bevy [`Entity`](bevy::prelude::Entity) handle.
///
/// A named newtype over the placement list (no-bare-types: the setup outcome is a
/// domain value, not a bare `Vec`). Each [`OccupantPlacement`] pairs a
/// `(cell, level)` with the SPAWNED [`Entity`](bevy::prelude::Entity) handle — **never a numeric id**
/// (GTW-10 / GTW-12). The placements are in authored-ganger order. The seeded
/// [`CoverLedger`] / [`SurfaceGrid`] / [`OccupancyGrid`] / [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph)
/// are inserted as resources, queried off the world rather than returned here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BattleSetup {
    /// The spawned `(cell, level) → Entity` occupant placements, in authored order.
    pub occupants: Vec<OccupantPlacement>,
}

impl BattleSetup {
    /// The number of gangers spawned by the setup.
    #[must_use]
    pub const fn ganger_count(&self) -> usize {
        self.occupants.len()
    }
}

/// The read-only content registries [`setup_battle`] resolves a
/// [`Situation`](crate::situation::Situation)'s authored references against — grouped into
/// one borrow-bundle so the setup's parameter list stays under clippy's argument-count gate
/// (the GTW-414 gang registry was the 8th argument; bundling the resolution sources keeps
/// the signature small).
///
/// A named borrow-bundle (no-bare-types: the setup's resolution sources are a domain
/// grouping, not a bare tuple of refs), the `gdtf_app` `LoadAssetCollections`-style
/// transparent-bundle precedent — it wraps existing world-state by reference, never a
/// domain scalar. Each field is the registry / tuning a setup phase looks an authored key
/// up in:
///
/// - `gangs` — resolve each [`PlacedGanger`]'s `(gang, member)` ref to a
///   [`GangMember`](crate::ganger::GangMember) (GTW-414).
/// - `weapons` / `armor` — resolve the resolved member's weapon / armor keys.
/// - `stat_tuning` — derive each ganger's computed stats from its eight attributes.
/// - `terrain` — resolve cover / slab / floor piece keys (`None` ⇒ the floor falls back to
///   `fallback_floor_cost`; cover / slab keys then fail with `TerrainNotFound`).
#[derive(Clone, Copy)]
pub struct BattleRegistries<'a> {
    /// The gang rosters each placed ganger's `(gang, member)` ref resolves against (GTW-414).
    pub gangs:       &'a GangRegistry,
    /// The weapon registry each resolved roster member's weapon key resolves against.
    pub weapons:     &'a WeaponRegistry,
    /// The armor registry each resolved roster member's armor key resolves against.
    pub armor:       &'a ArmorRegistry,
    /// The stat tuning each ganger's computed stats are derived with (GTW-384).
    pub stat_tuning: &'a GangerStatTuning,
    /// The terrain registry cover / slab / floor piece keys resolve against (GTW-396);
    /// `None` skips floor resolution (the [`fallback_floor_cost`](setup_battle) is used).
    pub terrain:     Option<&'a TerrainRegistry>,
}

impl<'a> BattleRegistries<'a> {
    /// Build the resolution borrow-bundle from its five registry / tuning refs — the shape
    /// every [`setup_battle`] caller assembles.
    #[must_use]
    pub const fn new(
        gangs: &'a GangRegistry,
        weapons: &'a WeaponRegistry,
        armor: &'a ArmorRegistry,
        stat_tuning: &'a GangerStatTuning,
        terrain: Option<&'a TerrainRegistry>,
    ) -> Self {
        Self {
            gangs,
            weapons,
            armor,
            stat_tuning,
            terrain,
        }
    }
}

/// Compose ONE ganger as a Bevy `bsn!` [`Scene`] — the per-field component tree
/// [`setup_battle`] spawns for each [`GangerSpawn`](crate::situation::GangerSpawn)
/// (GTW-322).
///
/// The ganger carries its OWN state only — **no equipment stat data**. Since GTW-323
/// slice 3 (ADR-0004) the weapon and armor stats live exclusively on the related
/// weapon ([`Wields`]) and armor-piece ([`Wears`]) entities spawned alongside the
/// ganger (see [`wielded_weapon_scenes`] / [`worn_piece_scenes`]); the ganger holds
/// the relationship, never the components. So this scene composes the per-field ganger
/// state ([`Position`] / [`GangerName`] / [`Faction`] / [`Facing`] / [`Stance`] /
/// [`Aiming`] / [`LifeState`](crate::ganger::LifeState)), the EIGHT authored DIRECT
/// ATTRIBUTES ([`Speed`] / [`Aim`] / [`Strength`] / [`Toughness`] / [`Reflexes`] /
/// [`Cool`] / [`Grit`] / [`Luck`] — the raw potential), the DERIVED computed stats
/// ([`Shooting`] / [`Tu`] / [`TuMax`] / [`Hp`] / [`HpMax`] / [`Wounds`] / [`WoundsMax`]
/// plus the dormant [`Fight`] / [`Reactions`] / [`Morale`] / [`Bottle`]) computed at
/// setup from the attributes × the passed [`GangerStatTuning`] (GTW-384 — fully derived,
/// never authored: the situation authors attributes, the pools/skills are derived
/// here, full at battle start so the current pool == max), and the GTW-279 empty
/// [`InflictedWounds`] record — and nothing else.
///
/// **The `bsn!` recipe (GTW-322 spike).** Every newtype with a `Type::new(value)`
/// constructor is inlined in `bsn!`. The runtime-valued fieldless enum
/// [`LifeState`](crate::ganger::LifeState) (no `new(value)` whole-value ctor) has NO
/// `bsn!` grammar form — inlining a variant would NARROW the authored value — so it is
/// bridged via [`template_value`](bevy::scene::template_value) and tuple-composed onto
/// the SAME root entity. The composed type carries a GTW-322 spawn-seed-sentinel
/// [`Default`].
///
/// **Deferred materialization.** `bsn!`-scene components materialize on the
/// `SpawnScene` schedule (~one `app.update()` later), NOT synchronously. The caller
/// keys occupancy off the [`GangerSpawn`]'s authored
/// [`at`](crate::situation::GangerSpawn::at) value and the synchronously-reserved
/// `Entity` id (`spawn_scene(..).id()`), never off the deferred [`Position`]
/// component.
fn ganger_scene(
    placed: &PlacedGanger,
    member: &GangMember,
    tuning: &GangerStatTuning,
) -> impl Scene {
    // The `bsn!` `Type::new(expr)` form stores a DEFERRED constructor, so every value
    // it captures must be OWNED/`'static` — a borrow (`&PlacedGanger` / `&GangMember`)
    // captured into the macro would make the returned scene outlive the reference (the
    // GTW-322 spike's `'static` finding). So bind every inline value to an owned local
    // FIRST, and let the macro capture those owned locals (never the `&` param).
    //
    // GTW-414 schema v2: the placement fields (`at` / `faction` / `facing` / `stance` /
    // `aiming` / `life_state`) come from the situation-side `PlacedGanger`; the identity
    // (`name`) and the eight direct attributes come from the gang-roster `GangMember`.
    let at = placed.at;
    let name = (*member.name).clone();
    let faction = *placed.faction;
    let facing = *placed.facing;
    let stance = *placed.stance;
    let aiming = *placed.aiming;
    // GTW-384: DERIVE every computed stat from the eight roster attributes × the
    // tuning (the single source of truth — `derive_stats`). The gang roster carries the
    // attributes only; the pools/skills are derived here, full at battle start
    // (current pool == max).
    let attributes = member.attributes();
    let derived = derive_stats(&attributes, tuning);
    // The eight roster attribute magnitudes (the raw potential, carried on the ganger).
    let speed = *member.speed;
    let aim = *member.aim;
    let strength = *member.strength;
    let toughness = *member.toughness;
    let reflexes = *member.reflexes;
    let cool = *member.cool;
    let grit = *member.grit;
    let luck = *member.luck;
    // The derived computed-stat magnitudes (Deref'd out of the DerivedStats record).
    let shooting = *derived.shooting;
    let fight = *derived.fight;
    let reactions = *derived.reactions;
    let morale = *derived.morale;
    let tu = *derived.tu;
    let tu_max = *derived.tu_max;
    let hp = *derived.hp;
    let hp_max = *derived.hp_max;
    let wounds = *derived.wounds;
    let wounds_max = *derived.wounds_max;
    let bottle = *derived.bottle;
    // The runtime-valued leaf with no `bsn!` grammar form, owned for the
    // `template_value` tuple-composition tail (see the doc-comment recipe).
    let life_state = placed.life_state;
    (
        bsn! {
            Position::new(at)
            GangerName::new(name)
            Faction::new(faction)
            Facing::new(facing)
            Stance::new(stance)
            Aiming::new(aiming)
            // The eight authored DIRECT ATTRIBUTES (the raw potential).
            Speed::new(speed)
            Aim::new(aim)
            Strength::new(strength)
            Toughness::new(toughness)
            Reflexes::new(reflexes)
            Cool::new(cool)
            Grit::new(grit)
            Luck::new(luck)
            // The DERIVED computed stats (attributes × GangerStatTuning, full at start).
            Shooting::new(shooting)
            Fight::new(fight)
            Reactions::new(reactions)
            Morale::new(morale)
            Tu::new(tu)
            TuMax::new(tu_max)
            Hp::new(hp)
            HpMax::new(hp_max)
            Wounds::new(wounds)
            WoundsMax::new(wounds_max)
            Bottle::new(bottle)
            InflictedWounds::default()
            // GTW-438: seed the EMPTY injury ledger so every ganger carries
            // InflictedInjuries from frame 0 — the `apply_injury` boundary needs it
            // present to `gain` into, and the GTW-436 projector / inspect read it. It
            // derives Default + Clone + Component (the bsn! sentinel-Default requirement,
            // bsn-sentinel-defaults convention), starting at the zero-delta empty ledger.
            InflictedInjuries::default()
            // GTW-438: seed the EMPTY injury-bleed accrual so the bleed runtime's
            // `Option<&BleedAfflicted>` query matches every ganger; `apply_injury` keeps it
            // in sync with the ledger's accrued bleed. Starts at the no-bleed `0`.
            BleedAfflicted::default()
            // GTW-406: seed the wall-peek offset so the automatic positional populator's
            // `&mut PeekOffset` query matches every ganger from frame 0 (a correctness
            // prerequisite — the populator skips a ganger that lacks the component). It
            // derives Default + Clone + Component (the bsn! sentinel-Default requirement),
            // and starts at the centred, no-peek `Vec2::ZERO`.
            PeekOffset::default()
        },
        // The runtime-valued component with no `bsn!` grammar form, bridged via
        // `template_value` and tuple-composed onto the SAME root entity (the GTW-322
        // spike's canonical runtime-value path).
        template_value(life_state),
    )
}

/// Compose the six worn-armor-piece **related scenes** for a ganger as a
/// [`SceneList`] — one piece entity per [`BodyPart`], each carrying its stat
/// components, to spawn-and-relate via `Wears` (GTW-323 slice 1, ADR-0004).
///
/// The [`setup_battle`] spawn loop hands this list to
/// [`queue_spawn_related_scenes::<Wears>`](bevy::scene::EntityCommandsSceneExt::queue_spawn_related_scenes)
/// on the freshly-spawned ganger entity: the framework spawns one entity per scene,
/// applies the scene's components, and inserts [`WornBy`](crate::armor::WornBy)`(ganger)` on each — whose
/// back-reference hook populates the ganger's [`Wears`] collection automatically.
///
/// Each piece scene tags the entity with its [`BodyPart`] (so the `struck_piece`
/// lookup keys `ganger → Wears → the BodyPart-tagged piece`) and carries the five
/// per-piece stat components ([`ArmorFloor`] / [`ArmorProtection`] / [`ArmorIntegrity`]
/// / [`ArmorHardness`] / `ArmorType`) read **by value** from the resolved
/// [`ArmorSpec`] in [`BodyPart::ALL`] order. These piece entities are the ONLY armor
/// storage — GTW-323 slice 3 removed the transient ganger-side copy, so no armor stat
/// data is stored on the ganger. The four stat newtypes inline via their
/// `Type::new(value)` `bsn!` form; the
/// runtime-valued [`BodyPart`] tag and `ArmorType` (fieldless enums with no `new`
/// grammar form) bridge via [`template_value`] (their GTW-322 spawn-seed-sentinel
/// [`Default`]s seed the slot before the authored value overwrites it), tuple-composed
/// onto the same piece entity.
fn worn_piece_scenes(spec: &ArmorSpec) -> impl SceneList {
    // bsn! `Type::new(expr)` stores a DEFERRED constructor, so every captured value
    // must be OWNED (the GTW-322 `'static` finding). Read each piece by value out of
    // the spec FIRST (ArmorPiece is Copy), then let the macro capture the owned locals.
    let pieces = spec.pieces();
    bsn_list! {
        worn_piece_scene(BodyPart::Head, pieces[BodyPart::Head.index()]),
        worn_piece_scene(BodyPart::Torso, pieces[BodyPart::Torso.index()]),
        worn_piece_scene(BodyPart::LeftArm, pieces[BodyPart::LeftArm.index()]),
        worn_piece_scene(BodyPart::RightArm, pieces[BodyPart::RightArm.index()]),
        worn_piece_scene(BodyPart::LeftLeg, pieces[BodyPart::LeftLeg.index()]),
        worn_piece_scene(BodyPart::RightLeg, pieces[BodyPart::RightLeg.index()]),
    }
}

/// Compose ONE worn-armor-piece entity as a `bsn!` [`Scene`] — its [`BodyPart`] tag
/// plus the five stat components, read by value from `piece` (GTW-323 slice 1).
///
/// The four stat newtypes inline via `Type::new(value)`; the runtime-valued
/// [`BodyPart`] tag and `ArmorType` fieldless enums bridge via [`template_value`]
/// (no `bsn!` grammar form), tuple-composed onto the same piece entity (the GTW-322
/// runtime-value path). The [`WornBy`](crate::armor::WornBy) back-reference is inserted by the framework's
/// `queue_spawn_related_scenes::<Wears>` wiring, NOT here, so it is absent from this
/// scene.
fn worn_piece_scene(part: BodyPart, piece: crate::armor::ArmorPiece) -> impl Scene {
    // Bind every inline value to an owned local FIRST (the bsn! `'static` finding).
    let floor = *piece.floor;
    let protection = *piece.protection;
    let integrity = *piece.integrity;
    let hardness = *piece.hardness;
    let armor_type = piece.armor_type;
    (
        bsn! {
            ArmorFloor::new(floor)
            ArmorProtection::new(protection)
            ArmorIntegrity::new(integrity)
            ArmorHardness::new(hardness)
        },
        // The runtime-valued fieldless enums with no `bsn!` grammar form, bridged via
        // `template_value` and tuple-composed onto the SAME piece entity (GTW-322).
        template_value(part),
        template_value(armor_type),
    )
}

/// Compose the wielded-weapon **related scene list** for a ganger as a [`SceneList`] —
/// the single weapon entity carrying the full GTW-200 decomposed weapon-stat component
/// set, read by value from the resolved [`WeaponBundle`], to spawn-and-relate via
/// [`Wields`] (GTW-323 slice 2, ADR-0004).
///
/// The [`setup_battle`] spawn loop hands this list to
/// [`queue_spawn_related_scenes::<Wields>`](bevy::scene::EntityCommandsSceneExt::queue_spawn_related_scenes)
/// on the freshly-spawned ganger entity: the framework spawns the weapon entity, applies
/// the scene's components, and inserts [`WieldedBy`](crate::weapon::WieldedBy)`(ganger)`
/// on it — whose back-reference hook populates the ganger's [`Wields`] collection
/// automatically. So `fire()` reads the weapon stats off the **weapon entity** through
/// `ganger → Wields → the weapon entity`, mirroring the slice-1 armor traversal. The
/// list holds ONE entry this slice (a ganger wields a single weapon); the
/// [`queue_spawn_related_scenes`](bevy::scene::EntityCommandsSceneExt::queue_spawn_related_scenes)
/// surface takes a [`SceneList`], so the single weapon scene is wrapped in a one-element
/// `bsn_list!` (the same shape the multi-weapon loadout the ADR anticipates would take).
fn wielded_weapon_scenes(weapon: &WeaponBundle) -> impl SceneList {
    bsn_list! { wielded_weapon_scene(weapon) }
}

/// Compose ONE wielded-weapon entity as a `bsn!` [`Scene`] — the [`Weapon`] marker plus
/// the full GTW-200 decomposed weapon-stat component set, read by value from the
/// resolved [`WeaponBundle`] (GTW-323 slice 2, ADR-0004).
///
/// This weapon entity is the ONLY weapon storage — GTW-323 slice 3 removed the
/// transient on-ganger weapon copy, so no weapon stat data is stored on the ganger.
/// The [`Weapon`] marker, every weapon-number newtype, the [`WeaponName`], the
/// [`FireMode`], and the [`Stable`] tag inline via their `Type::new(value)` `bsn!`
/// form; the runtime-valued [`DamageType`](crate::weapon::DamageType) and the
/// value-typed [`Magazine`](crate::magazine::Magazine) — neither has a `bsn!` grammar
/// form — bridge via [`template_value`] and tuple-compose onto the same weapon entity.
/// The [`WieldedBy`](crate::weapon::WieldedBy) back-reference is inserted by the
/// framework's `queue_spawn_related_scenes::<Wields>` wiring, NOT here, so it is absent
/// from this scene.
fn wielded_weapon_scene(weapon: &WeaponBundle) -> impl Scene {
    // bsn! `Type::new(expr)` stores a DEFERRED constructor, so every captured value must
    // be OWNED (the GTW-322 `'static` finding). Read each stat by value out of the
    // bundle FIRST, then let the macro capture the owned locals (never the `&` param).
    let weapon_name = (*weapon.name).clone();
    let base_spread = *weapon.base_spread;
    let accuracy = *weapon.accuracy;
    let kickback = *weapon.kickback;
    let fatal_bias = *weapon.fatal_bias;
    let weapon_damage = *weapon.damage;
    let weapon_punch = *weapon.punch;
    let weapon_shred = *weapon.shred;
    let fire_mode = (*weapon.fire_mode).clone();
    let stable = *weapon.stable;
    // The runtime-valued / value-typed leaves with no `bsn!` grammar form, owned for the
    // `template_value` tuple-composition tail (the GTW-322 runtime-value path). The GTW-443
    // `Handedness` is a runtime-valued enum (like `DamageType`), so it bridges the same way
    // — WITHOUT it the GTW-443 `WeaponQuery`'s `&Handedness` column would not match the
    // spawned weapon entity and `fire()` would fail closed (an empty volley).
    let damage_type = weapon.damage_type;
    let magazine = weapon.magazine;
    let handedness = weapon.handedness;
    (
        bsn! {
            Weapon
            WeaponName::new(weapon_name)
            BaseSpread::new(base_spread)
            Accuracy::new(accuracy)
            Kickback::new(kickback)
            FatalBias::new(fatal_bias)
            WeaponDamage::new(weapon_damage)
            WeaponPunch::new(weapon_punch)
            WeaponShred::new(weapon_shred)
            FireMode::new(fire_mode)
            Stable::new(stable)
        },
        // The runtime-valued / value-typed components with no `bsn!` grammar form,
        // bridged via `template_value` and tuple-composed onto the SAME weapon entity.
        template_value(damage_type),
        template_value(magazine),
        template_value(handedness),
    )
}

/// Build the battle in the ECS world from a [`Situation`](crate::situation::Situation)
/// — the E1.8 setup: the setup system that builds the scene from the situation (see the
/// [`crate::situation`] module doc, the setup-on-entry source of truth).
///
/// GTW-414 schema v2: the function accepts `gangs: &GangRegistry` and resolves every
/// [`PlacedGanger`]'s `(gang, member)` ref against it BEFORE any entity is spawned
/// (abort-first invariant) — the resolved [`GangMember`] supplies the ganger's identity,
/// eight attributes, and weapon / armor keys (the old `GangerSpawn` fields, now sourced
/// from the reusable gang roster). A missing gang / member returns
/// [`BattleSetupError::GangNotFound`] / [`BattleSetupError::GangMemberNotFound`] (no panic).
///
/// GTW-396: the function also accepts `terrain: Option<&TerrainRegistry>` and resolves
/// every authored terrain piece key (cover / slab / floor) against it BEFORE any entity
/// is spawned (abort-first invariant). If `terrain` is `None` or a key resolves
/// to nothing, the function returns [`BattleSetupError::TerrainNotFound`] (no panic, no
/// partial world). If a floor piece's move cost is below
/// [`MIN_MOVE_COST`](crate::pathfinder::MIN_MOVE_COST), it returns
/// [`BattleSetupError::FloorCostBelowMinimum`]. The resolved specs feed:
/// - cover pieces → `CoverEntry` + `TerrainGraphicKey` / `FootfallSound` on the entity
/// - slab pieces → `SlabEntry` (eagerly inserted into `SlabLedger`) + the same hooks
/// - floor pieces → `FloorCostGrid` (default + sparse overrides)
///
/// When `terrain` is `None` OR the situation's `default_floor` is empty (the
/// `#[serde(default)]` sentinel), the floor cost grid falls back to the caller-supplied
/// `fallback_floor_cost` (the `CombatTuning::move_costs.open` value), preserving
/// pre-GTW-396 behavior for test fixtures and situations that haven't migrated.
///
/// Steps, in order:
///
/// 1. **Pre-validate** all terrain piece keys (cover + slab + floor) against the
///    registry — abort-first (return early with `Err` before any spawn).
/// 2. **Spawn each ganger + relate its equipment** — for every
///    [`GangerSpawn`](crate::situation::GangerSpawn),
///    `commands.spawn_scene(`[`ganger_scene`]`(..))` the ganger's OWN per-field state
///    as a Bevy `bsn!` [`Scene`] (GTW-322).
/// 3. **Seed the [`CoverLedger`]** — insert a [`CoverEntry`](crate::cover::CoverEntry)
///    for every wall and scatter piece (the one unified ledger). Spawn ONE terrain entity
///    per cover piece carrying `TerrainCell`, `TerrainPieceKind`, max `CoverHp`,
///    `HeightBand`, `ArmorProtection`, `ArmorHardness`, `TerrainGraphicKey`, `FootfallSound`
///    (GTW-395/396 presentation seam; audio is stubbed — `FootfallSound` is attached
///    and doc-commented as unconsumed until a future footfall-audio ticket).
/// 4. **Seed the [`SurfaceGrid`] + [`SlabLedger`]** — mark every authored slab
///    `Present` and **eagerly insert** its [`SlabEntry`] (HP/armor from the resolved
///    terrain spec). The ledger's `entry_seeded` path returns the eagerly-inserted entry
///    unchanged (the `or_insert` wins only for absent keys), so the authored per-slab HP
///    is honored on first strike without a new `deplete_slab` signature (verified per
///    `ledger.rs:68-74`). Spawn ONE terrain entity per slab carrying `TerrainCell`,
///    `TerrainPieceKind::Slab`, `SlabHp`, `ArmorProtection`, `ArmorHardness`,
///    `TerrainGraphicKey`, `FootfallSound`.
/// 5. **Build the [`OccupancyGrid`]** + insert [`TerrainIndex`].
/// 6. **Build the [`FloorCostGrid`]** from resolved floor specs and insert it.
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
/// - [`BattleSetupError::ArmorNotFound`] — the resolved roster member's armor key absent.
/// - [`BattleSetupError::TerrainNotFound`] — cover/slab/floor piece key absent.
/// - [`BattleSetupError::FloorCostBelowMinimum`] — floor `move_cost < MIN_MOVE_COST`.
///
/// All are validated BEFORE any entity is spawned (abort-first invariant).
#[expect(
    clippy::too_many_lines,
    reason = "setup_battle executes 7 sequential, order-dependent phases \
              (pre-resolve → cover spawn → slab spawn → occupancy → floor grid → \
              link graph) that cannot be broken into smaller fns without threading \
              partial-state through many more parameters; the 5 private helpers already \
              extract every non-trivial sub-computation"
)]
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
        armor,
        stat_tuning,
        terrain,
    } = registries;
    // Validate the vertical links FIRST, so a bad authored link aborts the whole
    // setup before any entity is spawned or any resource inserted (no partial,
    // unspawnable world left behind on a validation failure).
    let vertical_graph = build_vertical_link_graph(situation)?;

    // GTW-414 schema v2: resolve every PlacedGanger's (gang, member) ref against the
    // GangRegistry up front — BEFORE any spawn — so a missing gang/member aborts setup
    // with the typed error (no panic) with no partial world spawned (the abort-first
    // invariant). The resolved roster member supplies the identity + eight attributes +
    // weapon/armor keys the rest of the setup reads (the old GangerSpawn fields, now
    // sourced from the gang roster). Each (placed, member) borrow is held in
    // PlacedGanger order, parallel to situation.gangers.
    let mut resolved_members: Vec<(&PlacedGanger, &GangMember)> =
        Vec::with_capacity(situation.gangers.len());
    for placed in &situation.gangers {
        let Some(roster) = gangs.roster(&placed.gang) else {
            return Err(BattleSetupError::GangNotFound {
                gang: placed.gang.clone(),
            });
        };
        let Some(member) = roster.member(&placed.member) else {
            return Err(BattleSetupError::GangMemberNotFound {
                gang:   placed.gang.clone(),
                member: placed.member.clone(),
            });
        };
        resolved_members.push((placed, member));
    }

    // GTW-457: reject a situation that authors two gangers on the SAME (cell, level)
    // — BEFORE any ganger entity is spawned (abort-first, mirroring the gang/member
    // ref checks above). The GTW-156 occupancy pour is last-write-wins, so a duplicate
    // would silently overwrite the first ganger's occupancy slot while BOTH entities
    // survive stacked on one cell (`docs/combat/resolution.md`: one object per cell).
    // A trusted authored situation with stacked spawns is a DATA bug to fail LOUDLY on,
    // not auto-relocate (auto-relocation belongs in the GTW-424 procgen assembler).
    if let Some(at) = first_stacked_cell(situation) {
        return Err(BattleSetupError::StackedGangers { at });
    }

    // Resolve every ganger's weapon key against the registry up front — BEFORE the
    // spawn loop — so a missing key aborts setup with WeaponNotFound (no panic) with
    // no partial world spawned (the abort-first invariant). The weapon KEY now comes
    // from the resolved gang-roster member (GTW-414). The resolved bundles are cloned
    // by value (the registry's specs are Clone) and consumed by the spawn loop.
    let mut weapon_bundles = Vec::with_capacity(resolved_members.len());
    for (_placed, member) in &resolved_members {
        let Some(spec) = weapons.spec(&member.weapon) else {
            return Err(BattleSetupError::WeaponNotFound {
                weapon: member.weapon.clone(),
            });
        };
        weapon_bundles.push(spec.clone().into_bundle(member.weapon.clone()));
    }

    // Resolve every ganger's armor key against the armor registry the same way —
    // BEFORE the spawn loop — so a missing key aborts setup with ArmorNotFound (no
    // panic) with no partial world spawned (the abort-first invariant, mirroring the
    // weapon resolution; GTW-269). The armor KEY now comes from the resolved gang-roster
    // member (GTW-414). The resolved specs are copied by value (ArmorSpec is Copy) and the
    // spawn loop spawns each ganger's worn-armor-piece entities from its spec (related via
    // `Wears`; GTW-323 slice 3 — no on-ganger `WornArmor`).
    let mut armor_specs = Vec::with_capacity(resolved_members.len());
    for (_placed, member) in &resolved_members {
        let Some(spec) = armor.spec(&member.armor) else {
            return Err(BattleSetupError::ArmorNotFound {
                armor: member.armor.clone(),
            });
        };
        armor_specs.push(*spec);
    }

    // GTW-396: pre-resolve every terrain piece key (cover + slab + floor) against the
    // registry BEFORE any spawn — abort-first. If the registry is absent or a key is
    // missing, return TerrainNotFound. Validate floor costs >= MIN_MOVE_COST.
    // Collect resolved pieces in parallel to their source lists.

    // Resolve cover pieces (walls + scatter):
    let mut resolved_covers: Vec<ResolvedCoverPiece> = Vec::new();
    for cover in situation.walls.iter().chain(situation.scatter.iter()) {
        let spec = resolve_terrain_or_err(terrain, &cover.piece)?;
        let Some(resolved) = resolve_cover_spec(&cover.piece, spec) else {
            return Err(BattleSetupError::TerrainNotFound {
                piece: cover.piece.clone(),
            });
        };
        resolved_covers.push(resolved);
    }

    // Resolve slab pieces:
    let mut resolved_slabs: Vec<ResolvedSlabPiece> = Vec::new();
    for slab_spawn in &situation.slabs {
        let spec = resolve_terrain_or_err(terrain, &slab_spawn.piece)?;
        let Some(resolved) = resolve_slab_spec(&slab_spawn.piece, spec) else {
            return Err(BattleSetupError::TerrainNotFound {
                piece: slab_spawn.piece.clone(),
            });
        };
        resolved_slabs.push(resolved);
    }

    // Resolve floor: default_floor + per-cell overrides. If no terrain registry or
    // empty sentinel, skip floor resolution (use fallback_floor_cost).
    let (default_floor_cost, floor_overrides) =
        resolve_floor_costs(terrain, situation, fallback_floor_cost)?;

    // 1. Spawn each ganger with its full component set + seeded worn armor + the
    //    resolved WeaponBundle as a single `bsn!` Scene (GTW-322), keeping the
    //    synchronously-reserved Entity handle (never a numeric id — GTW-10 / GTW-12).
    //    `commands.spawn_scene(..)` reserves the Entity id immediately (so it can key
    //    occupancy), but the scene's COMPONENTS materialize a frame later on the
    //    `SpawnScene` schedule — so occupancy is keyed off the ganger's authored `at`
    //    value + the reserved id, never off the deferred `Position` component.
    let mut occupants = Vec::with_capacity(resolved_members.len());
    for (((placed, member), weapon_bundle), armor_spec) in resolved_members
        .iter()
        .copied()
        .zip(weapon_bundles)
        .zip(armor_specs)
    {
        // The ganger carries its OWN state only — NO equipment stat data (GTW-323
        // slice 3, ADR-0004). The empty InflictedWounds record (GTW-279) and the
        // GTW-291 display ceilings (HpMax / WoundsMax) ride inside `ganger_scene`; the
        // weapon + armor stats live on the related entities spawned below. GTW-414: the
        // ganger's identity + attributes come from the resolved gang-roster `member`, its
        // placement + faction from the situation-side `placed`.
        let entity = commands
            .spawn_scene(ganger_scene(placed, member, stat_tuning))
            .id();
        // GTW-323 slice 1 (ADR-0004): spawn the six worn-armor-piece entities from the
        // resolved spec and relate them to this ganger via `Wears` — using `bsn!`
        // (`queue_spawn_related_scenes::<Wears>(bsn_list!{..})`), the post-GTW-322 spawn
        // form. The framework inserts `WornBy(entity)` on each spawned piece, whose hook
        // populates the ganger's `Wears` collection. `linked_spawn` makes the pieces
        // battle-local (despawning the ganger cascade-despawns them). These piece
        // entities are the ONLY armor storage — the sim's hit-pipeline read+wear AND the
        // presenter read both go through the relationship (no on-ganger `WornArmor`).
        commands
            .entity(entity)
            .queue_spawn_related_scenes::<Wears>(worn_piece_scenes(&armor_spec));
        // GTW-323 slice 2 (ADR-0004): spawn the wielded-weapon entity from the resolved
        // `WeaponBundle` and relate it to this ganger via `Wields` — using `bsn!`
        // (`queue_spawn_related_scenes::<Wields>(..)`), the post-GTW-322 spawn form
        // (mirroring the `Wears` spawn above). The framework inserts `WieldedBy(entity)`
        // on the spawned weapon, whose hook populates the ganger's `Wields` collection.
        // `linked_spawn` makes the weapon battle-local (despawning the ganger
        // cascade-despawns it). This weapon entity is the ONLY weapon storage — the
        // sim's `fire()` read+wear AND the presenter's weapon panel / fire-mode reads
        // both go through `ganger → Wields → the weapon entity` (no on-ganger copy).
        commands
            .entity(entity)
            .queue_spawn_related_scenes::<Wields>(wielded_weapon_scenes(&weapon_bundle));
        // The occupant's silhouette band is derived from its authored stance
        // (standing → HIGH, kneeling → MID, prone → LOW) so the grid pour places the
        // occupant AND its band together (GTW-304). Keyed off the authored `at` value
        // (NOT the deferred Position component) and the reserved Entity id.
        occupants.push(OccupantPlacement::new(
            placed.at,
            entity,
            silhouette_band(*placed.stance),
        ));
    }

    // 2. Seed the cover ledger from walls + scatter (the one unified ledger).
    //    Step 2.5 (GTW-395/396): spawn ONE terrain entity per cover piece carrying
    //    the static stats + presentation hooks (TerrainGraphicKey / FootfallSound).
    //    The resolved_covers vec is in walls-then-scatter order, mirroring the
    //    situation.walls.chain(situation.scatter) iteration order below.
    let mut cover_ledger = CoverLedger::new();
    let mut terrain_pairs: Vec<(TerrainIndexKey, bevy::prelude::Entity)> = Vec::new();
    // GTW-483: capture each cover piece's occupancy TerrainKind here — derived from the
    // resolved SPEC VARIANT (`resolved.piece_kind`), NOT from which authoring list the
    // piece sat in. A Cover/Scatter spec authored in the `walls` list (or a Wall spec in
    // `scatter`) therefore reads its DEF's own kind into the occupancy grid. In
    // walls-then-scatter order, mirroring the placement-build zip below.
    let mut occupancy_kinds: Vec<TerrainKind> =
        Vec::with_capacity(situation.walls.len() + situation.scatter.len());
    let mut resolved_covers_iter = resolved_covers.into_iter();
    for cover in situation.walls.iter().chain(situation.scatter.iter()) {
        let resolved = resolved_covers_iter
            .next()
            .unwrap_or_else(|| unreachable!("resolved_covers length matches covers length"));
        occupancy_kinds.push(TerrainKind::from(resolved.piece_kind));
        let entry = CoverEntry::seeded(
            resolved.max_hp,
            resolved.height_band,
            resolved.armor_protection,
            resolved.armor_hardness,
        );
        cover_ledger.insert(cover.at, entry);
        // GTW-395/396: spawn the terrain entity for this cover piece. The entity carries
        // STATIC stats (the max HP ceiling + band + armor) plus the GTW-396 presentation
        // hooks (TerrainGraphicKey + FootfallSound). The live HP pool stays authoritative
        // in the CoverLedger. Commands::spawn is the bevy-traps #7 form (never
        // world.spawn inside a registered system).
        //
        // FootfallSound is attached but UNCONSUMED — no footfall-audio system is built
        // yet (guns-only; a future ticket wires the audio system). It is carried now so
        // authored .terrain.ron files can specify it without a schema change.
        let entity = commands
            .spawn((
                TerrainCell::new(cover.at),
                resolved.piece_kind,
                entry.max_hp,           // CoverHp — the static max (now Component)
                entry.height_band,      // HeightBand — the static band (now Component)
                entry.armor_protection, // ArmorProtection — already Component
                entry.armor_hardness,   // ArmorHardness — already Component
                resolved.graphic,       // TerrainGraphicKey — presenter resolves to atlas entry
                resolved.footfall, // FootfallSound — future footfall-audio pass (GTW-XXX: footfall audio system consumes this)
            ))
            .id();
        terrain_pairs.push((TerrainIndexKey::Cover(cover.at), entity));
    }
    commands.insert_resource(cover_ledger);

    // GTW-391/392: hoist the stair-cell and brace-stair-cell computations ABOVE the slab
    // loop so the slab loop can consult the brace set when spawning slab entities. Verified
    // safe (Blocker-2 fix): the slab loop body reads only surface_grid / slab_ledger /
    // commands / resolved_slabs — none of the stair state produced here.
    //
    // GTW-391: both endpoints of every Stair link (for LOS eye-lift + dual-cell presence).
    // Ladders are excluded (no stair eye-lift / no dual-cell body presence).
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
    let brace_cells_resource = BraceStairCells::new(brace_stair_cells_set.clone());

    // 3. Seed the surface grid: every authored slab is Present (zero ground damage
    //    by lazy default). Eagerly seed the SlabLedger from the resolved per-slab
    //    specs (GTW-396 Decision C, major #4): `entry_seeded` returns an eagerly-inserted
    //    entry unchanged on subsequent calls, so the authored per-slab HP is honored on
    //    first strike via the normal depletion path (no deplete_slab signature change).
    //    Also spawn ONE terrain entity per slab carrying the static stats + hooks.
    let mut surface_grid = SurfaceGrid::new();
    let mut slab_ledger = SlabLedger::new();
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

        // GTW-395/396: spawn the slab entity. Carries STATIC stats (max HP / armor)
        // plus GTW-396 presentation hooks. The live pool stays in slab_ledger.
        //
        // FootfallSound is attached but UNCONSUMED (guns-only; future footfall-audio
        // ticket). GTW-XXX: footfall audio system consumes this.
        let slab_entity = commands
            .spawn((
                TerrainCell::new(slab_spawn.at),
                TerrainPieceKind::Slab,
                resolved.max_hp,           // SlabHp — the static max (now Component)
                resolved.armor_protection, // ArmorProtection — already Component
                resolved.armor_hardness,   // ArmorHardness — already Component
                resolved.graphic.clone(),  // TerrainGraphicKey — presenter resolves to atlas entry
                resolved.footfall.clone(), // FootfallSound — future footfall-audio pass
            ))
            .id();

        // GTW-392: a slab is a stair-brace slab when the cell DIRECTLY BELOW it is a
        // brace-eligible (LOWER-endpoint) stair cell — a kneeling occupant on that stair
        // braces under this slab. Uses the brace-only set (lower endpoints), NOT
        // `stair_cell_set` (which has both endpoints for GTW-391 LOS/presence).
        //
        // `cell_below` returns None for level 0 (no cell below ground) — a level-0 slab
        // never becomes a brace slab (correct: ground level has no lower stair endpoint).
        if let Some(below) = cell_below(slab_spawn.at)
            && brace_stair_cells_set.contains(&below)
        {
            commands.entity(slab_entity).insert(TerrainBrace);
        }

        terrain_pairs.push((TerrainIndexKey::Slab(slab_spawn.at), slab_entity));
    }
    commands.insert_resource(surface_grid);
    commands.insert_resource(slab_ledger);

    // GTW-392: insert the BraceStairCells resource (the lower-endpoint stair-cell set).
    // Removed at teardown alongside the other battle-lifetime resources.
    commands.insert_resource(brace_cells_resource);

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

    Ok(setup)
}

/// Whether `situation`'s authored ganger cells contain a duplicate — two gangers
/// spawned on the SAME `(cell, level)`.
///
/// A thin `bool` wrapper over the private `first_stacked_cell` detection — `true`
/// iff some `(cell, level)` is authored for more than one ganger. [`setup_battle`] ENFORCES
/// this invariant (GTW-457): a duplicate aborts setup with
/// [`BattleSetupError::StackedGangers`] before any entity is spawned, since the
/// GTW-156 occupancy pour is last-write-wins and would otherwise silently overwrite
/// the first ganger's slot while both entities survive stacked on one cell.
#[must_use]
pub fn has_stacked_gangers(situation: &Situation) -> bool {
    first_stacked_cell(situation).is_some()
}

/// The first `(cell, level)` two or more authored gangers share, in
/// [`Situation::gangers`](crate::situation::Situation) order — or `None` when every
/// ganger has a distinct spawn cell.
///
/// The single HashSet-over-`at` detection both [`has_stacked_gangers`] (the `bool`
/// view) and the [`setup_battle`] pre-spawn gate (GTW-457 — the ENFORCED view that
/// needs the offending cell for [`BattleSetupError::StackedGangers`]) read.
#[must_use]
fn first_stacked_cell(situation: &Situation) -> Option<crate::metric::CellLevel> {
    let mut seen = HashSet::new();
    situation
        .gangers
        .iter()
        .find(|g| !seen.insert(g.at))
        .map(|g| g.at)
}

/// The cell directly below `cell` (`(x, y, z − 1)`), or `None` when `cell` is at
/// level 0 (no cell below ground).
///
/// A private helper for GTW-392 stair-brace slab placement: a slab at `(x, y, z)`
/// is a brace slab only when the cell at `(x, y, z − 1)` is a brace-eligible stair
/// cell. Level-0 slabs never become brace slabs (their `z − 1` would be negative).
fn cell_below(cell: crate::metric::CellLevel) -> Option<crate::metric::CellLevel> {
    let below_z = cell.z.checked_sub(1)?;
    // below_z >= 0 (checked_sub returned Some) and below_z <= MAX_LEVELS, so the
    // u8 conversion is lossless. If it somehow overflows (impossible in a valid grid),
    // try_from returns Err and we return None — fail-safe.
    let storey = u8::try_from(below_z).ok()?;
    let level = crate::metric::Level::new(storey);
    Some(crate::metric::CellLevel::new(
        crate::metric::Cell::new(cell.x, cell.y),
        level,
    ))
}
