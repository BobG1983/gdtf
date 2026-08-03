mod commands;
mod drive;
mod gate;
mod plugin;
mod schematic;
#[cfg(not(feature = "test-support"))]
mod ui;

crate::support_use!(plugin::ProcgenStepperPlugin;);

#[cfg(feature = "test-support")]
pub use commands::AutoStepDelay;
#[cfg(feature = "test-support")]
pub use commands::{AutoRunning, PendingStepCommand, StepCommand};
pub(crate) use gate::battle_setup_runs_directly;
crate::support_use!(gate::ProcgenStepperActive;);
#[cfg(feature = "test-support")]
pub use schematic::draw_schematic;
