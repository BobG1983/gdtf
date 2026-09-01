//! Pre-spawn resolution of authored keys against catalogs.

use bevy::{platform::collections::HashSet, prelude::Deref};

use super::super::terrain_resolve::{
    ResolvedCoverPiece, ResolvedSlabPiece, resolve_cover_def, resolve_slab_def,
    resolve_terrain_or_err,
};
use crate::{
    armor::{ArmorRegistry, ArmorSpec},
    effects::{
        fields::{FieldDefRegistry, FieldRegistry},
        on_death::TerrainOnDeathRegistry,
    },
    equipment::attachments::{AttachmentRegistry, resolve_pending_attachments},
    ganger::{GangMember, GangRegistry},
    situation::{BattleSetupError, PlacedGanger, Situation},
    terrain::{def::TerrainDefRegistry, entity::TerrainIndexKey},
    weapon::{
        FISTS_KEY, MeleeWeaponBundle, MeleeWeaponRegistry, PendingAttachments, WeaponBundle,
        WeaponName, WeaponRegistry, WeaponSpawnSiblings,
    },
};

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
        let pending = resolve_pending_attachments(&spec.slots, &spec.attachments, attachments);
        let (bundle, siblings) = spec.clone().into_bundle(member.weapon.clone());
        weapon_bundles.push((bundle, siblings, pending));
    }
    Ok(weapon_bundles)
}

pub(super) fn resolve_melee_bundles(
    resolved_members: &[(&PlacedGanger, &GangMember)],
    melee_weapons: &MeleeWeaponRegistry,
    attachments: Option<&AttachmentRegistry>,
) -> Result<Vec<(MeleeWeaponBundle, PendingAttachments)>, BattleSetupError> {
    let mut melee_bundles = Vec::with_capacity(resolved_members.len());
    for (_placed, member) in resolved_members {
        let melee_key = member
            .melee_weapon
            .clone()
            .unwrap_or_else(|| WeaponName::new(FISTS_KEY.to_owned()));
        let Some(spec) = melee_weapons.spec(&melee_key) else {
            return Err(BattleSetupError::MeleeWeaponNotFound { weapon: melee_key });
        };
        let pending = resolve_pending_attachments(&spec.slots, &spec.attachments, attachments);
        melee_bundles.push((spec.clone().into_bundle(melee_key), pending));
    }
    Ok(melee_bundles)
}

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

/// The cover pieces a situation spawns, plus the on-death effects their cells carry.
pub(super) struct ResolvedCovers {
    pub(super) pieces:   Vec<ResolvedCoverPiece>,
    pub(super) on_death: TerrainOnDeathRegistry,
}

pub(super) fn resolve_covers(
    situation: &Situation,
    terrain: Option<&TerrainDefRegistry>,
) -> Result<ResolvedCovers, BattleSetupError> {
    let mut pieces: Vec<ResolvedCoverPiece> = Vec::new();
    let mut on_death = TerrainOnDeathRegistry::default();
    for cover in situation.walls.iter().chain(situation.scatter.iter()) {
        let def = resolve_terrain_or_err(terrain, &cover.piece)?;
        if !def.on_death.is_empty() {
            on_death.insert(TerrainIndexKey::Cover(cover.at), def.on_death.clone());
        }
        let Some(resolved) = resolve_cover_def(&cover.piece, def) else {
            return Err(BattleSetupError::TerrainNotFound { piece: cover.piece });
        };
        pieces.push(resolved);
    }
    Ok(ResolvedCovers { pieces, on_death })
}

/// Resolve the slab pieces, registering each def's `on_death` under its slab key.
pub(super) fn resolve_slabs(
    situation: &Situation,
    terrain: Option<&TerrainDefRegistry>,
    on_death: &mut TerrainOnDeathRegistry,
) -> Result<Vec<ResolvedSlabPiece>, BattleSetupError> {
    let mut resolved_slabs: Vec<ResolvedSlabPiece> = Vec::new();
    for slab_spawn in &situation.slabs {
        let def = resolve_terrain_or_err(terrain, &slab_spawn.piece)?;
        if !def.on_death.is_empty() {
            on_death.insert(TerrainIndexKey::Slab(slab_spawn.at), def.on_death.clone());
        }
        let Some(resolved) = resolve_slab_def(&slab_spawn.piece, def) else {
            return Err(BattleSetupError::TerrainNotFound {
                piece: slab_spawn.piece,
            });
        };
        resolved_slabs.push(resolved);
    }
    Ok(resolved_slabs)
}

pub(super) fn build_field_registry(
    situation: &Situation,
    catalog: Option<&FieldDefRegistry>,
) -> Result<FieldRegistry, BattleSetupError> {
    let mut registry = FieldRegistry::new();
    for spawn in &situation.fields {
        let Some(def) = catalog.and_then(|c| c.def(&spawn.field)) else {
            return Err(BattleSetupError::FieldNotFound {
                field: spawn.field.clone(),
            });
        };
        registry.spawn(spawn.at, def.clone());
    }
    Ok(registry)
}

/// Whether any two gangers share a spawn cell.
#[must_use]
pub fn has_stacked_gangers(situation: &Situation) -> StackedGangers {
    StackedGangers::new(first_stacked_cell(situation).is_some())
}

/// Flag: at least one cell has stacked gangers.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackedGangers(bool);

impl StackedGangers {
    /// Wrap the flag.
    #[must_use]
    pub const fn new(stacked: bool) -> Self {
        Self(stacked)
    }
}

#[must_use]
pub(super) fn first_stacked_cell(situation: &Situation) -> Option<crate::metric::CellLevel> {
    let mut seen = HashSet::new();
    situation
        .gangers
        .iter()
        .find(|g| !seen.insert(g.at))
        .map(|g| g.at)
}
