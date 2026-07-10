//! The **fire** dispatch — the GTW-242 firing-arc + turn-to-fire gate over a buffered
//! [`FireRequested`](crate::acts::request::FireRequested), then the landed
//! [`fire`](crate::fire::fire) verb (E10.2 AC3 / GTW-242).
//!
//! No act logic is reimplemented here: the shot resolution REUSES [`fire`](crate::fire::fire)
//! verbatim; this slice only GATES it (the firing arc + the affordable turn-into-arc) and
//! front-loads the turn. The two queries that both touch `Facing`/`Tu` are time-multiplexed
//! through a [`ParamSet`](bevy::ecs::system::ParamSet) (`bevy-traps.md` #3 / #7 — no
//! `&mut World`).

mod arc;
mod dispatch;
mod emit;
mod params;
mod signals;

pub use arc::{CanEngage, FireArcDecision, can_engage, decide_fire_arc};
pub use dispatch::dispatch_fire;
pub use params::{BattleGridsParam, WeaponProbes};
pub use signals::FireDeclaration;
