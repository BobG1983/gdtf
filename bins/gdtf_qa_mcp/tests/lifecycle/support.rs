use core::time::Duration;
use std::{
    io::{self, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Instant,
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

const STDERR_SYNC_LIMIT: Duration = Duration::from_secs(30);

const STDERR_SYNC_STEP: Duration = Duration::from_millis(1);

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

fn await_captured_stderr(child: &dyn ManagedChild) {
    let deadline = Instant::now() + STDERR_SYNC_LIMIT;
    while !child.failure_tail().contains(STUB_STDERR_LINE) {
        if Instant::now() >= deadline {
            unreachable!(
                "the placeholder process writes {STUB_STDERR_LINE} to stderr and it is captured"
            );
        }
        thread::sleep(STDERR_SYNC_STEP);
    }
}

#[derive(Clone)]
pub(crate) struct FakeGameGate(Arc<AtomicBool>);

impl FakeGameGate {
    pub(crate) fn open(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

const GATE_POLL_STEP: Duration = Duration::from_millis(1);

pub(crate) fn spawn_gated_fake_game() -> (u16, FakeGameGate) {
    let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) else {
        unreachable!("the test can bind a loopback listener");
    };
    let Ok(addr) = listener.local_addr() else {
        unreachable!("the listener has a local address");
    };
    let port = addr.port();
    let gate = FakeGameGate(Arc::new(AtomicBool::new(false)));
    let open = Arc::clone(&gate.0);
    thread::spawn(move || {
        while !open.load(Ordering::SeqCst) {
            thread::sleep(GATE_POLL_STEP);
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
