use core::time::Duration;
use std::{
    io::{self, Read, Write},
    net::TcpStream,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
};

use cobalt_mcp_protocol::{
    framing::{FrameDecoder, encode},
    message::{
        HelloFacts, McpRequest, McpResponse, McpSessionError, ProtocolVersion, ServerNameNet,
    },
};
use cobalt_mcp_server::{
    BootTimeout, CargoPackage, ChildSpawner, EnvVarName, FeatureList, FeatureName, KillGrace,
    LaunchPolicy, LaunchSpec, LifecycleConfig, ManagedChild, McpChannel, McpPort, OrphanStop,
    OrphanTarget, OrphanWatch, PollInterval, PortHold, ProbeTimeout, ProcessChild, SweepInterval,
    SystemOrphanWatch,
};

use crate::ports::{bind_loopback, issue_free_port, port_of};

pub(crate) const STUB_STDERR_LINE: &str = "boot-oops";

pub(crate) struct StubSpawner;

pub(crate) struct GatedStubSpawner(FakeGameGate);

impl GatedStubSpawner {
    pub(crate) const fn new(gate: FakeGameGate) -> Self {
        Self(gate)
    }
}

impl ChildSpawner for GatedStubSpawner {
    fn spawn(&self, port: McpPort, spec: &LaunchSpec) -> io::Result<Box<dyn ManagedChild>> {
        let child = StubSpawner.spawn(port, spec)?;
        self.0.open();
        Ok(child)
    }
}

impl ChildSpawner for StubSpawner {
    fn spawn(&self, _port: McpPort, _spec: &LaunchSpec) -> io::Result<Box<dyn ManagedChild>> {
        let mut command = Command::new("sh");
        command
            .args(["-c", "echo boot-oops 1>&2; exec sleep 10"])
            .stdout(Stdio::null());
        let child = ProcessChild::spawn(command)?;
        await_captured_stderr(child.as_ref());
        Ok(child)
    }
}

// The wait has no deadline: the capture thread always delivers the line, load only delays it.
fn await_captured_stderr(child: &dyn ManagedChild) {
    while !child.failure_tail().contains(STUB_STDERR_LINE) {
        thread::yield_now();
    }
}

#[derive(Clone)]
pub(crate) struct FakeGameGate(mpsc::Sender<()>);

impl FakeGameGate {
    pub(crate) fn open(&self) {
        let _ = self.0.send(());
    }
}

/// Probe timeout the fixtures use while waiting for their own listener to hold the port.
pub(crate) const PROBE: ProbeTimeout = ProbeTimeout::new(Duration::from_millis(300));

// No deadline: the fixture's own listener always comes up, load only delays it.
fn await_holding(port: u16) {
    let watch = SystemOrphanWatch::new();
    while !matches!(
        watch.inspect(McpPort::new(port), PROBE),
        PortHold::Orphan(_)
    ) {
        thread::yield_now();
    }
}

pub(crate) fn spawn_gated_fake_game() -> (u16, FakeGameGate) {
    let listener = bind_loopback();
    let port = port_of(&listener);
    let (open_tx, open_rx) = mpsc::channel::<()>();
    let gate = FakeGameGate(open_tx);
    thread::spawn(move || {
        if open_rx.recv().is_err() {
            return;
        }
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                return;
            };
            answer_one(&mut stream);
        }
    });
    await_holding(port);
    (port, gate)
}

/// A listener that accepts the connection and never answers the handshake.
pub(crate) fn spawn_silent_listener() -> u16 {
    let listener = bind_loopback();
    let port = port_of(&listener);
    thread::spawn(move || {
        for stream in listener.incoming() {
            // Accepted and dropped: the connect lands, the handshake never gets an answer.
            let Ok(stream) = stream else {
                return;
            };
            drop(stream);
        }
    });
    await_holding(port);
    port
}

pub(crate) fn gated_listeners(count: usize) -> Vec<(McpPort, FakeGameGate)> {
    (0..count)
        .map(|_| {
            let (port, gate) = spawn_gated_fake_game();
            (McpPort::new(port), gate)
        })
        .collect()
}

pub(crate) fn spawn_fake_game() -> u16 {
    let (port, gate) = spawn_gated_fake_game();
    gate.open();
    port
}

pub(crate) fn answer_one(stream: &mut TcpStream) {
    let mut decoder = FrameDecoder::new();
    let mut buf = [0u8; 1024];
    loop {
        let read = match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(count) => count,
        };
        decoder.push(&buf[..read]);
        let frame = match decoder.next_frame() {
            Ok(Some(frame)) => frame,
            Ok(None) => continue,
            Err(_) => return,
        };
        let response = match frame.decode::<McpRequest>() {
            Ok(McpRequest::Hello(_)) => McpResponse::HelloOk(HelloFacts::new(
                ProtocolVersion::new(1),
                ServerNameNet::new("fake-mcp".to_owned()),
            )),
            Ok(_) => McpResponse::Error(McpSessionError::Malformed),
            Err(_) => return,
        };
        if let Ok(out) = encode(&response) {
            drop(stream.write_all(&out));
        }
        return;
    }
}

pub(crate) fn free_port() -> u16 {
    issue_free_port()
}

pub(crate) const fn fast_config(boot_ms: u64) -> LifecycleConfig {
    fast_config_with_policy(boot_ms, LaunchPolicy::Reuse)
}

pub(crate) const fn fast_config_with_policy(boot_ms: u64, policy: LaunchPolicy) -> LifecycleConfig {
    config_booting_within(
        BootTimeout::new(Duration::from_millis(boot_ms)),
        SweepInterval::new(Duration::from_secs(60)),
        policy,
    )
}

/// A config whose boot wait never runs out, for a case that must reach `Launched`.
pub(crate) const fn no_boot_deadline_config() -> LifecycleConfig {
    no_boot_deadline_config_with_policy(LaunchPolicy::Reuse)
}

/// The same never-expiring boot wait, starting a child on every launch.
pub(crate) const fn always_spawning_no_boot_deadline_config() -> LifecycleConfig {
    no_boot_deadline_config_with_policy(LaunchPolicy::AlwaysSpawn)
}

/// The same never-expiring boot wait under an explicit launch policy.
pub(crate) const fn no_boot_deadline_config_with_policy(policy: LaunchPolicy) -> LifecycleConfig {
    config_booting_within(
        BootTimeout::new(Duration::MAX),
        SweepInterval::new(Duration::from_secs(60)),
        policy,
    )
}

pub(crate) const fn config_sweeping_every(
    boot_ms: u64,
    sweep_interval: SweepInterval,
    launch_policy: LaunchPolicy,
) -> LifecycleConfig {
    config_booting_within(
        BootTimeout::new(Duration::from_millis(boot_ms)),
        sweep_interval,
        launch_policy,
    )
}

pub(crate) const fn config_booting_within(
    boot_timeout: BootTimeout,
    sweep_interval: SweepInterval,
    launch_policy: LaunchPolicy,
) -> LifecycleConfig {
    LifecycleConfig::new(
        boot_timeout,
        PollInterval::new(Duration::from_millis(50)),
        KillGrace::new(Duration::from_secs(1)),
        ProbeTimeout::new(Duration::from_millis(300)),
        sweep_interval,
        launch_policy,
    )
}

pub(crate) fn recipe_with_features(features: &[&str]) -> LaunchSpec {
    LaunchSpec::new(
        CargoPackage::new(SAMPLE_PACKAGE.to_owned()),
        FeatureList::new(
            features
                .iter()
                .map(|name| FeatureName::new((*name).to_owned()))
                .collect(),
        ),
        None,
        sample_channel(),
    )
}

// The package a test child is launched from. No test builds it; the spawners are stubs.
pub(crate) const SAMPLE_PACKAGE: &str = "sample_package";

/// Channel env var names for a host these tests register.
pub(crate) fn sample_channel() -> McpChannel {
    McpChannel::new(
        EnvVarName::new("SAMPLE_CHANNEL".to_owned()),
        EnvVarName::new("SAMPLE_CHANNEL_PORT".to_owned()),
    )
}

/// The launch recipe a registered host hands the manager when a call overrides nothing.
pub(crate) fn sample_spec() -> LaunchSpec {
    recipe_with_features(&["sample_feature"])
}

/// An orphan watch reporting every port free, for a case whose own fixture holds the port.
pub(crate) struct WatchFreePort;

impl OrphanWatch for WatchFreePort {
    fn inspect(&self, _port: McpPort, _timeout: ProbeTimeout) -> PortHold {
        PortHold::Free
    }

    fn stop(&self, _target: OrphanTarget) -> OrphanStop {
        OrphanStop::Stopped
    }
}
