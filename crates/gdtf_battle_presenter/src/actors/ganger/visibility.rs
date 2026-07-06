//! The drawn-storey-band visibility policy: the shared band predicate and the
//! on-change band re-apply.

use bevy::prelude::*;
use gdtf_battle_sim::Position;

use super::sprite_map::{GangerSprite, GangerSprites};
use crate::{ActiveLevel, ViewMode};

/// Whether a ganger at `pos` is DRAWN — i.e. its storey lies within the drawn band under the
/// current [`ViewMode`] (GTW-520 C4, widened for the GTW-521 view toggle).
///
/// Consults the ONE shared band predicate
/// [`ActiveLevel::draws_storey`](crate::ActiveLevel::draws_storey), the successor to the
/// pre-GTW-520 on-active-storey hard cut: a ganger on ANY storey within the drawn band is
/// drawn (it peeks through floor-gaps on the lower storeys GTW-519 already renders terrain
/// for), and one strictly ABOVE the band ceiling is culled. Every ganger-visibility
/// site ([`spawn_ganger_sprites`](super::spawn_move::spawn_ganger_sprites) /
/// [`move_ganger_sprites`](super::spawn_move::move_ganger_sprites) /
/// [`apply_active_level_filter`]
/// AND the fog writer's `present_actor_fog`) routes through this SAME predicate so they
/// cannot drift.
///
/// The [`ViewMode`] chooses the band CEILING (GTW-521 C2): [`ViewMode::DownToActive`] caps at
/// the active level (unchanged GTW-520); [`ViewMode::FullView`] draws every storey. It reads
/// the typed [`Level`](gdtf_battle_sim::Level) via the canonical
/// [`CellLevel::level`](gdtf_battle_sim::CellLevel::level) accessor (GTW-565 — [`Position`]
/// Derefs to [`CellLevel`](gdtf_battle_sim::CellLevel); the accessor owns the one storey
/// clamp, a no-op on every real key) and asks the [`ActiveLevel`] whether that storey is
/// drawn under `view`. The move / filter systems have the [`ActiveLevel`] + [`ViewMode`] as
/// [`Res`]; the actor fog arm holds the dereferenced values and rebuilds one via
/// [`ActiveLevel::new`] — both reach the same predicate.
pub(super) fn ganger_in_drawn_band(pos: &Position, active: ActiveLevel, view: ViewMode) -> bool {
    active.draws_storey(pos.level(), view)
}

/// `Update` (`PresenterSystems::Scene`, runs only on an [`ActiveLevel`] OR [`ViewMode`]
/// change): show the ganger sprites within the new drawn storey band, hide the rest.
///
/// On an [`ActiveLevel`](crate::ActiveLevel) change (`ActiveLevel::is_changed`) OR a
/// [`ViewMode`](crate::ViewMode) change (`ViewMode::is_changed`, GTW-521 — the full-view
/// toggle widens/narrows the band ceiling exactly as a level cycle moves it) it walks every
/// live ganger and sets its mapped presenter sprite's [`Visibility`] by whether the ganger's
/// `Position` lies WITHIN the new drawn band ([`ganger_in_drawn_band`], the shared
/// [`ActiveLevel::draws_storey`](crate::ActiveLevel::draws_storey) predicate under the current
/// [`ViewMode`] — GTW-520 C4, the SAME band the S4/S5 terrain + spawn/move sites and the fog
/// writer consult). Above-band sprites are HIDDEN (not despawned — the move / reframe systems
/// keep them current), in-band sprites (in [`ViewMode::FullView`] every storey) are SHOWN. It
/// is gated to only run when a triggering resource changed so it does no per-frame work.
///
/// Param-only (`bevy-traps.md` #7): [`Res<GangerSprites>`], [`Res<ActiveLevel>`],
/// [`Res<ViewMode>`], the ganger [`Position`] query, and the presenter-sprite [`Visibility`]
/// query.
pub fn apply_active_level_filter(
    sprites: Res<GangerSprites>,
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    gangers: Query<(Entity, &Position)>,
    mut presenters: Query<&mut Visibility, With<GangerSprite>>,
) {
    // GTW-521: re-apply the band filter on EITHER an active-level cycle OR a view-mode toggle —
    // both change which storeys are drawn, so a stale filter would leave upper-storey gangers
    // wrongly hidden (or lower ones wrongly shown) after a FullView flip.
    if !active.is_changed() && !view.is_changed() {
        return;
    }
    for (entity, pos) in &gangers {
        let Some(presenter) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok(mut visibility) = presenters.get_mut(presenter) else {
            continue;
        };
        // GTW-520 C4 / GTW-521 C2: show a ganger anywhere in the new DRAWN band, hide only
        // one strictly above the band ceiling — the shared band predicate under the current
        // ViewMode, so this on-change re-apply agrees with spawn / move (and the fog writer)
        // exactly.
        *visibility = if ganger_in_drawn_band(pos, *active, *view) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
