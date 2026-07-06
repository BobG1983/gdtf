//! Unit tests for the DEV battle-script drive triggers: the `FireAtFrame` /
//! `FireModeOverride` / `FallAtFrame` parse gates plus the headless drives of the
//! REAL fire / fall paths (the trigger systems only write real sim messages /
//! component writes, so they ARE headless-testable — the systems are driven
//! directly on minimal apps, minus the unrelated `BattleScapeState` gate). Moved
//! with their systems out of `capture/test` (GTW-632); coverage rides along
//! unmodified.

mod fall;
mod fire;
