use std::{
    io::Write,
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    thread,
};

use gdtf_qa_protocol::{
    framing::encode,
    message::{HelloFacts, QaError, QaResponse},
    timeouts::NetTimeouts,
};

use super::serve::handle_client;
use crate::channel::IncomingRequest;

/// its wiring on `cfg(debug_assertions)` and an env var) whose
pub fn run_listener(
    listener: TcpListener,
    request_tx: Sender<IncomingRequest>,
    timeouts: NetTimeouts,
    facts: HelloFacts,
) {
    let busy = Arc::new(AtomicBool::new(false));
    for stream in listener.incoming() {
        let Ok(stream) = stream else {
            continue;
        };
        if busy.swap(true, Ordering::AcqRel) {
            reject_busy(stream);
            continue;
        }
        let tx = request_tx.clone();
        let busy = Arc::clone(&busy);
        let facts = facts.clone();
        thread::spawn(move || {
            handle_client(stream, &tx, timeouts, &facts);
            busy.store(false, Ordering::Release);
        });
    }
}

fn reject_busy(mut stream: TcpStream) {
    if let Ok(frame) = encode(&QaResponse::Error(QaError::Busy)) {
        drop(stream.write_all(&frame));
    }
}
