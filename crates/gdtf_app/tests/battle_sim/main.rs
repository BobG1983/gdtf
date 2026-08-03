//! GTW-207 (E10.5): `BattleSimPlugin` drives the render-free authoritative sim
//! battle RNG streams (GTW-14), builds the battle from the authored `Situation` via the
//! authoritative `setup_battle`, gates Generation's state advance on REAL setup
mod generation_gate;
mod harness;
mod seed_logging;
mod setup;
