//! The two-sided socket timeout reaping an idle client (GTW-736).

use std::{
    io::Read,
    net::{Ipv4Addr, TcpStream},
    time::Duration,
};

use gdtf_net_qa_transport::NetIoTimeout;

use super::harness::{TestResult, spawn_fake_host_side, spawn_listener};

/// The two-sided socket timeout reaps an idle client: a client that connects and sends
/// nothing is closed by the server after the read timeout, which the client observes as
/// EOF (a zero-length read).
#[test]
fn an_idle_client_is_reaped_by_the_read_timeout() -> TestResult {
    // Short server timeout so the reap is observed quickly.
    let (port, inbox) = spawn_listener(NetIoTimeout::new(Duration::from_millis(150)))?;
    spawn_fake_host_side(inbox);

    let mut client = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
    // A client-side safety timeout so a broken server fails the test instead of hanging.
    client.set_read_timeout(Some(Duration::from_secs(5)))?;

    // Send nothing: the server's read times out (~150ms), the handler reaps us, and our
    // blocking read returns EOF once the server closes.
    let mut buf = [0u8; 16];
    let read = client.read(&mut buf)?;
    assert_eq!(
        read, 0,
        "the server must close an idle connection via the read-timeout reap",
    );
    Ok(())
}
