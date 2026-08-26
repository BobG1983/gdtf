use serde::Deserialize;

use crate::values::{
    ArmorTypeRow, BandRow, BodyPartRow, CategoryRow, DamageTypeRow, DurationRow, EffectRow,
    FacingRow, FightModeRow, FireModeRow, FootfallRow, GangAttributeRow, HandednessRow, HitTypeRow,
    InjuryEffectRow, LosRow, OnDeathVariantRow, SeverityRow, SlotDeclRow, SlotRow, SourceRow,
    TagRow, TerrainKindRow, TileRoleRow, TrajectoryRow,
};

/// Which single-value field a write named, carrying the value read back off the draft.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum FieldRow {
    ArmorName(String),
    SpriteName(String),
    AttachmentName(String),
    ArmorFloor {
        part:  BodyPartRow,
        value: i32,
    },
    ArmorProtection {
        part:  BodyPartRow,
        value: i32,
    },
    ArmorHardness {
        part:  BodyPartRow,
        value: i32,
    },
    ArmorIntegrity {
        part:  BodyPartRow,
        value: i32,
    },
    ArmorType {
        part:  BodyPartRow,
        value: ArmorTypeRow,
    },
    SpriteBaseSource(SourceRow),
    SpriteAnchorX(u32),
    SpriteAnchorY(u32),
    SpriteFps(f32),
    SpriteFacingOverride {
        facing: FacingRow,
        source: Option<SourceRow>,
    },
    SpriteFrame {
        index:  usize,
        source: SourceRow,
    },
    SpriteAnimated(bool),
    AttachmentDisplayName(String),
    AttachmentSlot(SlotRow),
    AttachmentEffect {
        index:  usize,
        effect: EffectRow,
    },
    TerrainKind(TerrainKindRow),
    TerrainDisplayName(String),
    TerrainHp(u32),
    TerrainArmorProtection(i32),
    TerrainArmorHardness(i32),
    TerrainHeightBand(BandRow),
    TerrainGraphic(TileRoleRow),
    TerrainFootfall(FootfallRow),
    TerrainMountedWeapon(Option<String>),
    TerrainBlocksPathing(Option<bool>),
    TerrainBlocksLos(Option<LosRow>),
    InjuryKey(String),
    InjuryName(String),
    InjuryCategory(CategoryRow),
    InjurySeverity(SeverityRow),
    InjuryPopupText(String),
    InjuryLogText(String),
    InjuryInspectText(String),
    InjuryEffect {
        index:  usize,
        effect: InjuryEffectRow,
    },
    MeleeWeaponName(String),
    MeleeWeaponDamage(i32),
    MeleeWeaponPunch(i32),
    MeleeWeaponShred(i32),
    MeleeWeaponDamageType(DamageTypeRow),
    MeleeWeaponFatalBias(f32),
    MeleeWeaponHandedness(HandednessRow),
    MeleeWeaponReach(u16),
    MeleeWeaponShove(bool),
    GangName(String),
    GangMemberName {
        index: usize,
        name:  String,
    },
    GangMemberAttribute {
        index:     usize,
        attribute: GangAttributeRow,
        value:     f32,
    },
    GangMemberWeapon {
        index: usize,
        key:   String,
    },
    GangMemberArmor {
        index: usize,
        key:   String,
    },
    GangMemberMeleeWeapon {
        index: usize,
        key:   Option<String>,
    },
    WeaponName(String),
    WeaponBaseSpread(f32),
    WeaponAccuracy(f32),
    WeaponKickback(f32),
    WeaponDamage(i32),
    WeaponPunch(i32),
    WeaponShred(i32),
    WeaponDamageType(DamageTypeRow),
    WeaponFatalBias(f32),
    WeaponHandedness(HandednessRow),
    WeaponTrajectory(TrajectoryRow),
    WeaponStable(bool),
    WeaponShove(bool),
    WeaponMagazineSize(u16),
    WeaponMagazineReloadTu(u8),
    WeaponDot(bool),
    WeaponDotDamage(u16),
    WeaponDotTurns(u8),
    WeaponDotDamageType(DamageTypeRow),
    WeaponOnDeath(bool),
    WeaponOnDeathVariant(OnDeathVariantRow),
    WeaponOnDeathHitType(HitTypeRow),
    WeaponOnDeathDamage(u16),
    WeaponOnDeathDamageType(DamageTypeRow),
    WeaponOnDeathField(String),
    FieldName(String),
    FieldDamage(u16),
    FieldDamageType(DamageTypeRow),
    FieldDuration(DurationRow),
}

/// Which list a write named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ListRow {
    EntrySides,
    TerrainTags,
    AttachmentEffects,
    SpriteFrames,
    InjuryEffects,
    MeleeWeaponFightModes,
    MeleeWeaponSlots,
    MeleeWeaponAttachments,
    GangMembers,
    WeaponFireModes,
    WeaponSlots,
    WeaponAttachments,
    FieldImmuneArmorTypes,
}

/// One member of the list a reply reads back.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum ListMemberRow {
    EntrySide(FacingRow),
    TerrainTag(TagRow),
    AttachmentEffect(EffectRow),
    SpriteFrame(SourceRow),
    InjuryEffect(InjuryEffectRow),
    FightMode(FightModeRow),
    Slot(SlotDeclRow),
    Attachment(String),
    GangMember(String),
    FireMode(FireModeRow),
    ImmuneArmorType(ArmorTypeRow),
}

/// `editor.set_field`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct SetFieldReplyRow {
    pub(crate) field: FieldRow,
}

/// `editor.list_op`'s reply body.
#[derive(Debug, Deserialize)]
pub(crate) struct ListOpReplyRow {
    pub(crate) list:    ListRow,
    pub(crate) members: Vec<ListMemberRow>,
}
