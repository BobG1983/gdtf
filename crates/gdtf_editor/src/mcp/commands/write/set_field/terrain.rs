//! The Terrain form's own field arms, written through the draft its field stack writes.

use bevy::asset::uuid::Uuid;
use cobalt_mcp_protocol::command::RefusalNote;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    terrain::{
        def::{LeavesBehind, TerrainDefRegistry, TerrainUuid, owed_views_for},
        piece::TerrainGraphicKey,
    },
    weapon::{WeaponName, WeaponRegistry},
};
use gdtf_content_families::sprites::{SpriteDefRegistry, SpriteName};

use super::terrain_on_death;
use crate::{
    mcp::{
        commands::write::form_fault::{FormWriteFault, NOT_AN_EMPLACEMENT},
        wire::{
            ArmorHardnessNet, ArmorProtectionNet, BlocksPathingNet, EditorDraftNameNet,
            FootfallNet, HeightBandNet, LeavesBehindNet, LosBlockingNet, MountedWeaponNet,
            TerrainFieldNet, TerrainHpNet, TerrainKindNet, TerrainViewNet, TerrainViewSpriteNet,
        },
    },
    terrain_form::{TerrainDraft, TerrainKindChoice},
};

/// The registries the leaves-behind control reads its two choice lists from.
pub(super) struct TerrainWriteRegistries<'a> {
    /// Every weapon the mounted-weapon pick offers.
    pub(super) weapons: Option<&'a WeaponRegistry>,
    /// Every def the leaves-behind piece pick offers.
    pub(super) terrain: Option<&'a TerrainDefRegistry>,
    /// Every sprite the leaves-behind sprite pick offers.
    pub(super) sprites: Option<&'a SpriteDefRegistry>,
}

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

const NO_TERRAIN_REGISTRY: RefusalNote = RefusalNote::from_static(
    "the leaves-behind piece pick reads the terrain registry, and it is not in the world, so the \
     form itself offers no def to choose",
);

const NO_SPRITE_REGISTRY: RefusalNote = RefusalNote::from_static(
    "the leaves-behind sprite pick reads the sprite registry, and it is not in the world, so the \
     form itself offers no sprite to choose",
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

// The note a view the open draft's kind and tags never owe is refused with.
fn view_not_owed(view: TerrainViewNet) -> RefusalNote {
    RefusalNote::from_owned(format!(
        "the Terrain form draws art only for the views the draft's own kind and tags owe, and \
         {:?} is not one of them, so this write names a control the open draft does not have",
        view.to_view(),
    ))
}

// The view is written only when the draft's own kind and tags owe it.
fn write_view(
    draft: &mut TerrainDraft,
    view: TerrainViewNet,
    sprite: TerrainViewSpriteNet,
) -> Result<TerrainFieldNet, FormWriteFault> {
    let wanted = view.to_view();
    if !owed_views_for(draft.kind().piece_kind(), draft.tags()).contains(&wanted) {
        return Err(FormWriteFault::Gated(view_not_owed(view)));
    }
    draft.set_view(wanted, sprite.to_key());
    let stored = draft
        .views()
        .sprite(wanted)
        .map_or(sprite, TerrainViewSpriteNet::from_key);
    Ok(TerrainFieldNet::View {
        view:   TerrainViewNet::from_view(wanted),
        sprite: stored,
    })
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

// The def key the piece pick holds, or the fault a key it does not hold answers.
fn known_piece(
    registry: &TerrainDefRegistry,
    named: &crate::mcp::wire::TerrainKeyNet,
) -> Result<TerrainUuid, FormWriteFault> {
    let Ok(parsed) = Uuid::parse_str(named) else {
        return Err(FormWriteFault::bad(format!(
            "`{}` is not the hyphenated UUID text a terrain key is written as",
            **named
        )));
    };
    let wanted = TerrainUuid::new(parsed);
    if registry.def(&wanted).is_some() {
        Ok(wanted)
    } else {
        Err(FormWriteFault::bad(format!(
            "`{}` is not a def the terrain registry holds, so the leaves-behind pick offers no \
             such row",
            **named
        )))
    }
}

// The sprite name the sprite pick holds, or the fault a name it does not hold answers.
fn known_sprite(
    registry: &SpriteDefRegistry,
    named: &crate::mcp::wire::SpriteKeyNet,
) -> Result<TerrainGraphicKey, FormWriteFault> {
    if registry.contains(&SpriteName::new((**named).clone())) {
        Ok(TerrainGraphicKey::new((**named).clone()))
    } else {
        Err(FormWriteFault::bad(format!(
            "`{}` is not a sprite the registry holds, so the leaves-behind pick offers no such \
             row",
            **named
        )))
    }
}

fn write_leaves_behind(
    draft: &mut TerrainDraft,
    registries: &TerrainWriteRegistries<'_>,
    leaves: LeavesBehindNet,
) -> Result<TerrainFieldNet, FormWriteFault> {
    let wanted = match leaves {
        LeavesBehindNet::Nothing => LeavesBehind::Nothing,
        LeavesBehindNet::Piece(named) => {
            let Some(registry) = registries.terrain else {
                return Err(FormWriteFault::MissingModel(NO_TERRAIN_REGISTRY));
            };
            LeavesBehind::Piece(known_piece(registry, &named)?)
        }
        LeavesBehindNet::Sprite(named) => {
            let Some(registry) = registries.sprites else {
                return Err(FormWriteFault::MissingModel(NO_SPRITE_REGISTRY));
            };
            LeavesBehind::Sprite(known_sprite(registry, &named)?)
        }
    };
    draft.set_leaves_behind(wanted);
    Ok(TerrainFieldNet::LeavesBehind(
        LeavesBehindNet::from_leaves_behind(draft.leaves_behind()),
    ))
}

/// Write one Terrain field, answering the field as the draft stores it.
pub(super) fn write(
    draft: &mut TerrainDraft,
    registries: &TerrainWriteRegistries<'_>,
    field: TerrainFieldNet,
) -> Result<TerrainFieldNet, FormWriteFault> {
    let weapons = registries.weapons;
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
        TerrainFieldNet::LeavesBehind(leaves) => write_leaves_behind(draft, registries, leaves),
        TerrainFieldNet::View { view, sprite } => write_view(draft, view, sprite),
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
