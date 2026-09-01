//! The Terrain form's own field arms, written through the draft its field stack writes.

use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    weapon::{WeaponName, WeaponRegistry},
};
use gdtf_qa_protocol::command::RefusalNote;

use super::terrain_on_death;
use crate::{
    net_qa::{
        commands::write::form_fault::{FormWriteFault, NOT_AN_EMPLACEMENT},
        wire::{
            ArmorHardnessNet, ArmorProtectionNet, BlocksPathingNet, EditorDraftNameNet,
            FootfallNet, HeightBandNet, LosBlockingNet, MountedWeaponNet, TerrainFieldNet,
            TerrainHpNet, TerrainKindNet, TileRoleNet,
        },
    },
    terrain_form::{TerrainDraft, TerrainKindChoice, offered_graphic_roles},
};

const NO_HEIGHT_BAND: RefusalNote = RefusalNote::from_static(
    "the Terrain form draws the height band only for a kind whose has_height_band is true, so \
     this write names a control the open draft does not have",
);

const NO_FOOTFALL: RefusalNote = RefusalNote::from_static(
    "the Terrain form draws the footfall pick only for a kind whose offers_footfall is true, and \
     the draft's own setter silently does nothing otherwise",
);

const NO_WEAPON_REGISTRY: RefusalNote = RefusalNote::from_static(
    "the mounted-weapon pick reads the weapon registry, and it is not in the world, so the form \
     itself offers no name to choose",
);

// The value clamped the way the HP input's own range clamps a drag.
fn clamped_hp(hp: TerrainHpNet) -> TerrainHpNet {
    TerrainHpNet::new((*hp).clamp(
        *TerrainDraft::HP_RANGE.start(),
        *TerrainDraft::HP_RANGE.end(),
    ))
}

// The protection clamped the way the armor input's own range clamps a drag.
fn clamped_protection(value: ArmorProtectionNet) -> ArmorProtectionNet {
    ArmorProtectionNet::from_protection(ArmorProtection::new((*value).clamp(
        *TerrainDraft::ARMOR_RANGE.start(),
        *TerrainDraft::ARMOR_RANGE.end(),
    )))
}

// The hardness clamped the way the armor input's own range clamps a drag.
fn clamped_hardness(value: ArmorHardnessNet) -> ArmorHardnessNet {
    ArmorHardnessNet::from_hardness(ArmorHardness::new((*value).clamp(
        *TerrainDraft::ARMOR_RANGE.start(),
        *TerrainDraft::ARMOR_RANGE.end(),
    )))
}

// The name the picker holds for this key, or the fault a key it does not hold answers.
fn known_weapon(
    registry: &WeaponRegistry,
    named: &MountedWeaponNet,
) -> Result<WeaponName, FormWriteFault> {
    let wanted = named.to_name();
    if registry.spec(&wanted).is_some() {
        Ok(wanted)
    } else {
        Err(FormWriteFault::bad(format!(
            "`{}` is not a weapon the registry holds, so the mounted-weapon pick offers no such \
             row",
            **named
        )))
    }
}

fn write_hp(draft: &mut TerrainDraft, hp: TerrainHpNet) -> TerrainFieldNet {
    let clamped = clamped_hp(hp);
    draft.set_cover_hp(clamped.to_cover_hp());
    draft.set_slab_hp(clamped.to_slab_hp());
    TerrainFieldNet::Hp(TerrainHpNet::new(*draft.cover_hp()))
}

const fn write_height_band(
    draft: &mut TerrainDraft,
    band: HeightBandNet,
) -> Result<TerrainFieldNet, FormWriteFault> {
    if !draft.kind().has_height_band() {
        return Err(FormWriteFault::Gated(NO_HEIGHT_BAND));
    }
    draft.set_height_band(band.to_band());
    Ok(TerrainFieldNet::HeightBand(HeightBandNet::from_band(
        draft.height_band(),
    )))
}

fn write_graphic(
    draft: &mut TerrainDraft,
    role: TileRoleNet,
) -> Result<TerrainFieldNet, FormWriteFault> {
    let wanted = role.to_role();
    if !offered_graphic_roles().contains(&wanted) {
        return Err(FormWriteFault::bad(format!(
            "{wanted:?} is not a role the Terrain form's graphic picker offers"
        )));
    }
    draft.set_graphic(wanted);
    let stored = TileRoleNet::from_role(draft.graphic()).unwrap_or(role);
    Ok(TerrainFieldNet::Graphic(stored))
}

const fn write_footfall(
    draft: &mut TerrainDraft,
    footfall: FootfallNet,
) -> Result<TerrainFieldNet, FormWriteFault> {
    if !draft.kind().offers_footfall() {
        return Err(FormWriteFault::Gated(NO_FOOTFALL));
    }
    draft.set_footfall(footfall.to_choice());
    Ok(TerrainFieldNet::Footfall(FootfallNet::from_choice(
        draft.footfall(),
    )))
}

fn write_mounted_weapon(
    draft: &mut TerrainDraft,
    weapons: Option<&WeaponRegistry>,
    named: Option<MountedWeaponNet>,
) -> Result<TerrainFieldNet, FormWriteFault> {
    if draft.kind() != TerrainKindChoice::Emplacement {
        return Err(FormWriteFault::Gated(NOT_AN_EMPLACEMENT));
    }
    let Some(registry) = weapons else {
        return Err(FormWriteFault::MissingModel(NO_WEAPON_REGISTRY));
    };
    let wanted = match named.as_ref() {
        Some(named) => Some(known_weapon(registry, named)?),
        None => None,
    };
    draft.set_mounted_weapon(wanted);
    Ok(TerrainFieldNet::MountedWeapon(
        draft.mounted_weapon().map(MountedWeaponNet::from_name),
    ))
}

/// Write one Terrain field, answering the field as the draft stores it.
pub(super) fn write(
    draft: &mut TerrainDraft,
    weapons: Option<&WeaponRegistry>,
    field: TerrainFieldNet,
) -> Result<TerrainFieldNet, FormWriteFault> {
    match field {
        TerrainFieldNet::Kind(kind) => {
            draft.set_kind(kind.to_choice());
            Ok(TerrainFieldNet::Kind(TerrainKindNet::from_choice(
                draft.kind(),
            )))
        }
        TerrainFieldNet::DisplayName(name) => {
            draft.set_display_name((*name).clone());
            Ok(TerrainFieldNet::DisplayName(EditorDraftNameNet::new(
                draft.display_name(),
            )))
        }
        TerrainFieldNet::Hp(hp) => Ok(write_hp(draft, hp)),
        TerrainFieldNet::ArmorProtection(value) => {
            draft.set_armor_protection(clamped_protection(value).to_protection());
            Ok(TerrainFieldNet::ArmorProtection(
                ArmorProtectionNet::from_protection(draft.armor_protection()),
            ))
        }
        TerrainFieldNet::ArmorHardness(value) => {
            draft.set_armor_hardness(clamped_hardness(value).to_hardness());
            Ok(TerrainFieldNet::ArmorHardness(
                ArmorHardnessNet::from_hardness(draft.armor_hardness()),
            ))
        }
        TerrainFieldNet::HeightBand(band) => write_height_band(draft, band),
        TerrainFieldNet::Graphic(role) => write_graphic(draft, role),
        TerrainFieldNet::Footfall(footfall) => write_footfall(draft, footfall),
        TerrainFieldNet::MountedWeapon(named) => write_mounted_weapon(draft, weapons, named),
        TerrainFieldNet::BlocksPathing(blocks) => {
            draft.set_blocks_pathing(blocks.map(|blocks| *blocks));
            Ok(TerrainFieldNet::BlocksPathing(
                draft.blocks_pathing().map(BlocksPathingNet::new),
            ))
        }
        TerrainFieldNet::BlocksLos(blocking) => {
            draft.set_blocks_los(blocking.map(LosBlockingNet::to_blocking));
            Ok(TerrainFieldNet::BlocksLos(
                draft.blocks_los().map(LosBlockingNet::from_blocking),
            ))
        }
        TerrainFieldNet::OnDeathVariant { index, variant } => {
            terrain_on_death::variant(draft, index, variant)
        }
        TerrainFieldNet::OnDeathHitType { index, hit_type } => {
            terrain_on_death::hit_type(draft, index, hit_type)
        }
        TerrainFieldNet::OnDeathDamage { index, damage } => {
            terrain_on_death::damage(draft, index, damage)
        }
        TerrainFieldNet::OnDeathDamageType { index, damage_type } => {
            terrain_on_death::damage_type(draft, index, damage_type)
        }
        TerrainFieldNet::OnDeathField { index, field } => {
            terrain_on_death::field(draft, index, field)
        }
    }
}
