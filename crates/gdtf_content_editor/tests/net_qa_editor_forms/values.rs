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
