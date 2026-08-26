//! One field of a draft on the wire, named by the form it belongs to.

use serde::{Deserialize, Serialize};

use super::{
    draft_name::EditorDraftNameNet,
    value::{FieldDamageNet, FieldDurationNet},
};
use crate::net_qa::wire::{
    armor::{
        ArmorFloorNet, ArmorHardnessNet, ArmorIntegrityNet, ArmorProtectionNet, ArmorTypeNet,
        BodyPartNet,
    },
    attachment::{
        AttachmentEffectNet, AttachmentSlotNet, DamageTypeNet, FatalBiasNet, WeaponDamageNet,
        WeaponPunchNet, WeaponShredNet,
    },
    fire_mode::HitTypeNet,
    gang::{GangAttributeNet, GangAttributeValueNet},
    injury::{InjuryCategoryNet, InjuryEffectNet, InjuryKeyNet, InjurySeverityNet, InjuryTextNet},
    key::EditorKeyNet,
    list::EditorListIndexNet,
    melee_weapon::{HandednessNet, ReachNet, ShoveNet},
    sprite::{SpriteAnimatedNet, SpriteFacingNet, SpriteFpsNet, SpritePxNet, SpriteSourceNet},
    terrain::{
        BlocksPathingNet, FootfallNet, HeightBandNet, LosBlockingNet, MountedWeaponNet,
        TerrainHpNet,
    },
    terrain_kind::TerrainKindNet,
    tile_role::TileRoleNet,
    weapon::{
        AccuracyNet, BaseSpreadNet, DotDamageNet, DotEnabledNet, DotTurnsNet, ExplodeDamageNet,
        FieldKeyNet, KickbackNet, MagazineSizeNet, OnDeathEnabledNet, OnDeathVariantNet,
        ReloadTuNet, StableNet, TrajectoryStyleNet,
    },
};

/// One field of the active mode's draft, carrying the value it is set to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorFieldNet {
    /// The Terrain draft's kind pick.
    TerrainKind(TerrainKindNet),
    /// The Armor draft's name.
    ArmorName(EditorDraftNameNet),
    /// The Sprite draft's name.
    SpriteName(EditorDraftNameNet),
    /// The Attachment draft's name.
    AttachmentName(EditorDraftNameNet),
    /// One armor piece's damage floor.
    ArmorFloor {
        /// Which piece.
        part:  BodyPartNet,
        /// The floor it is set to.
        value: ArmorFloorNet,
    },
    /// One armor piece's protection.
    ArmorProtection {
        /// Which piece.
        part:  BodyPartNet,
        /// The protection it is set to.
        value: ArmorProtectionNet,
    },
    /// One armor piece's hardness.
    ArmorHardness {
        /// Which piece.
        part:  BodyPartNet,
        /// The hardness it is set to.
        value: ArmorHardnessNet,
    },
    /// One armor piece's integrity.
    ArmorIntegrity {
        /// Which piece.
        part:  BodyPartNet,
        /// The integrity it is set to.
        value: ArmorIntegrityNet,
    },
    /// One armor piece's material.
    ArmorType {
        /// Which piece.
        part:  BodyPartNet,
        /// The material it is set to.
        value: ArmorTypeNet,
    },
    /// The Sprite draft's base image source.
    SpriteBaseSource(SpriteSourceNet),
    /// The Sprite draft's anchor, horizontal axis.
    SpriteAnchorX(SpritePxNet),
    /// The Sprite draft's anchor, vertical axis.
    SpriteAnchorY(SpritePxNet),
    /// The Sprite animation's frame rate.
    SpriteFps(SpriteFpsNet),
    /// One facing's source override, set or cleared.
    SpriteFacingOverride {
        /// Which facing.
        facing: SpriteFacingNet,
        /// The source it overrides with, or none to clear it.
        source: Option<SpriteSourceNet>,
    },
    /// One animation frame's source.
    SpriteFrame {
        /// Which frame.
        index:  EditorListIndexNet,
        /// The source it is set to.
        source: SpriteSourceNet,
    },
    /// Whether the Sprite draft is animated.
    SpriteAnimated(SpriteAnimatedNet),
    /// The Attachment draft's display name.
    AttachmentDisplayName(EditorDraftNameNet),
    /// The Attachment draft's mounting slot.
    AttachmentSlot(AttachmentSlotNet),
    /// One authored effect, variant and payload together.
    AttachmentEffect {
        /// Which effect.
        index:  EditorListIndexNet,
        /// The effect it is set to.
        effect: AttachmentEffectNet,
    },
    /// The Terrain draft's display name.
    TerrainDisplayName(EditorDraftNameNet),
    /// The Terrain draft's hit points, written to both the cover and the slab field.
    TerrainHp(TerrainHpNet),
    /// The Terrain draft's armor protection.
    TerrainArmorProtection(ArmorProtectionNet),
    /// The Terrain draft's armor hardness.
    TerrainArmorHardness(ArmorHardnessNet),
    /// The Terrain draft's height band, for the kinds that carry one.
    TerrainHeightBand(HeightBandNet),
    /// The Terrain draft's graphic role.
    TerrainGraphic(TileRoleNet),
    /// The Terrain draft's footfall sound, on a Slab.
    TerrainFootfall(FootfallNet),
    /// The Terrain draft's mounted weapon, on an Emplacement, set or cleared.
    TerrainMountedWeapon(Option<MountedWeaponNet>),
    /// The Terrain draft's pathing override, set or cleared.
    TerrainBlocksPathing(Option<BlocksPathingNet>),
    /// The Terrain draft's line-of-sight override, set or cleared.
    TerrainBlocksLos(Option<LosBlockingNet>),
    /// The Injury draft's key.
    InjuryKey(InjuryKeyNet),
    /// The Injury draft's display name.
    InjuryName(EditorDraftNameNet),
    /// The Injury draft's category.
    InjuryCategory(InjuryCategoryNet),
    /// The Injury draft's severity rank.
    InjurySeverity(InjurySeverityNet),
    /// The Injury draft's popup text.
    InjuryPopupText(InjuryTextNet),
    /// The Injury draft's log text.
    InjuryLogText(InjuryTextNet),
    /// The Injury draft's inspect text.
    InjuryInspectText(InjuryTextNet),
    /// One authored injury effect, variant and payload together.
    InjuryEffect {
        /// Which effect.
        index:  EditorListIndexNet,
        /// The effect it is set to.
        effect: InjuryEffectNet,
    },
    /// The Melee Weapon draft's display name.
    MeleeWeaponName(EditorDraftNameNet),
    /// The Melee Weapon draft's damage.
    MeleeWeaponDamage(WeaponDamageNet),
    /// The Melee Weapon draft's punch.
    MeleeWeaponPunch(WeaponPunchNet),
    /// The Melee Weapon draft's shred.
    MeleeWeaponShred(WeaponShredNet),
    /// The Melee Weapon draft's damage channel.
    MeleeWeaponDamageType(DamageTypeNet),
    /// The Melee Weapon draft's fatal bias.
    MeleeWeaponFatalBias(FatalBiasNet),
    /// The Melee Weapon draft's handedness.
    MeleeWeaponHandedness(HandednessNet),
    /// The Melee Weapon draft's reach, which the form clamps at one.
    MeleeWeaponReach(ReachNet),
    /// Whether the Melee Weapon draft shoves on connect.
    MeleeWeaponShove(ShoveNet),
    /// The Gang draft's own name.
    GangName(EditorDraftNameNet),
    /// One member's display name.
    GangMemberName {
        /// Which member.
        index: EditorListIndexNet,
        /// The name it is set to.
        name:  EditorDraftNameNet,
    },
    /// One member's attribute.
    GangMemberAttribute {
        /// Which member.
        index:     EditorListIndexNet,
        /// Which of the member's eight attributes.
        attribute: GangAttributeNet,
        /// The value it is set to.
        value:     GangAttributeValueNet,
    },
    /// One member's primary weapon key.
    GangMemberWeapon {
        /// Which member.
        index: EditorListIndexNet,
        /// The weapon registry key it is set to.
        key:   EditorKeyNet,
    },
    /// One member's armor key.
    GangMemberArmor {
        /// Which member.
        index: EditorListIndexNet,
        /// The armor registry key it is set to.
        key:   EditorKeyNet,
    },
    /// One member's melee weapon key, set or cleared to the fists default.
    GangMemberMeleeWeapon {
        /// Which member.
        index: EditorListIndexNet,
        /// The melee registry key it is set to, or none for the fists default.
        key:   Option<EditorKeyNet>,
    },
    /// The Weapon draft's display name.
    WeaponName(EditorDraftNameNet),
    /// The Weapon draft's base cone spread.
    WeaponBaseSpread(BaseSpreadNet),
    /// The Weapon draft's accuracy.
    WeaponAccuracy(AccuracyNet),
    /// The Weapon draft's kickback.
    WeaponKickback(KickbackNet),
    /// The Weapon draft's damage.
    WeaponDamage(WeaponDamageNet),
    /// The Weapon draft's punch.
    WeaponPunch(WeaponPunchNet),
    /// The Weapon draft's shred.
    WeaponShred(WeaponShredNet),
    /// The Weapon draft's damage channel.
    WeaponDamageType(DamageTypeNet),
    /// The Weapon draft's fatal bias.
    WeaponFatalBias(FatalBiasNet),
    /// The Weapon draft's handedness.
    WeaponHandedness(HandednessNet),
    /// The Weapon draft's shot trajectory.
    WeaponTrajectory(TrajectoryStyleNet),
    /// Whether the Weapon draft is braced by design.
    WeaponStable(StableNet),
    /// Whether the Weapon draft shoves on connect.
    WeaponShove(ShoveNet),
    /// The Weapon draft's magazine size.
    WeaponMagazineSize(MagazineSizeNet),
    /// The Weapon draft's reload cost in time units.
    WeaponMagazineReloadTu(ReloadTuNet),
    /// Whether the Weapon draft authors a damage-over-time profile.
    WeaponDot(DotEnabledNet),
    /// The DOT profile's per-turn damage, behind the DOT tick box.
    WeaponDotDamage(DotDamageNet),
    /// The DOT profile's duration, behind the DOT tick box, where the form turns zero into one.
    WeaponDotTurns(DotTurnsNet),
    /// The DOT profile's damage channel, behind the DOT tick box.
    WeaponDotDamageType(DamageTypeNet),
    /// Whether the Weapon draft authors an on-death effect.
    WeaponOnDeath(OnDeathEnabledNet),
    /// Which on-death effect the draft is on, behind the on-death tick box.
    WeaponOnDeathVariant(OnDeathVariantNet),
    /// The explode effect's hit geometry, behind the on-death tick box.
    WeaponOnDeathHitType(HitTypeNet),
    /// The explode effect's blast damage, behind the on-death tick box.
    WeaponOnDeathDamage(ExplodeDamageNet),
    /// The explode effect's damage channel, behind the on-death tick box.
    WeaponOnDeathDamageType(DamageTypeNet),
    /// The leave-field effect's field key, behind the on-death tick box.
    WeaponOnDeathField(FieldKeyNet),
    /// The Field draft's save stem.
    FieldName(EditorDraftNameNet),
    /// The Field draft's drain per tick.
    FieldDamage(FieldDamageNet),
    /// The Field draft's damage channel.
    FieldDamageType(DamageTypeNet),
    /// The Field draft's lifetime, where a zero turn count is refused rather than clamped.
    FieldDuration(FieldDurationNet),
}
