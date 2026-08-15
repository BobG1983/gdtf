use std::{
    net::{Ipv4Addr, TcpStream},
    time::Duration,
};

use gdtf_qa_protocol::{
    message::{ProtocolVersion, QaError, QaRequest, QaResponse},
    timeouts::{NetIoTimeout, NetReplyTimeout, NetTimeouts},
};

use super::harness::{
    TestResult, host_reply_facts, read_response, spawn_fake_host_side, spawn_listener, test_facts,
    write_request,
};

#[test]
fn hello_round_trips_and_a_second_client_is_busy() -> TestResult {
    // Never fire during this test; reaping has its own suite.
    let (port, inbox) = spawn_listener(NetTimeouts::new(
        NetIoTimeout::new(Duration::MAX),
        NetReplyTimeout::new(Duration::MAX),
    ))?;
    spawn_fake_host_side(inbox);

    let mut client_a = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
    write_request(&mut client_a, &QaRequest::Hello(ProtocolVersion::CURRENT))?;
    let response_a = read_response(&mut client_a)?;
    assert!(
        matches!(
            &response_a,
            QaResponse::HelloOk(facts) if *facts == test_facts()
        ),
        "expected the LISTENER's own HelloOk for client A, got {response_a:?}",
    );

    write_request(&mut client_a, &QaRequest::Catalogue)?;
    let forwarded = read_response(&mut client_a)?;
    assert!(
        matches!(
            &forwarded,
            QaResponse::HelloOk(facts) if *facts == host_reply_facts()
        ),
        "expected the forwarded request to be answered by the host side, got {forwarded:?}",
    );

    let mut client_b = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
    let response_b = read_response(&mut client_b)?;
    assert!(
        matches!(&response_b, QaResponse::Error(QaError::Busy)),
        "expected Busy for the second concurrent client, got {response_b:?}",
    );

    drop(client_a);
    Ok(())
}
