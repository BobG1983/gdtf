//! The `*Requested` buffered [`Message`](bevy::prelude::Message) types — the message-driven **input
//! contract** for the landed combat acts (E10.2 / GTW-204; the seventh,
//! [`MoveRequested`], added in GTW-234; the eighth, [`ReloadRequested`], in GTW-275; the
//! ninth, the fieldless [`EndTurnRequested`] turn signal, in GTW-309).
//!
//! Eight [`#[derive(Message)]`](bevy::prelude::Message) buffered messages — mirroring
//! [`crate::bleed::Bleeding`] / [`crate::occupancy_sync::CoverDestroyed`], the buffered
//! `Message` API, NOT the observer `Event` API (`bevy-traps.md` #4). Each carries the
//! act's [`Entity`](bevy::prelude::Entity) actor ref(s) plus the act's OWNED payload. A `Message` cannot hold a
//! borrow, so [`FireRequested`] carries an OWNED [`FireModeSpec`](crate::weapon::FireModeSpec) (now `Copy` again,
//! GTW-260) plus the target [`Cell`](crate::metric::Cell) / [`Level`](crate::metric::Level) — the type has **no lifetime
//! parameter**; the [`dispatch_fire`](super::fire::dispatch_fire) system reconstructs the
//! borrow-based [`FireOrder`](crate::fire::FireOrder) `{ mode: &owned_spec, target_cell,
//! target_level }` from the owned payload at the call site.
//!
//! These carry [`Entity`](bevy::prelude::Entity) actor refs (matching the landed `fire(shooter: Entity)` and
//! the downed verbs' actor/target entities) — NOT presenter-facing integer ids. A
//! `*Resolved` integer-id boundary is a LATER epic; E10 relies on component
//! change-detection for the view, so this slice ships no `*Resolved` types.

mod downed;
mod emplacement;
mod fire;
mod melee;
mod movement;
mod open_door;
mod posture;
mod reload;
mod shove;
mod throw;
mod turn;

pub use downed::{ExecuteDownedRequested, StabilizeDownedRequested};
pub use emplacement::{EnterEmplacementRequested, ExitEmplacementRequested};
pub use fire::FireRequested;
pub use melee::{MeleeRequested, MeleeResolved, MeleeStruck, MeleeTarget};
pub use movement::MoveRequested;
pub use open_door::OpenDoorRequested;
pub use posture::{AimRequest, SetAimingRequested, SetFacingRequested, SetStanceRequested};
pub use reload::ReloadRequested;
pub use shove::{ShoveRequested, ShoveSource};
pub use throw::{ThrowGrenadeRequested, ThrowResolved};
pub use turn::EndTurnRequested;
