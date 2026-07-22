//! Ammo-state + shared firing-guard proofs: the magazine clamp / saturating
//! spend / burst clamp, the `can_fire` guard cases (AC1–AC8), and the GTW-775
//! ammo-type compatibility gate (`compat`).

mod support;

mod ammo;
mod compat;
mod guard;
mod hands;
