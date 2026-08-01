//! The child's OUTPUT capture — both streams, into one ring, read back through
//! `ManagedChild::output_tail` (GTW-943).
//!
//! This is the plumbing the `logs` tool stands on, and it is real throughout: a real `sh`
//! child writing to both of its streams, the real `ProcessChild` piping and draining them on
//! real reader threads, and the real ring the tail is rendered from. Before this the host set
//! the child's stdout to `Stdio::null()`, so anything a healthy child printed was gone before
//! anyone could ask for it (the GTW-904 finding).

use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

use gdtf_qa_mcp::{ManagedChild, OUTPUT_TAIL_LINES, ProcessChild, TailLines};

/// How long a read waits for a line to reach the ring before declaring the capture broken.
///
/// A hang guard, not a deadline the test races: the child's write and the reader thread that
/// drains it are both on the operating system's schedule, and the wait ends the moment the
/// line lands.
const CAPTURE_LIMIT: Duration = Duration::from_secs(30);

/// How often that wait re-reads the ring.
const CAPTURE_STEP: Duration = Duration::from_millis(1);

/// Which of the child's streams a line is written to.
enum Stream {
    /// The stream the host used to discard.
    Out,
    /// The stream the host has always captured.
    Err,
}

/// Spawn a real child that writes each `(stream, line)` in order, then exits.
///
/// The stream is EXPLICIT, never derived from position: the two reader threads drain their
/// pipes on the operating system's schedule, so lines from different streams interleave in
/// whatever order those threads observe them. A test that needs a deterministic ORDER writes
/// every line to one stream; a test that needs both streams asserts membership, not order.
fn child_printing(lines: &[(Stream, &str)]) -> Box<dyn ManagedChild> {
    let script = lines
        .iter()
        .map(|(stream, line)| match stream {
            Stream::Out => format!("echo {line}"),
            Stream::Err => format!("echo {line} 1>&2"),
        })
        .collect::<Vec<String>>()
        .join("; ");
    let mut command = Command::new("sh");
    command.args(["-c", &script]);
    let Ok(child) = ProcessChild::spawn(command) else {
        unreachable!("the test can spawn a placeholder child");
    };
    child
}

/// Block until every line in `wanted` has reached the ring.
fn await_captured(child: &dyn ManagedChild, wanted: &[&str]) -> String {
    let deadline = Instant::now() + CAPTURE_LIMIT;
    loop {
        let tail = child.output_tail(TailLines::default());
        if wanted.iter().all(|line| tail.contains(line)) {
            return (*tail).clone();
        }
        assert!(
            Instant::now() < deadline,
            "the child's lines must reach the capture ring; it held {tail:?}",
        );
        thread::sleep(CAPTURE_STEP);
    }
}

/// BOTH streams are captured, and the tail carries what the child actually printed.
///
/// The stdout line is the one that matters: it is the stream the host used to discard.
#[test]
fn both_output_streams_reach_the_tail() {
    let mut child = child_printing(&[(Stream::Out, "from-stdout"), (Stream::Err, "from-stderr")]);
    let tail = await_captured(child.as_ref(), &["from-stdout", "from-stderr"]);
    assert!(
        tail.contains("from-stdout"),
        "the child's STDOUT must be captured, not discarded: {tail:?}",
    );
    assert!(
        tail.contains("from-stderr"),
        "the child's stderr must still be captured: {tail:?}",
    );
    child.reap();
}

/// A line cap returns the LAST `max` lines, so a caller can bound a chatty child.
///
/// Every line goes to ONE stream, so the ring's order is the order the child wrote them —
/// the only arrangement in which "the newest two" is a determinate answer.
#[test]
fn a_line_cap_keeps_the_newest_lines() {
    let mut child = child_printing(&[
        (Stream::Out, "one"),
        (Stream::Out, "two"),
        (Stream::Out, "three"),
        (Stream::Out, "four"),
    ]);
    drop(await_captured(
        child.as_ref(),
        &["one", "two", "three", "four"],
    ));
    // Reap first: it joins the reader threads, so the ring is fully drained before it is
    // read (the GTW-756 ordering the failure paths follow).
    child.reap();

    let capped = child.output_tail(TailLines::new(2));
    let lines: Vec<&str> = capped.lines().collect();
    assert_eq!(
        lines,
        vec!["three", "four"],
        "a cap of two returns the two NEWEST lines: {capped:?}",
    );
}

/// A chatty child cannot grow the ring without limit: the oldest lines are evicted.
///
/// The cap is what stops a long-running child's output from growing the HOST's memory
/// forever, and it became load-bearing when `logs` turned this ring from a failure-only
/// buffer into a live read. Written against [`OUTPUT_TAIL_LINES`] rather than the number 512,
/// so retuning the depth retunes the test — what is pinned is that a bound EXISTS, not how
/// deep it is.
#[test]
fn the_ring_evicts_the_oldest_lines_past_its_cap() {
    let over = *OUTPUT_TAIL_LINES + 8;
    // A shell loop, not one `echo` per line: the script is built once and stays short
    // whatever the cap is set to.
    let mut command = Command::new("sh");
    command.args([
        "-c",
        &format!("i=1; while [ $i -le {over} ]; do echo line-$i; i=$((i+1)); done"),
    ]);
    let Ok(mut child) = ProcessChild::spawn(command) else {
        unreachable!("the test can spawn a placeholder child");
    };
    // Reap joins the reader threads, so every line the child wrote has reached the ring
    // before it is read — no polling, no clock in the assertion.
    child.reap();

    let whole = child.output_tail(TailLines::new(over * 2));
    let lines: Vec<&str> = whole.lines().collect();
    let oldest_kept = format!("line-{}", over - *OUTPUT_TAIL_LINES + 1);
    let newest = format!("line-{over}");
    assert_eq!(
        lines.len(),
        *OUTPUT_TAIL_LINES,
        "the ring holds at most its cap, however much the child printed",
    );
    assert_eq!(
        lines.first().copied(),
        Some(oldest_kept.as_str()),
        "the OLDEST lines are the ones dropped",
    );
    assert_eq!(
        lines.last().copied(),
        Some(newest.as_str()),
        "and the newest line is still there",
    );
}

/// A cap of zero returns nothing — an explicit "I want none of it", not the whole ring.
#[test]
fn a_zero_cap_returns_nothing() {
    let mut child = child_printing(&[(Stream::Out, "one"), (Stream::Err, "two")]);
    drop(await_captured(child.as_ref(), &["one", "two"]));
    child.reap();

    assert!(
        child.output_tail(TailLines::new(0)).is_empty(),
        "a cap of zero returns no lines",
    );
}
