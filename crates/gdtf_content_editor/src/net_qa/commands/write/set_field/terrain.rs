//! The Terrain form's own field arms, written through the draft its field stack writes.

use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    weapon::{WeaponName, WeaponRegistry},
};
use gdtf_qa_protocol::command::RefusalNote;

use crate::{
    net_qa::{
        commands::write::form_fault::{FormWriteFault, NOT_AN_EMPLACEMENT},
        wire::{
            ArmorHardnessNet, ArmorProtectionNet, BlocksPathingNet, EditorDraftNameNet,
            EditorFieldNet, FootfallNet, HeightBandNet, LosBlockingNet, MountedWeaponNet,
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

fn write_hp(draft: &mut TerrainDraft, hp: TerrainHpNet) -> EditorFieldNet {
    let clamped = clamped_hp(hp);
    draft.set_cover_hp(clamped.to_cover_hp());
    draft.set_slab_hp(clamped.to_slab_hp());
    EditorFieldNet::TerrainHp(TerrainHpNet::new(*draft.cover_hp()))
}

const fn write_height_band(
    draft: &mut TerrainDraft,
    band: HeightBandNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    if !draft.kind().has_height_band() {
        return Err(FormWriteFault::Gated(NO_HEIGHT_BAND));
    }
    draft.set_height_band(band.to_band());
    Ok(EditorFieldNet::TerrainHeightBand(HeightBandNet::from_band(
        draft.height_band(),
    )))
}

fn write_graphic(
    draft: &mut TerrainDraft,
    role: TileRoleNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    let wanted = role.to_role();
    if !offered_graphic_roles().contains(&wanted) {
        return Err(FormWriteFault::bad(format!(
            "{wanted:?} is not a role the Terrain form's graphic picker offers"
        )));
    }
    draft.set_graphic(wanted);
    let stored = TileRoleNet::from_role(draft.graphic()).unwrap_or(role);
    Ok(EditorFieldNet::TerrainGraphic(stored))
}

const fn write_footfall(
    draft: &mut TerrainDraft,
    footfall: FootfallNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    if !draft.kind().offers_footfall() {
        return Err(FormWriteFault::Gated(NO_FOOTFALL));
    }
    draft.set_footfall(footfall.to_choice());
    Ok(EditorFieldNet::TerrainFootfall(FootfallNet::from_choice(
        draft.footfall(),
    )))
}

fn write_mounted_weapon(
    draft: &mut TerrainDraft,
    weapons: Option<&WeaponRegistry>,
    named: Option<MountedWeaponNet>,
) -> Result<EditorFieldNet, FormWriteFault> {
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
    Ok(EditorFieldNet::TerrainMountedWeapon(
        draft.mounted_weapon().map(MountedWeaponNet::from_name),
    ))
}

/// Write one Terrain field, answering the field as the draft stores it.
pub(super) fn write(
    draft: &mut TerrainDraft,
    weapons: Option<&WeaponRegistry>,
    field: EditorFieldNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    match field {
        EditorFieldNet::TerrainKind(kind) => {
            draft.set_kind(kind.to_choice());
            Ok(EditorFieldNet::TerrainKind(TerrainKindNet::from_choice(
                draft.kind(),
            )))
        }
        EditorFieldNet::TerrainDisplayName(name) => {
            draft.set_display_name((*name).clone());
            Ok(EditorFieldNet::TerrainDisplayName(EditorDraftNameNet::new(
                draft.display_name(),
            )))
        }
        EditorFieldNet::TerrainHp(hp) => Ok(write_hp(draft, hp)),
        EditorFieldNet::TerrainArmorProtection(value) => {
            draft.set_armor_protection(clamped_protection(value).to_protection());
            Ok(EditorFieldNet::TerrainArmorProtection(
                ArmorProtectionNet::from_protection(draft.armor_protection()),
            ))
        }
        EditorFieldNet::TerrainArmorHardness(value) => {
            draft.set_armor_hardness(clamped_hardness(value).to_hardness());
            Ok(EditorFieldNet::TerrainArmorHardness(
                ArmorHardnessNet::from_hardness(draft.armor_hardness()),
            ))
        }
        EditorFieldNet::TerrainHeightBand(band) => write_height_band(draft, band),
        EditorFieldNet::TerrainGraphic(role) => write_graphic(draft, role),
        EditorFieldNet::TerrainFootfall(footfall) => write_footfall(draft, footfall),
        EditorFieldNet::TerrainMountedWeapon(named) => write_mounted_weapon(draft, weapons, named),
        EditorFieldNet::TerrainBlocksPathing(blocks) => {
            draft.set_blocks_pathing(blocks.map(|blocks| *blocks));
            Ok(EditorFieldNet::TerrainBlocksPathing(
                draft.blocks_pathing().map(BlocksPathingNet::new),
            ))
        }
        EditorFieldNet::TerrainBlocksLos(blocking) => {
            draft.set_blocks_los(blocking.map(LosBlockingNet::to_blocking));
            Ok(EditorFieldNet::TerrainBlocksLos(
                draft.blocks_los().map(LosBlockingNet::from_blocking),
            ))
        }
        _ => Err(FormWriteFault::ForeignArm),
    }
}
