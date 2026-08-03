use super::stance::stance_for_cover_band;
use crate::{cover::HeightBand, ganger::StanceKind};

#[test]
fn stance_for_cover_band_maps_each_band() {
    assert_eq!(stance_for_cover_band(HeightBand::Low), StanceKind::Prone);
    assert_eq!(
        stance_for_cover_band(HeightBand::Mid),
        StanceKind::Crouching
    );
    assert_eq!(
        stance_for_cover_band(HeightBand::High),
        StanceKind::Crouching
    );
}
