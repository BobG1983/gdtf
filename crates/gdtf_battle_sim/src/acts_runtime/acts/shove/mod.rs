//! The **shove** act (GTW-525, child GTW-39 of the falls epic) — a context-sensitive
//! pure-displacement knock-back that pushes an adjacent opposing target one cell directly
//! AWAY from the attacker, letting the resulting FALL (if any) do the harm through the SHARED
//! GTW-523 fall path.
//!
//! Three trigger paths share ONE displacement resolution (`docs/combat/resolution.md` §7 /
//! §Falls; the GTW-525 settled design):
//!
//! - **The deliberate SHOVE act** (any ganger — NOT weapon-gated): the input seam's
//!   `ActIntent::Shove` writes a [`ShoveRequested`](crate::acts::request::ShoveRequested)
//!   `{ shover, target, Deliberate }`; [`dispatch_shove`] gates 8-adjacency + opposing + alive
//!   and spends the [`ShoveTu`](crate::tuning::ShoveTu) leaf.
//! - **The `shove` WEAPON-TAG auto-shove** (any weapon, MELEE or RANGED): a CONNECTING attack
//!   whose weapon carries the [`Shove`](crate::weapon::Shove) tag writes a
//!   [`ShoveRequested`](crate::acts::request::ShoveRequested) `{ shover, target, Weapon }`
//!   internally (the connect already gated + charged, so no re-gate / no TU) — the melee
//!   connect ([`resolve_ganger_melee`](crate::acts::melee)) and the ranged connect
//!   ([`dispatch_fire`](crate::acts::dispatch_fire)) both hook it.
//! - **The shared displacement verb** ([`resolve_shove`]): pure geometry — one cell away from
//!   the attacker, resolving supported=>move / unsupported=>fall (via the SHARED
//!   [`resolve_drop`](crate::falls::resolve_drop)) / blocked=>no-op. NO RNG (the shove is not a
//!   contested roll); only the fall draws RNG, through the shared GTW-523 fork.
//!
//! Render-free, deterministic, unit-testable with injected seeded RNG. Code-health: a
//! directory module, `mod.rs` wiring-only + focused submodules by concern:
//!
//! - [`verb`] — the pure [`resolve_shove`] displacement verb + its [`ShoveOutcome`] (C1).
//! - [`apply`] — the shared [`apply_shove`](apply::apply_shove) helper: rewrites the target's
//!   [`Position`](crate::ganger::Position) and, on a fall, routes the drop through the shared
//!   GTW-523 fall-damage fork + emits [`FallOccurred`](crate::falls::FallOccurred) (C1).
//! - [`dispatch`] — the [`dispatch_shove`] system: drains
//!   [`ShoveRequested`](crate::acts::request::ShoveRequested) and resolves BOTH sources through
//!   the shared verb + apply helper (C2 / C3 / C6).

mod apply;
mod dispatch;
mod verb;

pub use dispatch::dispatch_shove;
pub use verb::{ShoveOutcome, resolve_shove};
