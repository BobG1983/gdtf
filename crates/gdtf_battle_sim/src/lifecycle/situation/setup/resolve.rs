//! Abort-first pre-spawn resolution + validation of every authored key — the
//! no-partial-world invariant: [`setup_battle`](super::setup_battle) resolves every
//! authored reference (gang members, weapon / melee / armor keys, cover / slab
//! terrain UUIDs, field keys) BEFORE any entity is spawned or resource inserted, so
//! a bad reference aborts with a typed [`BattleSetupError`] and no partial world.

use bevy::{platform::collections::HashSet, prelude::Deref};

use super::super::terrain_resolve::{
    ResolvedCoverPiece, ResolvedSlabPiece, resolve_cover_def, resolve_slab_def,
    resolve_terrain_or_err,
};
use crate::{
    armor::{ArmorRegistry, ArmorSpec},
    effects::fields::{FieldDefRegistry, FieldRegistry},
    equipment::attachments::{AttachmentRegistry, resolve_pending_attachments},
    ganger::{GangMember, GangRegistry},
    situation::{BattleSetupError, PlacedGanger, Situation},
    terrain::def::TerrainDefRegistry,
    weapon::{
        FISTS_KEY, MeleeWeaponBundle, MeleeWeaponRegistry, PendingAttachments, WeaponBundle,
        WeaponName, WeaponRegistry, WeaponSpawnSiblings,
    },
};

/// GTW-414 schema v2: resolve every [`PlacedGanger`]'s `(gang, member)` ref against
/// the [`GangRegistry`] up front — BEFORE any spawn — so a missing gang/member aborts
/// setup with the typed error (no panic) with no partial world spawned (the
/// abort-first invariant). The resolved roster member supplies the identity + eight
/// attributes + weapon/armor keys the rest of the setup reads (the old `GangerSpawn`
/// fields, now sourced from the gang roster). Each `(placed, member)` borrow is held
/// in [`PlacedGanger`] order, parallel to [`Situation::gangers`].
pub(super) fn resolve_members<'s, 'g>(
    situation: &'s Situation,
    gangs: &'g GangRegistry,
) -> Result<Vec<(&'s PlacedGanger, &'g GangMember)>, BattleSetupError> {
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
    Ok(resolved_members)
}

/// Resolve every ganger's weapon key against the registry up front — BEFORE the
/// spawn loop — so a missing key aborts setup with
/// [`WeaponNotFound`](BattleSetupError::WeaponNotFound) (no panic) with no partial
/// world spawned (the abort-first invariant). The weapon KEY comes from the resolved
/// gang-roster member (GTW-414). The resolved bundles are cloned by value (the
/// registry's specs are `Clone`) and consumed by the spawn loop.
pub(super) fn resolve_weapon_bundles(
    resolved_members: &[(&PlacedGanger, &GangMember)],
    weapons: &WeaponRegistry,
    attachments: Option<&AttachmentRegistry>,
) -> Result<Vec<(WeaponBundle, WeaponSpawnSiblings, PendingAttachments)>, BattleSetupError> {
    let mut weapon_bundles = Vec::with_capacity(resolved_members.len());
    for (_placed, member) in resolved_members {
        let Some(spec) = weapons.spec(&member.weapon) else {
            return Err(BattleSetupError::WeaponNotFound {
                weapon: member.weapon.clone(),
            });
        };
        // GTW-549: resolve the weapon's authored attachment KEYS against the registry into the
        // flat list of AttachmentEffects to apply — GATED by the GTW-554 slot fit (an item
        // whose slot the weapon never declares, or whose slot is at capacity, is CLEANLY
        // rejected). A missing registry (`None`) or an unresolved key contributes NO effects
        // (the fail-safe — a missing attachment applies nothing, never fails a battle). The
        // effects ride onto the spawned weapon as a PendingAttachments marker the post-spawn
        // `apply_pending_attachments` system applies.
        let pending = resolve_pending_attachments(&spec.slots, &spec.attachments, attachments);
        // GTW-544/547: `into_bundle` returns the resolved WeaponBundle + the optional `dot` /
        // `on_death` siblings; attachment effects are NO LONGER folded here (they apply
        // post-spawn via the commands extension).
        let (bundle, siblings) = spec.clone().into_bundle(member.weapon.clone());
        weapon_bundles.push((bundle, siblings, pending));
    }
    Ok(weapon_bundles)
}

/// GTW-505: resolve every ganger's MELEE weapon BEFORE the spawn loop too (abort-first,
/// mirroring the ranged path). The key is the member's authored `melee_weapon` key, OR
/// — when it authored none — the shipped `fists` default, so EVERY ganger gets a melee
/// weapon and any ganger can melee (the GTW-37 D3 ruling). A missing key (incl. a
/// missing `fists.melee_weapon.ron`) aborts with
/// [`MeleeWeaponNotFound`](BattleSetupError::MeleeWeaponNotFound) (no panic, no
/// partial world). The resolved bundles are cloned by value (the registry's specs are
/// `Clone`) and consumed by the spawn loop alongside the ranged bundle.
pub(super) fn resolve_melee_bundles(
    resolved_members: &[(&PlacedGanger, &GangMember)],
    melee_weapons: &MeleeWeaponRegistry,
    attachments: Option<&AttachmentRegistry>,
) -> Result<Vec<(MeleeWeaponBundle, PendingAttachments)>, BattleSetupError> {
    let mut melee_bundles = Vec::with_capacity(resolved_members.len());
    for (_placed, member) in resolved_members {
        // The melee KEY: the member's authored key, else the `fists` default.
        let melee_key = member
            .melee_weapon
            .clone()
            .unwrap_or_else(|| WeaponName::new(FISTS_KEY.to_owned()));
        let Some(spec) = melee_weapons.spec(&melee_key) else {
            return Err(BattleSetupError::MeleeWeaponNotFound { weapon: melee_key });
        };
        // GTW-554: melee weapons gain FULL attachment support — resolve the melee spec's
        // authored attachment keys through the SAME slot-gated seam as the ranged path (a
        // Counterweight/Pommel item fits only a melee weapon declaring that slot; ranged-style
        // items find no slot and are cleanly rejected). The resolved effects ride onto the
        // spawned MELEE weapon entity as its own PendingAttachments marker.
        let pending = resolve_pending_attachments(&spec.slots, &spec.attachments, attachments);
        melee_bundles.push((spec.clone().into_bundle(melee_key), pending));
    }
    Ok(melee_bundles)
}

/// Resolve every ganger's armor key against the armor registry the same way — BEFORE
/// the spawn loop — so a missing key aborts setup with
/// [`ArmorNotFound`](BattleSetupError::ArmorNotFound) (no panic) with no partial world
/// spawned (the abort-first invariant, mirroring the weapon resolution; GTW-269). The
/// armor KEY comes from the resolved gang-roster member (GTW-414). The resolved specs
/// are copied by value ([`ArmorSpec`] is `Copy`) and the spawn loop spawns each
/// ganger's worn-armor-piece entities from its spec (related via
/// [`Wears`](crate::armor::Wears); GTW-323 slice 3 — no on-ganger `WornArmor`).
pub(super) fn resolve_armor_specs(
    resolved_members: &[(&PlacedGanger, &GangMember)],
    armor: &ArmorRegistry,
) -> Result<Vec<ArmorSpec>, BattleSetupError> {
    let mut armor_specs = Vec::with_capacity(resolved_members.len());
    for (_placed, member) in resolved_members {
        let Some(spec) = armor.spec(&member.armor) else {
            return Err(BattleSetupError::ArmorNotFound {
                armor: member.armor.clone(),
            });
        };
        armor_specs.push(*spec);
    }
    Ok(armor_specs)
}

/// GTW-491: pre-resolve every COVER terrain definition UUID (walls + scatter) against
/// the [`TerrainDefRegistry`] BEFORE any spawn — abort-first. If the registry is
/// absent or a key is missing, return
/// [`TerrainNotFound`](BattleSetupError::TerrainNotFound) keyed by the unresolved
/// `TerrainUuid`. The resolved pieces are collected in walls-then-scatter order,
/// parallel to the source lists. GTW-547: each cover piece's authored on-death effect
/// is captured (keyed by its cell) as we resolve — a destroyed cover cell fans it via
/// `resolve_on_death` (cover is not an entity, so the effect lives in the
/// [`CoverOnDeathRegistry`](crate::effects::on_death::CoverOnDeathRegistry) keyed by cell, not
/// on a component).
#[expect(
    clippy::type_complexity,
    reason = "the resolved cover pieces and their captured on-death entries are produced \
              by ONE walls-then-scatter walk and consumed together by the orchestrator; \
              splitting the pair into a named struct would add a new abstraction the \
              GTW-583 split rules strike (P9 — no new types to shrink counts)"
)]
pub(super) fn resolve_covers(
    situation: &Situation,
    terrain: Option<&TerrainDefRegistry>,
) -> Result<
    (
        Vec<ResolvedCoverPiece>,
        Vec<(
            crate::metric::CellLevel,
            crate::effects::on_death::OnDeathEffect,
        )>,
    ),
    BattleSetupError,
> {
    let mut resolved_covers: Vec<ResolvedCoverPiece> = Vec::new();
    // GTW-547: capture each cover piece's authored on-death effect (keyed by its cell) as we
    // resolve — a destroyed cover cell fans it via `resolve_on_death` (cover is not an entity,
    // so the effect lives in the CoverOnDeathRegistry keyed by cell, not on a component).
    let mut cover_on_death_entries: Vec<(
        crate::metric::CellLevel,
        crate::effects::on_death::OnDeathEffect,
    )> = Vec::new();
    for cover in situation.walls.iter().chain(situation.scatter.iter()) {
        let def = resolve_terrain_or_err(terrain, &cover.piece)?;
        if let Some(effect) = &def.on_death {
            cover_on_death_entries.push((cover.at, effect.clone()));
        }
        let Some(resolved) = resolve_cover_def(&cover.piece, def) else {
            return Err(BattleSetupError::TerrainNotFound { piece: cover.piece });
        };
        resolved_covers.push(resolved);
    }
    Ok((resolved_covers, cover_on_death_entries))
}

/// GTW-491: pre-resolve every SLAB terrain definition UUID against the
/// [`TerrainDefRegistry`] BEFORE any spawn — abort-first, mirroring the cover
/// pre-resolve. A missing registry or key returns
/// [`TerrainNotFound`](BattleSetupError::TerrainNotFound) keyed by the unresolved
/// `TerrainUuid`; the resolved pieces are collected parallel to
/// [`Situation::slabs`].
pub(super) fn resolve_slabs(
    situation: &Situation,
    terrain: Option<&TerrainDefRegistry>,
) -> Result<Vec<ResolvedSlabPiece>, BattleSetupError> {
    let mut resolved_slabs: Vec<ResolvedSlabPiece> = Vec::new();
    for slab_spawn in &situation.slabs {
        let def = resolve_terrain_or_err(terrain, &slab_spawn.piece)?;
        let Some(resolved) = resolve_slab_def(&slab_spawn.piece, def) else {
            return Err(BattleSetupError::TerrainNotFound {
                piece: slab_spawn.piece,
            });
        };
        resolved_slabs.push(resolved);
    }
    Ok(resolved_slabs)
}

/// Build the live [`FieldRegistry`] from a situation's authored
/// [`fields`](crate::situation::Situation::fields) placements, resolving each
/// [`FieldKey`](crate::effects::fields::FieldKey) against the [`FieldDefRegistry`] catalog (GTW-545).
///
/// A private [`setup_battle`](super::setup_battle) helper (mirroring the existing pre-resolve helpers) so the field
/// seed phase is a single named call rather than a seventh inline phase body — keeping
/// `setup_battle` under clippy's line-count gate. Abort-first: a placement whose field key is
/// absent from the catalog (or a placement authored with NO catalog loaded) returns
/// [`BattleSetupError::FieldNotFound`] BEFORE any resource is inserted, so a bad field
/// reference leaves no partial world behind. A situation with NO authored fields returns an
/// empty registry regardless of whether a catalog is present (every test fixture path).
pub(super) fn build_field_registry(
    situation: &Situation,
    catalog: Option<&FieldDefRegistry>,
) -> Result<FieldRegistry, BattleSetupError> {
    let mut registry = FieldRegistry::new();
    for spawn in &situation.fields {
        // Resolve the field key against the catalog. A missing catalog OR a missing key both
        // fail closed with FieldNotFound (no panic) — the app always loads the catalog first.
        let Some(def) = catalog.and_then(|c| c.def(&spawn.field)) else {
            return Err(BattleSetupError::FieldNotFound {
                field: spawn.field.clone(),
            });
        };
        registry.spawn(spawn.at, def.clone());
    }
    Ok(registry)
}

/// Whether `situation`'s authored ganger cells contain a duplicate — two gangers
/// spawned on the SAME `(cell, level)`.
///
/// A thin `bool` wrapper over the sibling `first_stacked_cell` detection — `true`
/// iff some `(cell, level)` is authored for more than one ganger. [`setup_battle`](super::setup_battle) ENFORCES
/// this invariant (GTW-457): a duplicate aborts setup with
/// [`BattleSetupError::StackedGangers`] before any entity is spawned, since the
/// GTW-156 occupancy pour is last-write-wins and would otherwise silently overwrite
/// the first ganger's slot while both entities survive stacked on one cell.
#[must_use]
pub fn has_stacked_gangers(situation: &Situation) -> StackedGangers {
    StackedGangers::new(first_stacked_cell(situation).is_some())
}

/// Whether a situation authors two gangers on the SAME `(cell, level)` — the GTW-457
/// stacked-spawn invariant [`setup_battle`](super::setup_battle) enforces.
///
/// A named newtype over `bool` (no-bare-types: a stacked-spawn verdict is a domain
/// value, not a bare boolean). Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackedGangers(bool);

impl StackedGangers {
    /// Build a stacked-gangers verdict from its boolean state.
    #[must_use]
    pub const fn new(stacked: bool) -> Self {
        Self(stacked)
    }
}

/// The first `(cell, level)` two or more authored gangers share, in
/// [`Situation::gangers`](crate::situation::Situation) order — or `None` when every
/// ganger has a distinct spawn cell.
///
/// The single HashSet-over-`at` detection both [`has_stacked_gangers`] (the `bool`
/// view) and the [`setup_battle`](super::setup_battle) pre-spawn gate (GTW-457 — the ENFORCED view that
/// needs the offending cell for [`BattleSetupError::StackedGangers`]) read.
#[must_use]
pub(super) fn first_stacked_cell(situation: &Situation) -> Option<crate::metric::CellLevel> {
    let mut seen = HashSet::new();
    situation
        .gangers
        .iter()
        .find(|g| !seen.insert(g.at))
        .map(|g| g.at)
}
