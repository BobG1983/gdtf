//! The fire-mode control — a `gdtf_ui` horizontal [`SegmentedControl`](gdtf_ui::SegmentedControl) (GTW-265 /
//! GTW-277 / GTW-284) — its constructor, its offered-mode visibility driver, its press →
//! `SelectedFireMode` write, and its active-segment sync.
//!
//! GTW-277 migrated the fire-mode control from three ad-hoc toggle buttons to ONE generic
//! `gdtf_ui` [`SegmentedControl`](gdtf_ui::SegmentedControl) (3 horizontal segments Single / Burst / Full-Auto — the
//! Fire-Mode control in the mockup). Selecting a segment sets
//! [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) DIRECTLY to that mode's
//! read-back [`FireModeSpec`](gdtf_battle_sim::FireModeSpec) (never fabricated); the current
//! mode is the control's own [`ActiveSegment`](gdtf_ui::ActiveSegment) highlight, synced
//! FROM [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode). Mode does NOT use
//! [`ActIntent`](gdtf_battle_input::ActIntent)
//! (it writes the resource directly) — that is unchanged from GTW-265.
//!
//! ## Only the offered modes show, via per-segment visibility (GTW-284)
//!
//! The THREE segments are spawned ONCE — a weapon offers a SUBSET of {Single, Burst, Full}
//! — and [`rebuild_mode_segments`] reveals exactly the offered ones by toggling each
//! segment's [`Display`](bevy::ui::Display) (`Display::None` collapses a non-offered
//! segment so the row shrinks to the offered set) via `gdtf_ui`'s
//! [`set_segment_visible`](gdtf_ui::set_segment_visible) — NEVER despawning/respawning the
//! control on a selection/weapon change. So the segment [`Entity`](bevy::prelude::Entity) ids stay STABLE across a
//! weapon change ([[ui-mutate-not-respawn]] / the GTW-284 invariant) and the offered subset
//! is the only thing that visibly changes. An UNARMED selection (no
//! [`FireMode`](gdtf_battle_sim::FireMode)) hides every segment and the panel root.
//!
//! It runs `.after(UiSystems::ApplyTheme)` (`bevy-traps.md` #3) so its writes settle
//! deterministically relative to the theme pass.

mod cost_lines;
mod order;
mod select;
mod spawn;
mod visibility;

pub(in crate::states::running::game::battlescape) use cost_lines::sync_mode_tu_cost_lines;
pub(in crate::states::running::game::battlescape) use select::{
    mode_segment_write, sync_mode_active_segment,
};
pub(in crate::states::running::game::battlescape) use spawn::{
    spawn_mode_panel, tag_mode_segments,
};
pub(in crate::states::running::game::battlescape) use visibility::rebuild_mode_segments;
