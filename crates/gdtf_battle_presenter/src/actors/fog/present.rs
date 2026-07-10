//! The [`present_fog`] writer system: the public seam that desaturates the rendered
//! terrain layer (GTW-348) and dims lower drawn storeys (GTW-519) from the sim's
//! [`SquadVisibility`](gdtf_battle_sim::visibility::SquadVisibility).
//!
//! GTW-627: this is the TERRAIN arm only. Actor-sprite visibility is owned by the
//! ganger-visibility resolver
//! ([`resolve_ganger_visibility`](crate::resolve_ganger_visibility)), which composes the
//! same fog fact with the drawn-band storey fact through one pure classifier.

use bevy::prelude::*;
use gdtf_battle_sim::{prelude::CellLevel, visibility::SquadVisibility};

use super::material::Saturation;
use crate::{
    Brightness, ContextDepth, IsolateView, StoreyTreatment, StoreyViewMode, TerrainFogMaterial,
    TerrainSprite, ViewMode,
    actors::quiet::{set_fog_knobs_quiet, set_visibility_quiet},
    storey_treatment,
};

/// The [`TerrainFogMaterial`] saturation for a squad-VISIBLE cell — full colour (the atlas
/// tile's own pixels read through unchanged).
const VISIBLE_SATURATION: Saturation = Saturation::new(1.0);

/// The [`TerrainFogMaterial`] saturation for a squad-EXPLORED cell — full GREYSCALE
/// (GTW-348: EXPLORED renders at the SAME brightness but with its colour removed, so
/// colour-loss is the memory cue, not brightness-loss). `0.0` mixes the tile fully toward
/// its BT.709 luminance in the shader.
const EXPLORED_SATURATION: Saturation = Saturation::new(0.0);

/// The [`Brightness`] a [`StoreyTreatment::ContextBelow`] tile at depth 1 renders at —
/// tier ONE of the (at most two-tier) context ramp (GTW-594 C3).
///
/// This (with [`CONTEXT_TIER_TWO`]) is the GTW-519 below-active brightness knob, now the
/// NAMED `ContextBelow` treatment: a single tunable dim `< 1.0`, so a context storey draws
/// visibly darker than the full-bright active storey (the UFO:EU / `OpenXcom` multi-level
/// darken). It is a SEPARATE axis from the fog
/// [`saturation`](TerrainFogMaterial::saturation) (the EXPLORED colour-loss): a context
/// EXPLORED tile is BOTH greyscaled (saturation) AND dimmed (this brightness), the two
/// composing in the shader (grey-mix, then scale). It is deliberately NOT the DEPRECATED
/// `explored_dim` (the fog EXPLORED cue is colour-loss, never dimmed —
/// `docs/combat/visibility.md`); this dim expresses storey DEPTH, not fog state. `0.55` is a
/// legible-but-clearly-recessed dim, in-engine adjustable in the GTW-388 QA pass.
const CONTEXT_TIER_ONE: Brightness = Brightness::new(0.55);

/// The [`Brightness`] a [`StoreyTreatment::ContextBelow`] tile at depth `>= 2` renders at —
/// tier TWO, where the ramp CLAMPS (GTW-594 C3: any context-depth brightness ramp holds at
/// most TWO tiers; a deeper storey never dims further than this).
///
/// Deliberately EQUAL to [`CONTEXT_TIER_ONE`] today — the ramp is FLAT, so the GTW-594
/// promotion changes NO battlescape pixel (C3's "no visual change day one"); a future tune
/// may split the two tiers, never add a third.
const CONTEXT_TIER_TWO: Brightness = Brightness::new(0.55);

/// The fog treatment a `(cell, level)` resolves to, for a terrain tile the fog modulates.
///
/// A named view-domain decision (no bare tuple / option): the three states map to the
/// three rendered treatments. UNSEEN hides the tile; the other two show it at a
/// [`TerrainFogMaterial`] saturation (VISIBLE full colour, EXPLORED full greyscale).
pub(super) enum CellFog {
    /// Squad-VISIBLE — shown at full colour ([`VISIBLE_SATURATION`]).
    Visible,
    /// Squad-EXPLORED (not VISIBLE) — shown at full-brightness greyscale
    /// ([`EXPLORED_SATURATION`]).
    Explored,
    /// UNSEEN — hidden (the dark clear colour reads through).
    Unseen,
}

impl CellFog {
    /// Resolve the fog treatment for `key` from the squad sets (VISIBLE wins over
    /// EXPLORED; neither is UNSEEN).
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

/// The presenter's storey-treatment TABLE, brightness column (GTW-594 C1/C3): the
/// [`Brightness`] a [`StoreyTreatment::ContextBelow`] tile renders at, by depth.
///
/// The class → pixels mapping for the terrain arm's context treatment: depth `1` is
/// [`CONTEXT_TIER_ONE`], depth `>= 2` CLAMPS to [`CONTEXT_TIER_TWO`] (at most two tiers —
/// C3). Both tiers are the same flat `0.55` today, so this is bit-identical to the
/// pre-GTW-594 single knob (no visual change day one). Only
/// [`StoreyTreatment::Active`] maps to [`Brightness::FULL`] (the A1 law's second half),
/// asserted by the unit test below.
pub(super) fn context_below_brightness(depth: ContextDepth) -> Brightness {
    if *depth <= 1 {
        CONTEXT_TIER_ONE
    } else {
        CONTEXT_TIER_TWO
    }
}

/// `Update` ([`PresenterSystems::Compose`](crate::PresenterSystems) — the fog-composition
/// stage, chained strictly after the `Scene` stage that holds the terrain draw and the
/// destruction swaps, GTW-623): the presenter TERRAIN fog writer — the documented public
/// seam GTW-342 exposes.
///
/// It is the terrain VIEW arm of the squad fog (`docs/combat/visibility.md`): the sim owns
/// the three [`SquadVisibility`] states and recomputes them; this system READS them through
/// the pure seams and MODULATES the already-drawn layer in place — it never owns fog and
/// never repaints from a snapshot (the rendered layer IS the fog mask). Actor sprites are
/// NOT written here: the ganger-visibility resolver
/// ([`resolve_ganger_visibility`](crate::resolve_ganger_visibility), also in `Compose`)
/// owns every [`GangerSprite`](crate::GangerSprite) `Visibility` write.
///
/// For every DRAWN [`TerrainSprite`] (the whole `[0..=active]` band since GTW-519, not just
/// the active storey) it drives the tile's [`TerrainFogMaterial`] over TWO orthogonal axes
/// that COMPOSE in the shader:
///
/// * `saturation` + [`Visibility`] by the marker's fog STATE (GTW-348): VISIBLE → full colour
///   (`saturation` 1.0, shown); EXPLORED → FULL-brightness GREYSCALE (`saturation` 0.0 —
///   colour-loss as the memory cue, shown); UNSEEN → [`Visibility::Hidden`]. This drives ALL
///   drawn storeys, so a lower-storey EXPLORED cell still desaturates and a lower-storey
///   UNSEEN cell still hides.
/// * `brightness` by the tile's storey TREATMENT (GTW-519 → GTW-594: the shared
///   [`storey_treatment`] classifier + this presenter's table): the ONE
///   [`StoreyTreatment::Active`] storey is [`Brightness::FULL`] (full-bright — the A1 law:
///   only Active renders full-bright), a [`StoreyTreatment::ContextBelow`] storey is
///   dimmed through `context_below_brightness` (`< 1.0`, clamped to two tiers). A SEPARATE axis
///   from the fog saturation — they multiply (grey-mix, THEN scale), so a context EXPLORED
///   tile is greyscaled AND dimmed. A [`StoreyTreatment::Hidden`] sprite cannot normally
///   survive to this Compose stage (the Scene-stage redraw despawns the band's complement
///   the same update); one is defensively hidden.
///
/// # Tick-quiet writes (GTW-627 C4)
///
/// Every write goes through the shared seam in `actors/quiet.rs`, the same discipline the
/// ganger resolver uses: [`Visibility`] flips via `set_if_neq` (an already-correct tile is
/// not re-dirtied), and the material knobs are COMPARED through the immutable
/// [`Assets::get`](bevy::asset::Assets::get) first — the change-marking
/// [`Assets::get_mut`](bevy::asset::Assets::get_mut) (which re-uploads the uniform next
/// frame) runs only when a knob actually differs. Re-driving the target values each run
/// (not only on change) keeps a tile consistent after a fog flip / level cycle with no
/// compounding — the material is the single source, never a stacked modulate. An UNSEEN
/// tile is only hidden (its material knobs are left as-is — invisible, so they do not
/// matter — and re-driven the run it becomes shown). Mutation stays IN PLACE — never
/// despawn + respawn (the UI-mutate-not-respawn convention).
///
/// Param-only (`bevy-traps.md` #7): the read [`Res`]ources ([`SquadVisibility`] / the
/// [`ActiveLevel`](crate::ActiveLevel) / the [`ViewMode`] + [`IsolateView`] pair the
/// classifier's mode composes from), the terrain materials store
/// ([`ResMut<Assets<TerrainFogMaterial>>`]), and the [`TerrainSprite`] /
/// [`MeshMaterial2d`] terrain query. It takes no [`Commands`] — every change is an
/// in-place mutate.
pub fn present_fog(
    squad: Res<SquadVisibility>,
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
        // The storey-treatment brightness (GTW-519 → GTW-594): the shared classifier's
        // verdict through this presenter's table — FULL only on the one Active storey,
        // the two-tier context dim on a ContextBelow storey. Orthogonal to the fog
        // saturation below — a context EXPLORED tile ends up greyscaled AND dimmed (the
        // two axes compose in the shader).
        let brightness = match storey_treatment(marker.at.level(), *active, mode) {
            StoreyTreatment::Hidden => {
                // Cannot normally exist (the Scene-stage redraw despawned this storey's
                // sprites this update); hide defensively rather than mis-brighten.
                set_visibility_quiet(&mut visibility, Visibility::Hidden);
                continue;
            }
            StoreyTreatment::Active => Brightness::FULL,
            StoreyTreatment::ContextBelow(depth) => context_below_brightness(depth),
        };
        match CellFog::resolve(&squad, &marker.at) {
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

    /// The A1 law, second half (presenter table): only [`StoreyTreatment::Active`] maps
    /// to [`Brightness::FULL`] — every context depth renders strictly dimmer than full
    /// bright (`present_fog` maps `Active` to `Brightness::FULL` and everything else
    /// through this table).
    ///
    /// [`StoreyTreatment::Active`]: crate::StoreyTreatment::Active
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

    /// GTW-594 C3 — the context ramp CLAMPS at two tiers: depth 2 and every deeper depth
    /// share ONE brightness (tier two); depth 1 is tier one. (Both tiers are deliberately
    /// equal today — the flat pre-GTW-594 knob, no visual change day one.)
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
