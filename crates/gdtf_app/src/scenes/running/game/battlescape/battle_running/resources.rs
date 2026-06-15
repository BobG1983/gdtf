use bevy::prelude::*;

#[derive(Resource)]
pub(in crate::scenes::running::game::battlescape::battle_running) struct BattleRunningComplete;

/// The finite, deterministic **turn budget** that gates `BattleRunning` completion
/// (E10.6 / GTW-208) — how many `FixedUpdate` ticks remain before the phase ends.
///
/// A named newtype over the remaining tick count (no bare `u32` — `.claude/rules/no-bare-types.md`):
/// a private inner with a derived [`Deref`] so a reader sees the count without the inner
/// field escaping. It is grounded in the turn vocabulary of `docs/combat/combat.md` L34
/// (actions spend from a per-turn pool), alongside the existing
/// [`TurnTu`](gdtf_battle_sim::tuning::TurnTu) precedent — but DISTINCT from `Tu` / `TurnTu`
/// (those are TU *costs*; this counts remaining tactical-phase ticks).
///
/// This is a **placeholder completion gate** for the headless bootstrap: it is a real,
/// bounded, deterministic condition (decremented saturatingly once per tick, completing at
/// zero) that resolves even for the empty/unseeded deep walk — NOT the eventual "the turn
/// loop ended" condition, which E9/GTW-14 owns. It is inserted `OnEnter(BattleRunning)` and
/// removed `OnExit(BattleRunning)` (a per-run state-scoped resource, `bevy-traps.md` #1), so
/// a re-entry starts from a fresh budget.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::scenes::running::game::battlescape::battle_running) struct BattleRunTurnBudget(u32);

impl BattleRunTurnBudget {
    /// Build a turn budget of `ticks` remaining `FixedUpdate` ticks.
    pub(in crate::scenes::running::game::battlescape::battle_running) const fn new(
        ticks: u32,
    ) -> Self {
        Self(ticks)
    }

    /// Spend one tick from the budget, **saturating** at zero (never wraps below zero —
    /// `.claude/rules/no-bare-types.md` saturating arithmetic). Returns the decremented
    /// budget; the caller writes it back.
    pub(in crate::scenes::running::game::battlescape::battle_running) const fn tick(self) -> Self {
        Self(self.0.saturating_sub(1))
    }

    /// Whether the budget is spent (zero ticks remain) — the completion gate's condition.
    pub(in crate::scenes::running::game::battlescape::battle_running) const fn is_spent(
        self,
    ) -> bool {
        self.0 == 0
    }
}
