mod arc;
mod dispatch;
mod emit;
mod params;
mod signals;

pub use arc::{CanEngage, FireArcDecision, can_engage, decide_fire_arc};
pub use dispatch::dispatch_fire;
pub use params::{BattleGridsParam, WeaponProbes};
pub use signals::{FireDeclaration, RoundCount};
