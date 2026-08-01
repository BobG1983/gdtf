//! The externals the lifecycle tests supply — a stub spawner, a fake `net_qa` listener,
//! and the short-fused timing config (GTW-745).
//!
//! Only two externals stand in for the real world here: a stub [`ChildSpawner`] that
//! launches a harmless placeholder process (a `sh` that writes one stderr line then
//! sleeps) instead of the real game binary, and a fake `net_qa` listener — extending the
//! loopback harness from `loopback.rs` to answer the [`Hello`](QaRequest::Hello) handshake
//! — that stands in for the game's readiness endpoint. Decoupling the process (the stub)
//! from the listener (the fake server) is what lets one ticket prove every path without
//! launching the game: T10's smoke test drives the real `cargo run` spawner.
//!
//! [`StubSpawner`] builds a REAL [`ProcessChild`], so the production child-management
//! logic is what the tests exercise — and it returns only once the placeholder's stderr
//! line has actually reached that child's capture ring (see [`await_captured_stderr`]),
//! so no test has to race the child's write against a deadline (GTW-756).

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
    ProbeTimeout, ProcessChild, QaPort,
};
use gdtf_qa_protocol::{
    framing::{FrameDecoder, encode},
    message::{HelloFacts, ProtocolVersion, QaError, QaRequest, QaResponse, ServerNameNet},
};

/// The one line the placeholder process writes to stderr before it sleeps — what the
/// timeout path's captured tail must carry.
pub(crate) const STUB_STDERR_LINE: &str = "boot-oops";

/// How long [`await_captured_stderr`] waits for that line to reach the capture ring before
/// declaring the placeholder broken.
///
/// This is a hang guard, not a deadline the tests race: the wait ends the moment the line
/// lands (a few milliseconds in practice), so a loaded machine simply waits longer.
const STDERR_SYNC_LIMIT: Duration = Duration::from_secs(30);

/// How often that wait re-reads the capture ring.
const STDERR_SYNC_STEP: Duration = Duration::from_millis(1);

/// A spawner that launches a harmless placeholder process — NOT the game. It writes
/// [`STUB_STDERR_LINE`] to stderr (so the timeout path has a tail to capture) then sleeps,
/// ignoring the port (the fake listener binds that separately) and the launch recipe (it
/// runs a fixed `sh`, not a build). It builds a REAL [`ProcessChild`], so the production
/// child-management logic is what the tests exercise.
///
/// Its job stays narrow on purpose: it backs the timeout and stop-sequence coverage and
/// nothing else. The launch recipe reaching a REAL launcher is proved separately, against
/// the real [`CargoSpawner`](gdtf_qa_mcp::CargoSpawner), in `recipe.rs` (GTW-875).
pub(crate) struct StubSpawner;

/// The stub spawner, wired to a gated fake listener: spawning the placeholder process is
/// what lets that listener start answering, the same order a real child follows (it binds
/// its port once it is running).
pub(crate) struct GatedStubSpawner(FakeGameGate);

impl GatedStubSpawner {
    /// Wire the stub spawner to `gate`, so a spawn opens that fake listener.
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

/// Block until the placeholder's stderr line has been captured into `child`'s tail.
///
/// The line is the child's first action, but nothing says WHEN it runs — the child's write
/// and the reader thread that drains it are both on the operating system's schedule, and
/// `spawn` returns without waiting for either. Handing the manager a child whose line is
/// already captured gives the tests a real happens-before edge, so an assertion about the
/// tail never races the boot timeout that starts once this returns (GTW-756).
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

/// The switch that lets a bound fake listener start ANSWERING.
///
/// A launch now establishes who holds the port BEFORE it spawns anything (GTW-926), so a
/// fixture that answers from the moment it binds is an orphan by the manager's own
/// definition. This gate models the real order instead: the socket exists, and the listener
/// behind it starts answering when the CHILD is spawned.
#[derive(Clone)]
pub(crate) struct FakeGameGate(Arc<AtomicBool>);

impl FakeGameGate {
    /// Let the listener start answering probes.
    pub(crate) fn open(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// How often the gated listener re-reads its gate before accepting.
const GATE_POLL_STEP: Duration = Duration::from_millis(1);

/// Bind a loopback listener on an OS-assigned port that answers the `Hello` handshake only
/// once its gate is opened. Returns the bound port and that gate.
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

/// Bind a loopback listener that answers the `Hello` handshake straight away — a listener
/// already up before anything else happens, which is exactly what an orphaned child looks
/// like. Returns the bound port.
pub(crate) fn spawn_fake_game() -> u16 {
    let (port, gate) = spawn_gated_fake_game();
    gate.open();
    port
}

/// Read one framed request and answer it: `Hello` becomes `HelloOk`, anything else an
/// error. One request per connection, matching how the readiness probe connects.
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

/// Grab a loopback port with nothing listening on it (bind then drop), for the failure
/// paths where no readiness endpoint exists.
pub(crate) fn free_port() -> u16 {
    let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) else {
        unreachable!("the test can bind a loopback listener for a free port");
    };
    let Ok(addr) = listener.local_addr() else {
        unreachable!("the listener has a local address");
    };
    addr.port()
}

/// A short-fused lifecycle config so the timeout / stop paths finish in well under a
/// second, with `boot_ms` the readiness deadline.
pub(crate) const fn fast_config(boot_ms: u64) -> LifecycleConfig {
    LifecycleConfig::new(
        BootTimeout::new(Duration::from_millis(boot_ms)),
        PollInterval::new(Duration::from_millis(50)),
        KillGrace::new(Duration::from_secs(1)),
        ProbeTimeout::new(Duration::from_millis(300)),
    )
}
