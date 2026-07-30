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

use crate::support::{LOAD_PHASE_REPLIES, MAX_READINESS_POLLS, PhaseReport, TestError, TestResult};

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
/// exchanges — each phase reported over `phases` as it completes.
///
/// `opened` is signalled the moment the FIRST request is on the wire, before the test body
/// has run a single frame. That ordering is what makes the `Load` observation deterministic:
/// the app answers `GetEditorQueryOptions` on one of its first frames, when its content
/// folder loads have only just been requested in `Startup` and no registry can have resolved
/// yet. Waiting to connect until after the app started ticking would instead race the asset
/// pass, which finishes in as few as six frames under parallel `cargo` contention.
///
/// The handshake rides at the END of the `Load` phase for the same reason — every frame spent
/// on it before the options request would be a frame of asset pass the observation has to
/// outrun. Handshake ORDER is the `net_qa_hello/` suite's subject; this suite's is the query
/// pair.
///
/// The five exchanges AFTER that first reply cannot be made frame-free by pipelining them:
/// the transport is lockstep per connection. `handle_frame`
/// (`crates/gdtf_net_qa_transport/src/listener/serve.rs`) blocks on the responder channel for
/// request N's reply before it decodes request N+1, so a request is not even queued into the
/// [`NetInbox`](gdtf_net_qa_transport::NetInbox) until its predecessor has been answered, and
/// every exchange costs at least one app frame no matter how the client writes them. The
/// assertions therefore BRACKET the window (the closing options reply) instead of assuming
/// it — see `assertions::assert_load_phase`.
///
/// # Errors
///
/// Any socket, codec or channel failure, or a readiness poll that never reports `Editing`.
pub(crate) fn drive(
    port: NetQaPort,
    opened: &Sender<()>,
    phases: &Sender<PhaseReport>,
) -> TestResult {
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
    // Bound the client's own read so a failure surfaces as an error rather than a hang.
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    // ONE decoder across the whole connection: a reply may arrive in the same read as the
    // tail of the previous one, and the decoder owns that leftover.
    let mut decoder = FrameDecoder::new();

    stream.write_all(&encode(&QaRequest::GetEditorQueryOptions)?)?;
    opened.send(())?;

    // An array literal evaluates its elements left to right, so this IS the request order on
    // the wire. Its length is [`LOAD_PHASE_REPLIES`], the same fixed size
    // `assertions::assert_load_phase` destructures — adding an exchange here without giving it
    // an assertion arm there does not compile.
    let load_phase: [QaResponse; LOAD_PHASE_REPLIES] = [
        read_response(&mut stream, &mut decoder)?,
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
        exchange(
            &mut stream,
            &mut decoder,
            QaRequest::QueryEditor(EditorQueryKind::Validation),
        )?,
        exchange(&mut stream, &mut decoder, QaRequest::GetEditorQueryOptions)?,
        exchange(
            &mut stream,
            &mut decoder,
            QaRequest::Hello(ProtocolVersion::CURRENT),
        )?,
    ];
    phases.send(Ok(Vec::from(load_phase)))?;

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
    phases.send(Ok(editing_phase))?;
    Ok(())
}
