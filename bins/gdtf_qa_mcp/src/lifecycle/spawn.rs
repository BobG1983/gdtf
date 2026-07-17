//! How the game child is launched — the [`GameSpawner`] trait and its real
//! [`CargoSpawner`] (GTW-745).
//!
//! [`GameSpawner`] is the one piece the lifecycle manager takes as a dependency, so a test
//! can supply a stub that launches a harmless placeholder process while every other part
//! of the launch / stop logic runs unchanged. [`CargoSpawner`] is the real one: it runs
//! the dev launch recipe — the `net_qa`-featured build with the QA environment set.

use std::{
    io,
    process::{Command, Stdio},
};

use super::child::{GameChild, ProcessChild};
use crate::game::GamePort;

/// The environment variable that opts the game into the QA control channel.
const NET_QA_ENV: &str = "GDTF_NET_QA";

/// The environment variable that picks the game's loopback listen port.
const NET_QA_PORT_ENV: &str = "GDTF_NET_QA_PORT";

/// Launches the game child bound to a chosen port.
///
/// The manager depends on this trait, not on [`CargoSpawner`], so the readiness / timeout
/// / stop logic can be driven against a stub process in tests without launching the real
/// game binary.
pub trait GameSpawner {
    /// Spawn the game child, telling it to listen on `port`.
    ///
    /// # Errors
    ///
    /// The underlying [`io::Error`] if the child process cannot be spawned.
    fn spawn(&self, port: GamePort) -> io::Result<Box<dyn GameChild>>;
}

/// The real spawner — runs `cargo run` for the `net_qa`-featured dev build with the QA
/// environment set.
///
/// The child's stdout is discarded (the MCP host's own stdout is the JSON-RPC channel and
/// must not be polluted); its stderr is captured by [`ProcessChild`] for the failure tail.
#[derive(Debug, Clone, Copy, Default)]
pub struct CargoSpawner;

impl CargoSpawner {
    /// Build the real spawner.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl GameSpawner for CargoSpawner {
    fn spawn(&self, port: GamePort) -> io::Result<Box<dyn GameChild>> {
        let mut command = Command::new("cargo");
        command
            .args([
                "run",
                "-p",
                "grimdark_turfwar",
                "--features",
                "dynamic_linking,net_qa",
            ])
            .env(NET_QA_ENV, "1")
            .env(NET_QA_PORT_ENV, format!("{}", *port))
            .stdin(Stdio::null())
            .stdout(Stdio::null());
        ProcessChild::spawn(command)
    }
}
