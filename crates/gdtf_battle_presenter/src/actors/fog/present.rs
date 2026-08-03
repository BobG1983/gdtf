//! Apply fog saturation, brightness, and visibility to terrain tiles.

use bevy::prelude::*;
use gdtf_battle_sim::{prelude::CellLevel, visibility::SquadVisibility};

use super::material::Saturation;
use crate::{
    Brightness, ContextDepth, IsolateView, ShownSquadVisibility, StoreyTreatment, StoreyViewMode,
    TerrainFogMaterial, TerrainSprite, ViewMode,
    actors::quiet::{set_fog_knobs_quiet, set_visibility_quiet},
    storey_treatment,
};

const VISIBLE_SATURATION: Saturation = Saturation::new(1.0);

const EXPLORED_SATURATION: Saturation = Saturation::new(0.0);

const CONTEXT_TIER_ONE: Brightness = Brightness::new(0.55);

const CONTEXT_TIER_TWO: Brightness = Brightness::new(0.55);

pub(super) enum CellFog {
    Visible,
    Explored,
    Unseen,
}

impl CellFog {
    pub(super) fn resolve(squad: &SquadVisibility, key: &CellLevel) -> Self {
        if *squad.is_cell_visible(key) {
            Self::Visible
        } else if *squad.is_cell_explored(key) {
            Self::Explored
        } else {
            Self::Unseen
        }
    }
}

pub(super) fn context_below_brightness(depth: ContextDepth) -> Brightness {
    if *depth <= 1 {
        CONTEXT_TIER_ONE
    } else {
        CONTEXT_TIER_TWO
    }
}

/// Drive terrain fog materials and visibility from the shown squad map.
pub fn present_fog(
    squad: Res<ShownSquadVisibility>,
    active: Res<crate::ActiveLevel>,
    view: Res<ViewMode>,
    isolate: Res<IsolateView>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut terrain: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Visibility,
    )>,
) {
    let mode = StoreyViewMode::new(*view, *isolate);
    for (marker, mat_handle, mut visibility) in &mut terrain {
        let brightness = match storey_treatment(marker.at.level(), *active, mode) {
            StoreyTreatment::Hidden => {
                set_visibility_quiet(&mut visibility, Visibility::Hidden);
                continue;
            }
            StoreyTreatment::Active => Brightness::FULL,
            StoreyTreatment::ContextBelow(depth) => context_below_brightness(depth),
        };
        match CellFog::resolve(squad.visibility(), &marker.at) {
            CellFog::Visible => {
                set_fog_knobs_quiet(
                    &mut materials,
                    mat_handle.id(),
                    VISIBLE_SATURATION,
                    brightness,
                );
                set_visibility_quiet(&mut visibility, Visibility::Inherited);
            }
            CellFog::Explored => {
                set_fog_knobs_quiet(
                    &mut materials,
                    mat_handle.id(),
                    EXPLORED_SATURATION,
                    brightness,
                );
                set_visibility_quiet(&mut visibility, Visibility::Inherited);
            }
            CellFog::Unseen => {
                set_visibility_quiet(&mut visibility, Visibility::Hidden);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::metric::MAX_LEVELS;

    use super::{CONTEXT_TIER_ONE, CONTEXT_TIER_TWO, context_below_brightness};
    use crate::{Brightness, ContextDepth};

    #[test]
    fn only_the_active_treatment_is_full_bright() {
        for depth in 1..=MAX_LEVELS {
            let brightness = context_below_brightness(ContextDepth::new(depth));
            assert!(
                *brightness < *Brightness::FULL,
                "a ContextBelow tile at depth {depth} must render dimmer than the one \
                 full-bright Active storey (A1: only Active is full-bright); got {}",
                *brightness,
            );
        }
    }

    #[test]
    fn context_ramp_clamps_at_two_tiers() {
        let tier_two = context_below_brightness(ContextDepth::new(2));
        for depth in 2..=MAX_LEVELS {
            assert_eq!(
                context_below_brightness(ContextDepth::new(depth)),
                tier_two,
                "depth {depth} must clamp to tier two (a <=2-tier ramp — GTW-594 C3)",
            );
        }
        assert_eq!(
            context_below_brightness(ContextDepth::new(1)),
            CONTEXT_TIER_ONE,
            "depth 1 is tier one",
        );
        assert_eq!(
            CONTEXT_TIER_ONE, CONTEXT_TIER_TWO,
            "day one the ramp is FLAT (both tiers equal) — the GTW-594 promotion changes \
             no battlescape pixel (C3)",
        );
    }
}
