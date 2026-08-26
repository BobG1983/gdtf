use serde::Deserialize;

/// A client's own reading of one armor piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum BodyPartRow {
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

/// A client's own reading of a piece's material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ArmorTypeRow {
    Plated,
    Refractive,
    Flak,
    Void,
    Hazard,
    Reinforced,
    Ceramic,
}

/// A client's own reading of an attachment's mounting slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum SlotRow {
    Muzzle,
    Sight,
    Rail,
    Magazine,
    Counterweight,
    Pommel,
}

/// A client's own reading of a sprite facing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum FacingRow {
    North,
    East,
    South,
    West,
}

/// A client's own reading of a sheet rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct RectRow {
    pub(crate) w: u32,
    pub(crate) h: u32,
}

/// A client's own reading of where a sprite's pixels come from.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum SourceRow {
    File(String),
    Sheet { sheet: String, rect: RectRow },
}

/// A client's own reading of a fire mode, all five fields the form's row edits.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(crate) struct FireModeRow {
    pub(crate) kind:       ModeKindRow,
    pub(crate) cone_mult:  f32,
    pub(crate) tu_percent: f32,
    pub(crate) shots:      u16,
    pub(crate) hit_type:   HitTypeRow,
}

/// A client's own reading of a fire-mode selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ModeKindRow {
    Single,
    Burst,
    Full,
}

/// A client's own reading of a fire mode's hit geometry.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(crate) enum HitTypeRow {
    Single,
    Blast { radius: u8 },
    Cone { range: u8, angle: f32 },
    Line { range: u8 },
}

/// A client's own reading of one attachment effect, for the arms this suite drives.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(crate) enum EffectRow {
    Aim(f32),
    Stability(f32),
    GainFireMode(FireModeRow),
    Silence,
}

/// A client's own reading of the Terrain draft's kind pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum TerrainKindRow {
    Wall,
    Cover,
    Slab,
    Emplacement,
}

/// A client's own reading of one Terrain tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum TagRow {
    Openable,
    BlocksVision,
    BlocksPathfinding,
    Indestructible,
}

/// A client's own reading of a piece's height band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum BandRow {
    Low,
    Mid,
    High,
}

/// A client's own reading of a slab's footfall sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum FootfallRow {
    None,
    Metal,
    Grate,
}

/// A client's own reading of a line-of-sight override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum LosRow {
    Full,
    UpToHeightBand,
    None,
}

/// A client's own reading of a graphic role, for the arms this suite drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum TileRoleRow {
    Floor,
    Wall,
    Cover,
    Slab,
    Rubble,
}

/// A client's own reading of an injury's category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum CategoryRow {
    Head,
    Torso,
    Arm,
    Leg,
}

/// A client's own reading of an injury's severity rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum SeverityRow {
    Minor,
    Major,
    Critical,
}

/// A client's own reading of the stat a Modify effect names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum StatRow {
    Speed,
    Aim,
    Strength,
    Toughness,
    Reflexes,
    Cool,
    Grit,
    Luck,
    Shooting,
    Fight,
    Reactions,
    Morale,
    Tu,
    Hp,
    Wounds,
    Bottle,
}

/// A client's own reading of one injury effect.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(crate) enum InjuryEffectRow {
    Modify { stat: StatRow, amount: i8 },
    Bleeding { amount: u8 },
    DisableHand,
    MovementCostMul(f32),
}

/// A client's own reading of a melee weapon's damage channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum DamageTypeRow {
    Shock,
    Blast,
    Chem,
    Kinetic,
    Plasma,
    Rend,
    Las,
}

/// A client's own reading of how many hands a weapon takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum HandednessRow {
    OneHanded,
    TwoHanded,
}

/// A client's own reading of one fight mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct FightModeRow {
    pub(crate) kind:    FightModeKindRow,
    pub(crate) tu_cost: u16,
    pub(crate) strikes: u16,
}

/// A client's own reading of a fight mode's swing or thrust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum FightModeKindRow {
    Swing,
    Thrust,
}

/// A client's own reading of which attribute a member write named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum GangAttributeRow {
    Speed,
    Aim,
    Strength,
    Toughness,
    Reflexes,
    Cool,
    Grit,
    Luck,
}

/// A client's own reading of one slot declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct SlotDeclRow {
    pub(crate) slot:     SlotRow,
    pub(crate) capacity: u8,
}
