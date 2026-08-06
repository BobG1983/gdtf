use std::{
    io::Read,
    net::{Ipv4Addr, TcpStream},
    time::Duration,
};

use gdtf_qa_protocol::timeouts::{DEFAULT_REPLY_TIMEOUT, NetIoTimeout, NetTimeouts};

use super::harness::{TestResult, spawn_fake_host_side, spawn_listener};

#[test]
fn an_idle_client_is_reaped_by_the_read_timeout_not_by_the_reply_wait() -> TestResult {
    let timeouts = NetTimeouts::new(
        NetIoTimeout::new(Duration::from_millis(150)),
        DEFAULT_REPLY_TIMEOUT,
    );
    let (port, inbox) = spawn_listener(timeouts)?;
    spawn_fake_host_side(inbox);

    let mut client = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
    client.set_read_timeout(Some(Duration::from_secs(5)))?;

    let mut buf = [0u8; 16];
    let read = client.read(&mut buf)?;
    assert_eq!(
        read, 0,
        "the server must close an idle connection via the read-timeout reap, and the long reply \
         wait must not hold the channel open past it",
    );
    Ok(())
}
