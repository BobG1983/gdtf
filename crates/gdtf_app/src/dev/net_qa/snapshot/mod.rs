//! The battle-state snapshot view service — the GTW-694 architecture's T5 (GTW-738).
//!
//! An ON-DEMAND query service (NOT a change-driven presenter, NOT a continuously-updated
//! cache) that answers the routed
//! [`GetBattleState`](gdtf_qa_protocol::envelope::QaRequest::GetBattleState) requests the T3
//! router queues, reading `gdtf_battle_sim`'s public `Query`/`Res` surface AFTER
//! [`SimSystems::Simulate`](gdtf_battle_sim::occupancy_sync::SimSystems::Simulate) and
//! projecting the live post-Simulate state into the bevy-free wire
//! [`BattleView`](gdtf_qa_protocol::view::BattleView).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`read`] — the read-only [`GangerRow`](read::GangerRow) `QueryData` + the
//!   [`SnapshotWorld`](read::SnapshotWorld) `SystemParam` (the world surface).
//! - [`map`] — the pure sim → wire value/enum mappers (coordinate, faction, facing,
//!   stance, life, severity, body part).
//! - [`ganger`] — the per-ganger card projection (living gangers, weapon, injuries).
//! - [`panel`] — the focus-navigable HUD button token handout (GTW-789, the OBSERVE side
//!   of GTW-782's focus nav).
//! - [`build`] — the [`build_snapshots`](build::build_snapshots) service + the top-level
//!   [`BattleView`](gdtf_qa_protocol::view::BattleView) / terrain / fog / selection / turn
//!   assembly.

mod build;
mod ganger;
mod map;
mod panel;
mod read;

pub(in crate::dev::net_qa) use build::build_snapshots;
