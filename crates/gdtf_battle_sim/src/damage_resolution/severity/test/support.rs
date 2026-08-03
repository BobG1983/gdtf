use super::super::{SeverityInputs, part_severity_mod};
use crate::{
    armor::BodyPart,
    ganger::{Luck, Toughness},
    resolve_hit::PenetratingDamage,
    rng::{BattleSeed, SeverityRng},
    weapon::FatalBias,
};

pub(super) const SEED: u64 = 0xC0FF_EE15;

pub(super) fn rng() -> SeverityRng {
    SeverityRng::from_root(BattleSeed::new(SEED))
}

pub(super) fn inputs(
    pen: i32,
    toughness: f32,
    part: BodyPart,
    luck_shooter: f32,
    luck_defender: f32,
) -> SeverityInputs {
    SeverityInputs::new(
        PenetratingDamage::new(pen),
        Toughness::new(toughness),
        part_severity_mod(part),
        FatalBias::new(0.0),
        Luck::new(luck_shooter),
        Luck::new(luck_defender),
    )
}
