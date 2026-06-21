//! [`setup_battle`] — pour a [`Situation`](crate::situation::Situation) into the
//! battle ECS — plus its [`BattleSetup`] result and the
//! [`has_stacked_gangers`] sanity helper.

use bevy::{
    platform::collections::HashSet,
    prelude::Commands,
    scene::{CommandsSceneExt, Scene, bsn, template_value},
};

use crate::{
    armor::{ArmorRegistry, WornArmor},
    clearance::silhouette_band,
    cover::CoverLedger,
    ganger::{
        Aiming, Facing, Faction, GangerName, Hp, HpMax, Luck, Position, Shooting, Stance,
        Toughness, Tu, TuMax, Wounds, WoundsMax,
    },
    inflicted_wound::InflictedWounds,
    occupancy::{OccupancyGrid, OccupancyInput, OccupantPlacement, TerrainPlacement},
    situation::{BattleSetupError, GangerSpawn, Situation},
    surface::{SlabState, SurfaceGrid},
    vertical::build_vertical_link_graph,
    weapon::{
        Accuracy, BaseSpread, FatalBias, FireMode, Kickback, Stable, Weapon, WeaponBundle,
        WeaponDamage, WeaponName, WeaponPunch, WeaponRegistry, WeaponShred,
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

/// Compose ONE ganger as a Bevy `bsn!` [`Scene`] — the per-field component tree
/// [`setup_battle`] spawns for each [`GangerSpawn`](crate::situation::GangerSpawn)
/// (GTW-322).
///
/// Faithfully mirrors the old `commands.spawn(tuple).insert(bundle)` shape: the
/// per-field ganger state ([`Position`] / [`GangerName`] / [`Faction`] / [`Facing`] /
/// [`Stance`] / [`Aiming`] / [`Hp`] / [`HpMax`] / [`Wounds`] / [`WoundsMax`] / [`Tu`] /
/// [`TuMax`] / [`LifeState`](crate::ganger::LifeState)), the E3.0 / GTW-182 attribute stats ([`Shooting`] /
/// [`Toughness`] / [`Luck`]), the resolved [`WeaponBundle`]'s components ([`Weapon`]
/// marker + every weapon stat), the seeded [`WornArmor`], and the GTW-279 empty
/// [`InflictedWounds`] record — the SAME set, with the SAME authored values, that the
/// old tuple + second `insert` produced (behavior-preserving).
///
/// **The `bsn!` recipe (GTW-322 spike).** Every newtype with a `Type::new(value)`
/// constructor is inlined in `bsn!`. The runtime-valued fieldless enums
/// ([`LifeState`](crate::ganger::LifeState) and the weapon's [`DamageType`](crate::weapon::DamageType)) and the
/// value-typed [`Magazine`](crate::magazine::Magazine) / [`WornArmor`] (no
/// `new(value)` whole-value ctor) have NO `bsn!` grammar form — inlining a variant
/// would NARROW the authored value — so they are bridged via
/// [`template_value`](bevy::scene::template_value) and tuple-composed onto the SAME
/// root entity. Each composed type carries a GTW-322 spawn-seed-sentinel [`Default`].
///
/// **Deferred materialization.** `bsn!`-scene components materialize on the
/// `SpawnScene` schedule (~one `app.update()` later), NOT synchronously. The caller
/// keys occupancy off the [`GangerSpawn`]'s authored
/// [`at`](crate::situation::GangerSpawn::at) value and the synchronously-reserved
/// `Entity` id (`spawn_scene(..).id()`), never off the deferred [`Position`]
/// component.
fn ganger_scene(ganger: &GangerSpawn, weapon: &WeaponBundle, armor: &WornArmor) -> impl Scene {
    // The `bsn!` `Type::new(expr)` form stores a DEFERRED constructor, so every value
    // it captures must be OWNED/`'static` — a borrow (`&GangerSpawn` / `&WeaponBundle`)
    // captured into the macro would make the returned scene outlive the references (the
    // GTW-322 spike's `'static` finding). So bind every inline value to an owned local
    // FIRST, and let the macro capture those owned locals (never the `&` params).
    let at = ganger.at;
    let name = (*ganger.name).clone();
    let faction = *ganger.faction;
    let facing = *ganger.facing;
    let stance = *ganger.stance;
    let aiming = *ganger.aiming;
    let hp = *ganger.hp;
    let hp_max = *ganger.hp_max;
    let wounds = *ganger.wounds;
    let wounds_max = *ganger.wounds_max;
    let tu = *ganger.tu;
    let tu_max = *ganger.tu_max;
    let shooting = *ganger.shooting;
    let toughness = *ganger.toughness;
    let luck = *ganger.luck;
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
    // The runtime-valued / value-typed leaves with no `bsn!` grammar form, owned for
    // the `template_value` tuple-composition tail (see the doc-comment recipe).
    let life_state = ganger.life_state;
    let damage_type = weapon.damage_type;
    let magazine = weapon.magazine;
    let worn = *armor;
    (
        bsn! {
            Position::new(at)
            GangerName::new(name)
            Faction::new(faction)
            Facing::new(facing)
            Stance::new(stance)
            Aiming::new(aiming)
            Hp::new(hp)
            HpMax::new(hp_max)
            Wounds::new(wounds)
            WoundsMax::new(wounds_max)
            Tu::new(tu)
            TuMax::new(tu_max)
            Shooting::new(shooting)
            Toughness::new(toughness)
            Luck::new(luck)
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
            InflictedWounds::default()
        },
        // Runtime-valued / value-typed components with no `bsn!` grammar form, bridged
        // via `template_value` and tuple-composed onto the SAME root entity (the
        // GTW-322 spike's canonical runtime-value path).
        template_value(life_state),
        template_value(damage_type),
        template_value(magazine),
        template_value(worn),
    )
}

/// Build the battle in the ECS world from a [`Situation`](crate::situation::Situation)
/// — the E1.8 setup: the setup system that builds the scene from the situation (see the
/// [`crate::situation`] module doc, the setup-on-entry source of truth).
///
/// Steps, in order:
///
/// 1. **Spawn each ganger** — for every [`GangerSpawn`](crate::situation::GangerSpawn),
///    `commands.spawn_scene(`[`ganger_scene`]`(..))` the full per-field component set as a
///    Bevy `bsn!` [`Scene`] (GTW-322; the SAME entity tree the old spawn-tuple +
///    second `insert` produced) — [`Position`] from `at`,
///    the [`GangerName`](crate::ganger::GangerName) identity,
///    plus [`Faction`](crate::ganger::Faction) / [`Facing`](crate::ganger::Facing) /
///    [`Stance`](crate::ganger::Stance) / [`Aiming`](crate::ganger::Aiming) /
///    [`Hp`](crate::ganger::Hp) / [`HpMax`](crate::ganger::HpMax) /
///    [`Wounds`](crate::ganger::Wounds) / [`WoundsMax`](crate::ganger::WoundsMax) /
///    [`Tu`](crate::ganger::Tu) / [`TuMax`](crate::ganger::TuMax) /
///    [`LifeState`](crate::ganger::LifeState)), the E3.0 /
///    GTW-182 attribute stats ([`Shooting`](crate::ganger::Shooting) /
///    [`Toughness`](crate::ganger::Toughness) / [`Luck`](crate::ganger::Luck)) the
///    severity roll reads, the [`WeaponBundle`](crate::weapon::WeaponBundle) resolved
///    from the ganger's [`weapon`](crate::situation::GangerSpawn::weapon) key against
///    the [`WeaponRegistry`] (GTW-257 — the [`Weapon`](crate::weapon::Weapon) marker +
///    every weapon stat component), PLUS the battle-local [`WornArmor`] seeded by value
///    from the [`ArmorSpec`](crate::armor::ArmorSpec) resolved from the ganger's
///    [`armor`](crate::situation::GangerSpawn::armor) key against the [`ArmorRegistry`]
///    (GTW-269 — [`WornArmor::seed_from`](crate::armor::WornArmor::seed_from)), and the GTW-279
///    [`InflictedWounds`] record seeded **empty** (the [`Default`]) so a fresh ganger
///    starts with no recorded wounds. The returned Bevy
///    [`Entity`](bevy::prelude::Entity) handle is captured into the
///    [`OccupantPlacement`] list — NEVER a numeric id (GTW-10 / GTW-12).
/// 2. **Seed the [`CoverLedger`]** — insert a [`CoverEntry`](crate::cover::CoverEntry)
///    for every wall and scatter piece (the one unified ledger).
/// 3. **Seed the [`SurfaceGrid`]** — set [`SlabState::Present`] at every authored
///    slab (ground damage starts at zero by lazy default).
/// 4. **Build the [`OccupancyGrid`]** — pour an [`OccupancyInput`] of the authored
///    terrain (walls + scatter → their [`TerrainKind`](crate::occupancy::TerrainKind))
///    and the SPAWNED occupant entities through
///    [`OccupancyGrid::build_from_occupancy_input`](crate::occupancy::OccupancyGrid::build_from_occupancy_input).
/// 5. **Validate + build the [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph)** — via
///    [`build_vertical_link_graph`]; on success insert it, on failure return the
///    typed [`InvalidVerticalLink`](crate::vertical::InvalidVerticalLink) (the no-panic
///    contract). The gangers are spawned and the other three resources inserted
///    regardless — a bad vertical link does not unspawn them; the caller treats the
///    error as a setup abort.
///
/// All four resources ([`CoverLedger`], [`SurfaceGrid`], [`OccupancyGrid`],
/// [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph)) are inserted via [`Commands`]. The function is
/// render-free and headless-driven (it touches no renderer / asset server), so a
/// `MinimalPlugins` test can run it directly. It reads a [`WeaponRegistry`] by
/// reference (GTW-257) to resolve each ganger's
/// [`weapon`](crate::situation::GangerSpawn::weapon) key, and an [`ArmorRegistry`] by
/// reference (GTW-269) to resolve each ganger's
/// [`armor`](crate::situation::GangerSpawn::armor) key.
///
/// # Errors
///
/// Returns a [`BattleSetupError`]:
/// - [`BattleSetupError::InvalidLink`] if any authored vertical link fails
///   validation (level out of range, dangling endpoint, or same-storey) — see
///   [`build_vertical_link_graph`];
/// - [`BattleSetupError::WeaponNotFound`] if any ganger's
///   [`weapon`](crate::situation::GangerSpawn::weapon) key is absent from `weapons`
///   (no loaded `assets/weapons/*.ron` with that stem);
/// - [`BattleSetupError::ArmorNotFound`] if any ganger's
///   [`armor`](crate::situation::GangerSpawn::armor) key is absent from `armor`
///   (no loaded `assets/armor/*.armor.ron` with that stem).
///
/// All three are validated BEFORE any entity is spawned or any resource inserted, so a
/// failure leaves no partial, unspawnable world behind (the GTW-205 abort-first
/// invariant, extended to the weapon + armor resolution).
pub fn setup_battle(
    situation: &Situation,
    weapons: &WeaponRegistry,
    armor: &ArmorRegistry,
    commands: &mut Commands,
) -> Result<BattleSetup, BattleSetupError> {
    // Validate the vertical links FIRST, so a bad authored link aborts the whole
    // setup before any entity is spawned or any resource inserted (no partial,
    // unspawnable world left behind on a validation failure).
    let vertical_graph = build_vertical_link_graph(situation)?;

    // Resolve every ganger's weapon key against the registry up front — BEFORE the
    // spawn loop — so a missing key aborts setup with WeaponNotFound (no panic) with
    // no partial world spawned (the abort-first invariant). The resolved bundles are
    // cloned by value (the registry's specs are Clone) and consumed by the spawn loop.
    let mut weapon_bundles = Vec::with_capacity(situation.gangers.len());
    for ganger in &situation.gangers {
        let Some(spec) = weapons.spec(&ganger.weapon) else {
            return Err(BattleSetupError::WeaponNotFound {
                weapon: ganger.weapon.clone(),
            });
        };
        weapon_bundles.push(spec.clone().into_bundle(ganger.weapon.clone()));
    }

    // Resolve every ganger's armor key against the armor registry the same way —
    // BEFORE the spawn loop — so a missing key aborts setup with ArmorNotFound (no
    // panic) with no partial world spawned (the abort-first invariant, mirroring the
    // weapon resolution; GTW-269). The resolved specs are copied by value (ArmorSpec is
    // Copy) and the spawn loop seeds each ganger's WornArmor from its spec.
    let mut armor_specs = Vec::with_capacity(situation.gangers.len());
    for ganger in &situation.gangers {
        let Some(spec) = armor.spec(&ganger.armor) else {
            return Err(BattleSetupError::ArmorNotFound {
                armor: ganger.armor.clone(),
            });
        };
        armor_specs.push(*spec);
    }

    // 1. Spawn each ganger with its full component set + seeded worn armor + the
    //    resolved WeaponBundle as a single `bsn!` Scene (GTW-322), keeping the
    //    synchronously-reserved Entity handle (never a numeric id — GTW-10 / GTW-12).
    //    `commands.spawn_scene(..)` reserves the Entity id immediately (so it can key
    //    occupancy), but the scene's COMPONENTS materialize a frame later on the
    //    `SpawnScene` schedule — so occupancy is keyed off the ganger's authored `at`
    //    value + the reserved id, never off the deferred `Position` component.
    let mut occupants = Vec::with_capacity(situation.gangers.len());
    for ((ganger, weapon_bundle), armor_spec) in situation
        .gangers
        .iter()
        .zip(weapon_bundles)
        .zip(armor_specs)
    {
        // The battle-local worn armor seeded by value from the resolved spec, composed
        // onto the scene (GTW-269); the empty InflictedWounds record (GTW-279) and the
        // GTW-291 display ceilings (HpMax / WoundsMax) ride inside `ganger_scene`.
        let worn = WornArmor::seed_from(&armor_spec);
        let entity = commands
            .spawn_scene(ganger_scene(ganger, &weapon_bundle, &worn))
            .id();
        // The occupant's silhouette band is derived from its authored stance
        // (standing → HIGH, kneeling → MID, prone → LOW) so the grid pour places the
        // occupant AND its band together (GTW-304). Keyed off the authored `at` value
        // (NOT the deferred Position component) and the reserved Entity id.
        occupants.push(OccupantPlacement::new(
            ganger.at,
            entity,
            silhouette_band(*ganger.stance),
        ));
    }

    // 2. Seed the cover ledger from walls + scatter (the one unified ledger).
    let mut cover_ledger = CoverLedger::new();
    for cover in situation.walls.iter().chain(situation.scatter.iter()) {
        cover_ledger.insert(cover.at, cover.cover_entry());
    }
    commands.insert_resource(cover_ledger);

    // 3. Seed the surface grid: every authored slab is Present (zero ground damage
    //    by lazy default).
    let mut surface_grid = SurfaceGrid::new();
    for &slab in &situation.slabs {
        surface_grid.set_slab(slab, SlabState::Present);
    }
    commands.insert_resource(surface_grid);

    // Own the placements in the result up front, so the occupancy grid can borrow
    // them (no clone) and the same Vec is returned to the caller.
    let setup = BattleSetup { occupants };

    // 4. Build the occupancy grid: terrain from walls + scatter, occupants from the
    //    spawned entities (borrowed from the result's placement list).
    let terrain: Vec<TerrainPlacement> = situation
        .walls
        .iter()
        .chain(situation.scatter.iter())
        .map(|c| TerrainPlacement::new(c.at, c.terrain))
        .collect();
    let occupancy_input = OccupancyInput {
        terrain,
        occupants: setup.occupants.clone(),
    };
    commands.insert_resource(OccupancyGrid::build_from_occupancy_input(&occupancy_input));

    // 5. The validated vertical-link graph (validation already ran above).
    commands.insert_resource(vertical_graph);

    Ok(setup)
}

/// Whether `situation`'s authored ganger cells contain a duplicate — two gangers
/// spawned on the SAME `(cell, level)`.
///
/// A setup-time sanity helper (not an error in [`setup_battle`] — the occupancy
/// pour keeps the last write, matching the GTW-156 contract — but useful for a
/// caller / test to detect an over-stacked situation). Returns `true` if any
/// `(cell, level)` is authored for more than one ganger.
#[must_use]
pub fn has_stacked_gangers(situation: &Situation) -> bool {
    let mut seen = HashSet::new();
    situation.gangers.iter().any(|g| !seen.insert(g.at))
}
