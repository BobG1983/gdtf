//! `BattleSimPlugin` drives the render-free authoritative sim: battle RNG streams,
//! builds the battle from the authored `Situation` via `setup_battle`, and gates
//! Generation's state advance on real setup completing.
mod generation_gate;
mod harness;
mod seed_logging;
mod setup;
