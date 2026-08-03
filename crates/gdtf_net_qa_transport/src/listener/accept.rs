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
};

use super::serve::handle_client;
use crate::{channel::IncomingRequest, config::NetIoTimeout};

/// its wiring on `cfg(all(debug_assertions, feature = "net_qa"))` and an env var) whose
pub fn run_listener(
    listener: TcpListener,
    request_tx: Sender<IncomingRequest>,
    io_timeout: NetIoTimeout,
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
            handle_client(stream, &tx, io_timeout, &facts);
            busy.store(false, Ordering::Release);
        });
    }
}

fn reject_busy(mut stream: TcpStream) {
    if let Ok(frame) = encode(&QaResponse::Error(QaError::Busy)) {
        drop(stream.write_all(&frame));
    }
}
