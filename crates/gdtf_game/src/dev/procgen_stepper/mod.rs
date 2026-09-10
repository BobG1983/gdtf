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
pub use commands::{AutoRunning, AutoStepDelay, AutoStepTimer};
crate::support_use!(commands::{PendingStepCommand, StepCommand};);
crate::support_use!(gate::ProcgenStepperActive;);
#[cfg(feature = "headless_test")]
pub use schematic::draw_schematic;
