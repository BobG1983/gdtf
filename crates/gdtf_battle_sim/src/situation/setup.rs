//! [`setup_battle`] — pour a [`Situation`](crate::situation::Situation) into the
//! battle ECS — plus its [`BattleSetup`] result and the
//! [`has_stacked_gangers`] sanity helper.

use bevy::{platform::collections::HashSet, prelude::Commands};

use crate::{
    armor::WornArmor,
    cover::CoverLedger,
    ganger::Position,
    inflicted_wound::InflictedWounds,
    occupancy::{OccupancyGrid, OccupancyInput, OccupantPlacement, TerrainPlacement},
    situation::{BattleSetupError, Situation},
    surface::{SlabState, SurfaceGrid},
    vertical::build_vertical_link_graph,
    weapon::WeaponRegistry,
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

/// Build the battle in the ECS world from a [`Situation`](crate::situation::Situation)
/// — the E1.8 setup: the setup system that builds the scene from the situation (see the
/// [`crate::situation`] module doc, the setup-on-entry source of truth).
///
/// Steps, in order:
///
/// 1. **Spawn each ganger** — for every [`GangerSpawn`](crate::situation::GangerSpawn),
///    `commands.spawn(...)` the full per-field component set ([`Position`] from `at`,
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
///    from the ganger's roster [`SourceArmor`](crate::armor::SourceArmor)
///    ([`WornArmor::seed_from`](crate::armor::WornArmor::seed_from)), and the GTW-279
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
/// [`weapon`](crate::situation::GangerSpawn::weapon) key.
///
/// # Errors
///
/// Returns a [`BattleSetupError`]:
/// - [`BattleSetupError::InvalidLink`] if any authored vertical link fails
///   validation (level out of range, dangling endpoint, or same-storey) — see
///   [`build_vertical_link_graph`];
/// - [`BattleSetupError::WeaponNotFound`] if any ganger's
///   [`weapon`](crate::situation::GangerSpawn::weapon) key is absent from `weapons`
///   (no loaded `assets/weapons/*.ron` with that stem).
///
/// Both are validated BEFORE any entity is spawned or any resource inserted, so a
/// failure leaves no partial, unspawnable world behind (the GTW-205 abort-first
/// invariant, extended to the weapon resolution).
pub fn setup_battle(
    situation: &Situation,
    weapons: &WeaponRegistry,
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

    // 1. Spawn each ganger with its full component set + seeded worn armor + the
    //    resolved WeaponBundle, keeping the returned Entity handle (never a numeric id
    //    — GTW-10 / GTW-12). The weapon bundle is inserted in a SECOND `insert` call
    //    (the spawn tuple is already at its component-arity limit, and a Bundle inserts
    //    its whole component set in one call), mirroring the WornArmor seed-from at the
    //    spawn tuple.
    let mut occupants = Vec::with_capacity(situation.gangers.len());
    for (ganger, weapon_bundle) in situation.gangers.iter().zip(weapon_bundles) {
        let entity = commands
            .spawn((
                Position::new(ganger.at),
                ganger.name.clone(),
                ganger.faction,
                ganger.facing,
                ganger.stance,
                ganger.aiming,
                ganger.hp,
                ganger.wounds,
                ganger.tu,
                ganger.tu_max,
                ganger.life_state,
                ganger.shooting,
                ganger.toughness,
                ganger.luck,
                WornArmor::seed_from(&ganger.armor),
            ))
            // The weapon bundle + the GTW-279 empty inflicted-wound record + the
            // GTW-291 display ceilings (HpMax / WoundsMax) are inserted in a SECOND
            // `insert` call — the spawn tuple is already at its 15-component arity
            // limit. InflictedWounds is the additive record the resolution path
            // appends to (seeded empty here); HpMax / WoundsMax are the authored full
            // capacities the status panel reads as the HP-bar denominator / Wounds-pip
            // count — pure DISPLAY ceilings, NOT round-reset targets (no reset wiring),
            // seeded beside the persistent Hp / Wounds pools.
            .insert((
                weapon_bundle,
                InflictedWounds::default(),
                ganger.hp_max,
                ganger.wounds_max,
            ))
            .id();
        occupants.push(OccupantPlacement::new(ganger.at, entity));
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
