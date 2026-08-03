//! [`Bleeding`](crate::effects::bleed::Bleeding) is a buffered Bevy **message** (`#[derive(Message)]`), mirroring
mod downgate;
mod schedule;
mod tick;

#[cfg(test)]
mod test;

pub use downgate::mark_downed_bleeding;
pub use schedule::enemy_phase_started;
pub use tick::{BleedOngoing, BleedStarted, Bleeding, BleedingOut, tick_bleed};
