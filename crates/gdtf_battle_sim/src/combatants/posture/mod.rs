//! The **posture / orientation verbs** — the E4.1 mutators a ganger spends a turn on:
//! [`set_aiming`] toggles the aim flag, [`set_stance`] changes posture, and
//! [`set_facing`] turns in place. Each verb is **pure** — it takes and mutates the
//! landed ganger components ([`Aiming`](crate::ganger::Aiming) /
//! [`Stance`](crate::ganger::Stance) / [`Facing`](crate::ganger::Facing)) plus the
//! ganger's [`Tu`](crate::ganger::Tu) pool, reads the relevant tuning leaf, and charges
//! through the E4.0 [`crate::tu::spend_tu`] — there is **no
//! [`World`](bevy::ecs::world::World) access**, so the verbs unit-test against bare
//! component values with no ECS plumbing.
//!
//! Whether each verb charges TU is **grounded in the docs**, not invented:
//!
//! - [`set_aiming`] charges **no TU** — `docs/combat/combat.md` L34's action list
//!   (step, turn, snap/aimed/auto shot, kneel) does NOT name toggling the aim FLAG as a
//!   costed action; the aim cost is the **fire-time ×1.5 shot-cost premium**
//!   (`docs/combat/resolution.md` §1a, the [`crate::tuning::AimTuPremium`] leaf),
//!   charged when the shot fires (E4.2/E4.5), not per toggle. So it is a pure flag
//!   setter.
//! - [`set_stance`] charges [`crate::tuning::StanceChangeTu`] — combat.md L34 lists
//!   "kneel" among the actions that cost TUs and resolution.md §"What's tunable" names
//!   "stance-change TU".
//! - [`set_facing`] charges [`crate::tuning::TurnTu`] **per 45deg step** — combat.md L34
//!   affirmatively lists "turn" among the costed actions; the per-step magnitude is a
//!   value-agnostic tuning leaf (USER DECISION: 1 TU/step). It is a **PARTIAL** turn: the
//!   ganger turns as many whole 45deg steps as its [`Tu`](crate::ganger::Tu) pool affords
//!   and lands partway when it runs out (0 affordable steps = no turn, no charge). Turning
//!   is **not** a free toggle.
//!
//! Both costed verbs charge **only when the value actually changes** — re-asserting a
//! stance / facing a ganger already holds is a no-op (you don't pay to not move). The
//! charge is **saturating** (it floors at `0`, never underflows) because it goes
//! through [`crate::tu::spend_tu`]; [`set_facing`]'s per-step charge is also EXACT by
//! construction (it never exceeds the pool). The COST magnitudes are tuning
//! (`tuning.ron`); this slice ships the mechanism.

#[cfg(test)]
mod test;
mod verbs;

pub use verbs::{set_aiming, set_facing, set_stance};
