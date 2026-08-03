mod path;
mod payload;
mod pump;
mod verify;

#[cfg(test)]
mod test;

pub(in crate::dev::net_qa) use path::ShotSequence;
pub(in crate::dev::net_qa) use pump::{InFlightShots, drive_screenshots};

crate::support_use!(path::QaShotDir;);
crate::support_use!(payload::ScreenshotPayload;);
crate::support_use!(pump::ShotPollBudget;);
