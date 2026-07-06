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
    Brightness, TerrainFogMaterial, TerrainSprite,
    actors::quiet::{set_fog_knobs_quiet, set_visibility_quiet},
};

/// The [`TerrainFogMaterial`] saturation for a squad-VISIBLE cell — full colour (the atlas
/// tile's own pixels read through unchanged).
const VISIBLE_SATURATION: Saturation = Saturation::new(1.0);

/// The [`TerrainFogMaterial`] saturation for a squad-EXPLORED cell — full GREYSCALE
/// (GTW-348: EXPLORED renders at the SAME brightness but with its colour removed, so
/// colour-loss is the memory cue, not brightness-loss). `0.0` mixes the tile fully toward
/// its BT.709 luminance in the shader.
const EXPLORED_SATURATION: Saturation = Saturation::new(0.0);

/// The [`Brightness`] a terrain tile on a LOWER drawn storey renders at (GTW-519 — the
/// UFO:EU / `OpenXcom` multi-level darken): a single tunable dim `< 1.0`, so a storey BELOW
/// the active view level draws visibly darker than the full-bright active storey.
///
/// The ONE knob for the storey-depth darken axis. It is a SEPARATE axis from the fog
/// [`saturation`](TerrainFogMaterial::saturation) (the EXPLORED colour-loss): a lower
/// EXPLORED tile is BOTH greyscaled (saturation) AND dimmed (this brightness), the two
/// composing in the shader (grey-mix, then scale). It is deliberately NOT the DEPRECATED
/// `explored_dim` (the fog EXPLORED cue is colour-loss, never dimmed —
/// `docs/combat/visibility.md`); this dim expresses storey DEPTH, not fog state. `0.55` is a
/// legible-but-clearly-recessed dim, in-engine adjustable in the GTW-388 QA pass.
const LOWER_STOREY_BRIGHTNESS: Brightness = Brightness::new(0.55);

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
        if squad.is_cell_visible(key) {
            Self::Visible
        } else if squad.is_cell_explored(key) {
            Self::Explored
        } else {
            Self::Unseen
        }
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
/// * `brightness` by the tile's storey DEPTH (GTW-519 — the UFO:EU multi-level darken): the
///   ACTIVE view storey is [`Brightness::FULL`] (full-bright), a LOWER drawn storey is
///   [`LOWER_STOREY_BRIGHTNESS`] (`< 1.0`, dimmed). A SEPARATE axis from the fog saturation —
///   they multiply (grey-mix, THEN scale), so a lower EXPLORED tile is greyscaled AND dimmed.
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
/// [`ActiveLevel`](crate::ActiveLevel)), the terrain materials store
/// ([`ResMut<Assets<TerrainFogMaterial>>`]), and the [`TerrainSprite`] /
/// [`MeshMaterial2d`] terrain query. It takes no [`Commands`] — every change is an
/// in-place mutate.
pub fn present_fog(
    squad: Res<SquadVisibility>,
    active: Res<crate::ActiveLevel>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut terrain: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Visibility,
    )>,
) {
    for (marker, mat_handle, mut visibility) in &mut terrain {
        // The storey-depth brightness (GTW-519): FULL on the active view storey, dimmed on a
        // lower drawn storey. Orthogonal to the fog saturation below — a lower EXPLORED tile
        // ends up greyscaled AND dimmed (the two axes compose in the shader).
        let brightness = if marker.at.z == i32::from(***active) {
            Brightness::FULL
        } else {
            LOWER_STOREY_BRIGHTNESS
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
