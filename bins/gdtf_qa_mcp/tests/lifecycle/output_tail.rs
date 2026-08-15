use std::{process::Command, thread};

use gdtf_qa_mcp::{ManagedChild, OUTPUT_TAIL_LINES, ProcessChild, TailLines};

enum Stream {
    Out,
    Err,
}

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

// The wait has no deadline: the capture thread always delivers the lines, load only delays them.
fn await_captured(child: &dyn ManagedChild, wanted: &[&str]) -> String {
    loop {
        let tail = child.output_tail(TailLines::default());
        if wanted.iter().all(|line| tail.contains(line)) {
            return (*tail).clone();
        }
        thread::yield_now();
    }
}

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
    child.reap();

    let capped = child.output_tail(TailLines::new(2));
    let lines: Vec<&str> = capped.lines().collect();
    assert_eq!(
        lines,
        vec!["three", "four"],
        "a cap of two returns the two NEWEST lines: {capped:?}",
    );
}

#[test]
fn the_ring_evicts_the_oldest_lines_past_its_cap() {
    let over = *OUTPUT_TAIL_LINES + 8;
    let mut command = Command::new("sh");
    command.args([
        "-c",
        &format!("i=1; while [ $i -le {over} ]; do echo line-$i; i=$((i+1)); done"),
    ]);
    let Ok(mut child) = ProcessChild::spawn(command) else {
        unreachable!("the test can spawn a placeholder child");
    };
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
