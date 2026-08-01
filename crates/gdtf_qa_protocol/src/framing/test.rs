//! Property pins for the pure framing codec — split reads, oversize / junk rejection,
//! and the empty / max-size payload edges (GTW-734).

use crate::{
    framing::{Frame, FrameDecoder, MAX_FRAME_LEN, WireError, encode, encode_frame},
    message::{ProtocolVersion, QaRequest},
};

/// A representative message to frame in the codec pins.
fn a_message() -> QaRequest {
    QaRequest::Hello(ProtocolVersion::new(1))
}

/// Drain every frame currently available from `decoder`, failing loudly (no bare
/// `panic!`) on an unexpected codec error.
fn drain(decoder: &mut FrameDecoder) -> Vec<Frame> {
    let mut out = Vec::new();
    loop {
        match decoder.next_frame() {
            Ok(Some(frame)) => out.push(frame),
            Ok(None) => return out,
            Err(err) => unreachable!("no codec error is expected here: {err}"),
        }
    }
}

/// A message survives the full encode → frame-decode → decode path identically.
#[test]
fn round_trips_a_message_through_the_codec() {
    let message = a_message();
    let Ok(framed) = encode(&message) else {
        unreachable!("a small message encodes");
    };
    let mut decoder = FrameDecoder::new();
    decoder.push(&framed);
    let out = drain(&mut decoder);
    assert_eq!(out.len(), 1, "exactly one frame comes out");
    let Ok(parsed) = out[0].decode::<QaRequest>() else {
        unreachable!("the frame decodes back to a QaRequest");
    };
    assert_eq!(parsed, message, "the message is unchanged");
}

/// A read split at ANY byte boundary — mid-prefix or mid-payload — still reassembles
/// into exactly one frame that decodes to the original.
#[test]
fn tolerates_reads_split_at_every_boundary() {
    let message = a_message();
    let Ok(framed) = encode(&message) else {
        unreachable!("a small message encodes");
    };
    for split in 0..=framed.len() {
        let mut decoder = FrameDecoder::new();
        decoder.push(&framed[..split]);
        let mut out = drain(&mut decoder);
        decoder.push(&framed[split..]);
        out.extend(drain(&mut decoder));
        assert_eq!(out.len(), 1, "one frame reassembles at split {split}");
        let Ok(parsed) = out[0].decode::<QaRequest>() else {
            unreachable!("the reassembled frame decodes at split {split}");
        };
        assert_eq!(parsed, message, "the message is unchanged at split {split}");
    }
}

/// A prefix declaring more than the cap is rejected with the typed error, from the
/// prefix alone (before any payload is buffered).
#[test]
fn oversize_frame_is_rejected() {
    let Ok(over) = u32::try_from(MAX_FRAME_LEN.as_usize() + 1) else {
        unreachable!("the cap plus one fits a u32");
    };
    let mut decoder = FrameDecoder::new();
    decoder.push(&over.to_be_bytes());
    let result = decoder.next_frame();
    assert!(
        matches!(result, Err(WireError::Oversize { .. })),
        "an oversize prefix is rejected: {result:?}"
    );
    // The decoder stays latched in the error state.
    assert!(
        matches!(decoder.next_frame(), Err(WireError::Oversize { .. })),
        "the decoder stays poisoned after an oversize prefix"
    );
}

/// A well-formed frame whose payload is junk (non-RON, or non-UTF-8) decodes to the
/// typed `Malformed` error, never a panic.
#[test]
fn junk_payload_is_rejected_with_the_typed_error() {
    for junk in [b"not <valid> ron @#$".to_vec(), vec![0xFF_u8, 0xFE, 0x00]] {
        let Ok(framed) = encode_frame(&junk) else {
            unreachable!("junk bytes still frame (framing does not parse them)");
        };
        let mut decoder = FrameDecoder::new();
        decoder.push(&framed);
        let out = drain(&mut decoder);
        assert_eq!(out.len(), 1, "the junk framed cleanly");
        let result = out[0].decode::<QaRequest>();
        assert!(
            matches!(result, Err(WireError::Malformed)),
            "junk decodes to Malformed: {result:?}"
        );
    }
}

/// An empty payload frames to a bare 4-byte prefix and decodes back as an empty frame.
#[test]
fn empty_payload_frames_and_reassembles() {
    let Ok(framed) = encode_frame(&[]) else {
        unreachable!("an empty payload frames");
    };
    assert_eq!(
        framed,
        vec![0, 0, 0, 0],
        "an empty frame is a zero-length prefix"
    );
    let mut decoder = FrameDecoder::new();
    decoder.push(&framed);
    let out = drain(&mut decoder);
    assert_eq!(out.len(), 1, "the empty frame is yielded");
    assert!(out[0].payload().is_empty(), "its payload is empty");
}

/// A payload of exactly the cap is accepted; one byte over is rejected — the max-size
/// boundary.
#[test]
fn max_size_payload_boundary() {
    let at_cap = vec![b'x'; MAX_FRAME_LEN.as_usize()];
    let framed = encode_frame(&at_cap);
    assert!(
        framed.is_ok(),
        "a payload at the cap is accepted: {framed:?}"
    );

    let over_cap = vec![b'x'; MAX_FRAME_LEN.as_usize() + 1];
    let result = encode_frame(&over_cap);
    assert!(
        matches!(result, Err(WireError::Oversize { .. })),
        "a payload one byte over the cap is rejected: {result:?}"
    );
}
