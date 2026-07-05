//! Repaints the inspect panel from the EFFECTIVE
//! [`InspectTarget`](gdtf_battle_input::InspectTarget) — pinned-else-hovered (GTW-274 / GTW-300).
//!
//! [`update_inspect_panel`] reads the effective inspect cell + the
//! [`OccupancyGrid`](gdtf_battle_sim::OccupancyGrid) (occupant / terrain) +
//! [`CoverLedger`](gdtf_battle_sim::CoverLedger) and drives the panel:
//!
//! - a hovered GANGER → show the shared stat block (the host) for it (with its NAME line
//!   color tinted by the ganger's `Faction` — enemy red-ish, player the normal theme),
//!   hide the object block;
//! - a hovered non-floor OBJECT (wall / cover) → show the OBJECT stat block (title + a labeled
//!   Integrity bar + labeled Hardness / Protection / Height-band lines, GTW-295), hide the
//!   ganger stat block;
//! - bare floor / nothing → hide the whole panel.
//!
//! The panel ROOT is hidden by `Visibility`; the two SUB-BLOCKS by `Node.display` (`None`
//! removes a hidden block from layout, so the panel sizes to the visible block only — GTW-295
//! fixes the cover-hover balloon). Every change is a mutate of the existing widgets
//! ([[ui-mutate-not-respawn]]); the system never writes the sim. It runs in `Update` gated
//! `run_if(resource_exists::<BattleInProgress>)` (`bevy-traps.md` #1), `.after(InputSystems::Gather)`
//! so it observes the same update's hover pick.

mod fog;
mod object_block;
mod panel;
mod params;

pub(in crate::states::running::game::battlescape) use panel::update_inspect_panel;
