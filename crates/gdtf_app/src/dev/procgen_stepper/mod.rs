mod clock;
mod commands;
mod drive;
mod gate;
mod plugin;
mod schematic;
#[cfg(not(feature = "headless_test"))]
mod ui;

crate::support_use!(plugin::ProcgenStepperPlugin;);

#[cfg(feature = "headless_test")]
pub use commands::AutoStepDelay;
#[cfg(feature = "headless_test")]
pub use commands::{AutoRunning, PendingStepCommand, StepCommand};
pub(crate) use gate::battle_setup_runs_directly;
crate::support_use!(gate::ProcgenStepperActive;);
#[cfg(feature = "headless_test")]
pub use schematic::draw_schematic;
