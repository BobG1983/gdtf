//! The client half — a real socket speaking the real framing codec (GTW-805).

use std::{
    io::{Read as _, Write as _},
    net::{Ipv4Addr, TcpStream},
    sync::mpsc::Sender,
    time::Duration,
};

use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::{
    envelope::{ProtocolVersion, QaRequest, QaResponse},
    framing::{FrameDecoder, encode},
    view::{EditorQueryKind, EditorReadinessNet},
};

use crate::support::{MAX_READINESS_POLLS, PhaseReport, TestError, TestResult};

/// Read one whole framed [`QaResponse`] off the socket (tolerating a split read).
fn read_response(
    stream: &mut TcpStream,
    decoder: &mut FrameDecoder,
) -> Result<QaResponse, TestError> {
    let mut buf = [0u8; 1024];
    loop {
        if let Some(frame) = decoder.next_frame()? {
            return Ok(frame.decode::<QaResponse>()?);
        }
        let read = stream.read(&mut buf)?;
        assert!(read > 0, "the editor closed before a full response arrived");
        decoder.push(&buf[..read]);
    }
}

/// Send one request and read its reply.
fn exchange(
    stream: &mut TcpStream,
    decoder: &mut FrameDecoder,
    request: QaRequest,
) -> Result<QaResponse, TestError> {
    stream.write_all(&encode(&request)?)?;
    read_response(stream, decoder)
}

/// The client half: the `Load`-phase exchanges, a readiness poll, then the `Editing`-phase
/// exchanges — each phase reported over `tx` as it completes.
pub(crate) fn drive(port: NetQaPort, tx: &Sender<PhaseReport>) -> TestResult {
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
    // Bound the client's own read so a failure surfaces as an error rather than a hang.
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    // ONE decoder across the whole connection: a reply may arrive in the same read as the
    // tail of the previous one, and the decoder owns that leftover.
    let mut decoder = FrameDecoder::new();

    let load_phase = vec![
        exchange(
            &mut stream,
            &mut decoder,
            QaRequest::Hello(ProtocolVersion::CURRENT),
        )?,
        exchange(&mut stream, &mut decoder, QaRequest::GetEditorQueryOptions)?,
        exchange(
            &mut stream,
            &mut decoder,
            QaRequest::QueryEditor(EditorQueryKind::Readiness),
        )?,
        exchange(
            &mut stream,
            &mut decoder,
            QaRequest::QueryEditor(EditorQueryKind::Mode),
        )?,
    ];
    tx.send(Ok(load_phase))?;

    // Wait out the asset pass exactly the way a real client is told to: poll the options
    // reply until it reports `Editing`, acting on nothing before that.
    let mut options = None;
    for _ in 0..MAX_READINESS_POLLS {
        let reply = exchange(&mut stream, &mut decoder, QaRequest::GetEditorQueryOptions)?;
        if let QaResponse::EditorQueryOptions(view) = &reply
            && view.readiness == EditorReadinessNet::Editing
        {
            options = Some(reply);
            break;
        }
    }
    let options = options.ok_or("the editor never reported readiness Editing")?;

    let mut editing_phase = vec![options];
    for kind in EditorQueryKind::ALL {
        editing_phase.push(exchange(
            &mut stream,
            &mut decoder,
            QaRequest::QueryEditor(kind),
        )?);
    }
    tx.send(Ok(editing_phase))?;
    Ok(())
}
