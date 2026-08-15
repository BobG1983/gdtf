use core::time::Duration;
use std::{
    io::{self, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
};

use gdtf_qa_mcp::{
    BootTimeout, ChildSpawner, KillGrace, LaunchSpec, LifecycleConfig, ManagedChild, PollInterval,
    ProbeTimeout, ProcessChild, QaPort, SweepInterval,
};
use gdtf_qa_protocol::{
    framing::{FrameDecoder, encode},
    message::{HelloFacts, ProtocolVersion, QaError, QaRequest, QaResponse, ServerNameNet},
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
    fn spawn(&self, port: QaPort, spec: &LaunchSpec) -> io::Result<Box<dyn ManagedChild>> {
        let child = StubSpawner.spawn(port, spec)?;
        self.0.open();
        Ok(child)
    }
}

impl ChildSpawner for StubSpawner {
    fn spawn(&self, _port: QaPort, _spec: &LaunchSpec) -> io::Result<Box<dyn ManagedChild>> {
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
        let response = match frame.decode::<QaRequest>() {
            Ok(QaRequest::Hello(_)) => QaResponse::HelloOk(HelloFacts::new(
                ProtocolVersion::new(1),
                ServerNameNet::new("fake-net-qa".to_owned()),
            )),
            Ok(_) => QaResponse::Error(QaError::Malformed),
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
    config_sweeping_every(boot_ms, SweepInterval::new(Duration::from_secs(60)))
}

pub(crate) const fn config_sweeping_every(
    boot_ms: u64,
    sweep_interval: SweepInterval,
) -> LifecycleConfig {
    LifecycleConfig::new(
        BootTimeout::new(Duration::from_millis(boot_ms)),
        PollInterval::new(Duration::from_millis(50)),
        KillGrace::new(Duration::from_secs(1)),
        ProbeTimeout::new(Duration::from_millis(300)),
        sweep_interval,
    )
}
