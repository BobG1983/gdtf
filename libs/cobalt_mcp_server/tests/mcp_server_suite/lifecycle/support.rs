use core::time::Duration;
use std::{
    io::{self, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
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
};

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

pub(crate) fn spawn_gated_fake_game() -> (u16, FakeGameGate) {
    let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) else {
        unreachable!("the test can bind a loopback listener");
    };
    let Ok(addr) = listener.local_addr() else {
        unreachable!("the listener has a local address");
    };
    let port = addr.port();
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
    (port, gate)
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
    let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) else {
        unreachable!("the test can bind a loopback listener for a free port");
    };
    let Ok(addr) = listener.local_addr() else {
        unreachable!("the listener has a local address");
    };
    addr.port()
}

pub(crate) const fn fast_config(boot_ms: u64) -> LifecycleConfig {
    fast_config_with_policy(boot_ms, LaunchPolicy::Reuse)
}

pub(crate) const fn always_spawning_config(boot_ms: u64) -> LifecycleConfig {
    fast_config_with_policy(boot_ms, LaunchPolicy::AlwaysSpawn)
}

pub(crate) const fn fast_config_with_policy(boot_ms: u64, policy: LaunchPolicy) -> LifecycleConfig {
    config_sweeping_every(boot_ms, SweepInterval::new(Duration::from_secs(60)), policy)
}

pub(crate) const fn config_sweeping_every(
    boot_ms: u64,
    sweep_interval: SweepInterval,
    launch_policy: LaunchPolicy,
) -> LifecycleConfig {
    LifecycleConfig::new(
        BootTimeout::new(Duration::from_millis(boot_ms)),
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

pub(crate) struct WatchFreePort;

impl OrphanWatch for WatchFreePort {
    fn inspect(&self, _port: McpPort, _timeout: ProbeTimeout) -> PortHold {
        PortHold::Free
    }

    fn stop(&self, _target: OrphanTarget) -> OrphanStop {
        OrphanStop::Stopped
    }
}
