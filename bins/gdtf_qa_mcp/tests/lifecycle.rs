//! Game-lifecycle integration — the REAL launch / stop logic against a bevy-free fake
//! game server and a real stub child process (GTW-745).
//!
//! The [`GameManager`], its readiness poll, timeout handling, and SIGTERM→SIGKILL→reap
//! stop sequence are the actual production code here. Only two externals are supplied by
//! the test: a stub [`GameSpawner`] that launches a harmless placeholder process (a `sh`
//! that writes one stderr line then sleeps) instead of the real game binary, and a fake
//! `net_qa` listener — extending the loopback harness from `loopback.rs` to answer the
//! [`Hello`](QaRequest::Hello) handshake — that stands in for the game's readiness endpoint.
//! Decoupling the process (the stub) from the listener (the fake server) is what lets one
//! ticket prove every path without launching the game: T10's smoke test drives the real
//! `cargo run` spawner.

use core::time::Duration;
use std::{
    io::{self, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    process::{Command, Stdio},
    thread,
};

use gdtf_qa_mcp::{
    BootTimeout, GameChild, GameLifecycle, GameManager, GamePort, GameSpawner, KillGrace,
    LaunchFailure, LaunchOutcome, LifecycleConfig, PollInterval, ProbeTimeout, ProcessChild,
    StopOutcome,
};
use gdtf_qa_protocol::{
    envelope::{HelloFacts, ProtocolVersion, QaError, QaRequest, QaResponse, ServerNameNet},
    framing::{FrameDecoder, encode},
};

/// A spawner that launches a harmless placeholder process — NOT the game. It writes one
/// line to stderr (so the timeout path has a tail to capture) then sleeps, ignoring the
/// port (the fake listener binds that separately). It builds a REAL [`ProcessChild`], so
/// the production child-management logic is what the tests exercise.
struct StubSpawner;

impl GameSpawner for StubSpawner {
    fn spawn(&self, _port: GamePort) -> io::Result<Box<dyn GameChild>> {
        let mut command = Command::new("sh");
        command
            .args(["-c", "echo boot-oops 1>&2; exec sleep 10"])
            .stdout(Stdio::null());
        ProcessChild::spawn(command)
    }
}

/// Bind a loopback listener on an OS-assigned port and answer the `Hello` handshake on a
/// background thread for as many probes as arrive. Returns the bound port.
fn spawn_fake_game() -> u16 {
    let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) else {
        unreachable!("the test can bind a loopback listener");
    };
    let Ok(addr) = listener.local_addr() else {
        unreachable!("the listener has a local address");
    };
    let port = addr.port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                return;
            };
            answer_one(&mut stream);
        }
    });
    port
}

/// Read one framed request and answer it: `Hello` becomes `HelloOk`, anything else an
/// error. One request per connection, matching how the readiness probe connects.
fn answer_one(stream: &mut TcpStream) {
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
            Ok(_) => QaResponse::Error(QaError::BadRequest),
            Err(_) => return,
        };
        if let Ok(out) = encode(&response) {
            drop(stream.write_all(&out));
        }
        return;
    }
}

/// Grab a loopback port with nothing listening on it (bind then drop), for the timeout
/// path where no readiness endpoint exists.
fn free_port() -> u16 {
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
const fn fast_config(boot_ms: u64) -> LifecycleConfig {
    LifecycleConfig::new(
        BootTimeout::new(Duration::from_millis(boot_ms)),
        PollInterval::new(Duration::from_millis(50)),
        KillGrace::new(Duration::from_secs(1)),
        ProbeTimeout::new(Duration::from_millis(300)),
    )
}

/// A launch against a listening fake server becomes ready; a stop then reaps the child and
/// a second stop reports nothing running.
#[test]
fn launch_becomes_ready_then_stops() {
    let port = spawn_fake_game();
    let mut manager = GameManager::with_config(Box::new(StubSpawner), fast_config(2000));

    let outcome = manager.launch(GamePort::new(port));
    let LaunchOutcome::Launched {
        port: ready_port,
        pid,
    } = outcome
    else {
        unreachable!("launch against the fake server becomes ready: {outcome:?}");
    };
    assert_eq!(*ready_port, port);

    let stopped = manager.stop();
    let StopOutcome::Stopped { pid: stopped_pid } = stopped else {
        unreachable!("stop reaps the running child: {stopped:?}");
    };
    assert_eq!(pid, stopped_pid, "stop reaps the child that launched");
    assert_eq!(manager.stop(), StopOutcome::NotRunning);
}

/// A second launch while a child is already running is ensure-style — the same child, no
/// second spawn.
#[test]
fn second_launch_is_already_running() {
    let port = spawn_fake_game();
    let mut manager = GameManager::with_config(Box::new(StubSpawner), fast_config(2000));

    let LaunchOutcome::Launched { pid: first_pid, .. } = manager.launch(GamePort::new(port)) else {
        unreachable!("the first launch becomes ready");
    };
    let second = manager.launch(GamePort::new(port));
    let LaunchOutcome::AlreadyRunning {
        port: existing_port,
        pid,
    } = second
    else {
        unreachable!("a second launch is already-running, not a second spawn: {second:?}");
    };
    assert_eq!(*existing_port, port);
    assert_eq!(pid, first_pid, "already-running reports the same child");

    let _ = manager.stop();
}

/// With no readiness endpoint the launch times out, kills the orphaned child, and returns
/// a typed failure carrying the child's captured stderr tail.
#[test]
fn launch_times_out_and_captures_stderr() {
    let port = free_port();
    let mut manager = GameManager::with_config(Box::new(StubSpawner), fast_config(800));

    let outcome = manager.launch(GamePort::new(port));
    let LaunchOutcome::Failed(LaunchFailure::Timeout(tail)) = outcome else {
        unreachable!("no listener means the launch times out: {outcome:?}");
    };
    assert!(
        tail.contains("boot-oops"),
        "the failure carries the child's stderr tail: {}",
        tail.as_str()
    );
    // The orphaned child was already killed and reaped, so there is nothing left to stop.
    assert_eq!(manager.stop(), StopOutcome::NotRunning);
}

/// Stopping with nothing running is a typed no-op, never a hang or a signal to a dead pid.
#[test]
fn stop_with_nothing_running_is_not_running() {
    let mut manager = GameManager::with_config(Box::new(StubSpawner), fast_config(2000));
    assert_eq!(manager.stop(), StopOutcome::NotRunning);
}
