use serde::Deserialize;

use crate::values::{
    ArmorTypeRow, BandRow, BodyPartRow, CategoryRow, DamageTypeRow, DurationRow, EffectRow,
    FacingRow, FightModeRow, FireModeRow, FootfallRow, GangAttributeRow, HandednessRow, HitTypeRow,
    InjuryEffectRow, LosRow, OnDeathVariantRow, SeverityRow, SlotDeclRow, SlotRow, SourceRow,
    TagRow, TerrainKindRow, TileRoleRow, TrajectoryRow,
};

/// Which single-value field a write named, under the form that owns it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum FieldRow {
    Terrain(TerrainFieldRow),
    Armor(ArmorFieldRow),
    Sprite(SpriteFieldRow),
    Attachment(AttachmentFieldRow),
    Injury(InjuryFieldRow),
    MeleeWeapon(MeleeWeaponFieldRow),
    Gang(GangFieldRow),
    Weapon(WeaponFieldRow),
    Field(FieldFormFieldRow),
    Theme(ThemeFieldRow),
}

/// A Terrain field a write named, with the value read back off the draft.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum TerrainFieldRow {
    Kind(TerrainKindRow),
    DisplayName(String),
    Hp(u32),
    ArmorProtection(i32),
    ArmorHardness(i32),
    HeightBand(BandRow),
    Graphic(TileRoleRow),
    Footfall(FootfallRow),
    MountedWeapon(Option<String>),
    BlocksPathing(Option<bool>),
    BlocksLos(Option<LosRow>),
    OnDeathVariant {
        index:   usize,
        variant: OnDeathVariantRow,
    },
    OnDeathHitType {
        index:    usize,
        hit_type: HitTypeRow,
    },
    OnDeathDamage {
        index:  usize,
        damage: u16,
    },
    OnDeathDamageType {
        index:       usize,
        damage_type: DamageTypeRow,
    },
    OnDeathField {
        index: usize,
        field: String,
    },
}

/// An Armor field a write named, with the value read back off the draft.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum ArmorFieldRow {
    Name(String),
    Floor {
        part:  BodyPartRow,
        value: i32,
    },
    Protection {
        part:  BodyPartRow,
        value: i32,
    },
    Hardness {
        part:  BodyPartRow,
        value: i32,
    },
    Integrity {
        part:  BodyPartRow,
        value: i32,
    },
    Type {
        part:  BodyPartRow,
        value: ArmorTypeRow,
    },
}

/// A Sprite field a write named, with the value read back off the draft.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum SpriteFieldRow {
    Name(String),
    BaseSource(SourceRow),
    AnchorX(u32),
    AnchorY(u32),
    Fps(f32),
    FacingOverride {
        facing: FacingRow,
        source: Option<SourceRow>,
    },
    Frame {
        index:  usize,
        source: SourceRow,
    },
    Animated(bool),
}

/// An Attachment field a write named, with the value read back off the draft.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum AttachmentFieldRow {
    Name(String),
    DisplayName(String),
    Slot(SlotRow),
    Effect { index: usize, effect: EffectRow },
}

/// An Injury field a write named, with the value read back off the draft.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum InjuryFieldRow {
    Key(String),
    Name(String),
    Category(CategoryRow),
    Severity(SeverityRow),
    PopupText(String),
    LogText(String),
    InspectText(String),
    Effect {
        index:  usize,
        effect: InjuryEffectRow,
    },
}

/// A Melee Weapon field a write named, with the value read back off the draft.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum MeleeWeaponFieldRow {
    Name(String),
    Damage(i32),
    Punch(i32),
    Shred(i32),
    DamageType(DamageTypeRow),
    FatalBias(f32),
    Handedness(HandednessRow),
    Reach(u16),
    Shove(bool),
}

/// A Gang field a write named, with the value read back off the draft.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum GangFieldRow {
    Name(String),
    MemberName {
        index: usize,
        name:  String,
    },
    MemberAttribute {
        index:     usize,
        attribute: GangAttributeRow,
        value:     f32,
    },
    MemberWeapon {
        index: usize,
        key:   String,
    },
    MemberArmor {
        index: usize,
        key:   String,
    },
    MemberMeleeWeapon {
        index: usize,
        key:   Option<String>,
    },
}

/// A Weapon field a write named, with the value read back off the draft.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum WeaponFieldRow {
    Name(String),
    BaseSpread(f32),
    Accuracy(f32),
    Kickback(f32),
    Damage(i32),
    Punch(i32),
    Shred(i32),
    DamageType(DamageTypeRow),
    FatalBias(f32),
    Handedness(HandednessRow),
    Trajectory(TrajectoryRow),
    Stable(bool),
    Shove(bool),
    MagazineSize(u16),
    MagazineReloadTu(u8),
    Dot(bool),
    DotDamage(u16),
    DotTurns(u8),
    DotDamageType(DamageTypeRow),
    OnDeathVariant {
        index:   usize,
        variant: OnDeathVariantRow,
    },
    OnDeathHitType {
        index:    usize,
        hit_type: HitTypeRow,
    },
    OnDeathDamage {
        index:  usize,
        damage: u16,
    },
    OnDeathDamageType {
        index:       usize,
        damage_type: DamageTypeRow,
    },
    OnDeathField {
        index: usize,
        field: String,
    },
}

/// A Field field a write named, with the value read back off the draft.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum FieldFormFieldRow {
    Name(String),
    Damage(u16),
    DamageType(DamageTypeRow),
    Duration(DurationRow),
}

/// A Theme field a write named, with the value read back off the draft.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum ThemeFieldRow {
    Name(String),
}

/// Which list a write named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ListRow {
    EntrySides,
    TerrainTags,
    TerrainOnDeathEffects,
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
    WeaponOnDeathEffects,
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
    OnDeathEffect(OnDeathEffectRow),
}

/// A client's own reading of one authored on-death effect.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum OnDeathEffectRow {
    Explode {
        hit_type:    HitTypeRow,
        damage:      u16,
        damage_type: DamageTypeRow,
    },
    LeaveField {
        field: String,
    },
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
