//! Screenshot-capture env config: the frame parse, the multi-frame schedule, the
//! capture-path gate, and the `.fNN` naming. The env SNAPSHOT + resolution (and its
//! GTW-590 loud diagnostics) are pinned by the sibling `resolve` test module.

// `CaptureFrame` etc. + the pure parse cores are not re-exported from `mod.rs` (only
// `DevCapturePlugin` is, for the binary), so reach them through their home submodules.
use super::super::capture_config::{CaptureFrame, CaptureFrames, frame_path, parse_capture_path};

/// `CaptureFrame::DEFAULT` is 15 frames and `Default` agrees with it — the wait the
/// affordance uses when `GDTF_CAPTURE_FRAME` is unset, so the UI layout flushes before
/// the capture.
#[test]
fn capture_frame_default_is_fifteen() {
    assert_eq!(*CaptureFrame::DEFAULT, 15);
    assert_eq!(CaptureFrame::default(), CaptureFrame::DEFAULT);
}

/// The REAL frame parse ([`CaptureFrame::parse`], the core of `from_env`) takes a valid
/// `u32` and falls back to [`CaptureFrame::DEFAULT`] for an empty / non-numeric / absent
/// value — proving the parse path is wired without mutating the shared environment.
#[test]
fn capture_frame_parses_or_defaults() {
    assert_eq!(*CaptureFrame::parse(Some("0")), 0);
    assert_eq!(*CaptureFrame::parse(Some("7")), 7);
    assert_eq!(*CaptureFrame::parse(Some(" 42 ")), 42);
    for bad in ["", "  ", "x", "-1", "1.5", "12abc"] {
        assert_eq!(
            CaptureFrame::parse(Some(bad)),
            CaptureFrame::DEFAULT,
            "{bad:?} should fall back to the default frame",
        );
    }
    assert_eq!(CaptureFrame::parse(None), CaptureFrame::DEFAULT);
}

/// The REAL multi-frame parse ([`CaptureFrames::parse`], the core of `from_env`) turns a
/// `GDTF_CAPTURE_FRAMES` comma-list into a SORTED, DEDUPED, NON-EMPTY frame schedule —
/// and falls back to the single `GDTF_CAPTURE_FRAME` frame when the list is absent or has
/// no valid entry (the GTW-297 single-frame path stays working, GTW-306).
#[test]
fn capture_frames_parses_sorts_dedupes_or_falls_back() {
    let fallback = CaptureFrame::parse(Some("9"));

    // A real list: split, trim, sort, dedupe.
    let frames = CaptureFrames::parse(Some("16, 12 ,14,12,18"), fallback);
    let got: Vec<u32> = frames.iter().map(|f| **f).collect();
    assert_eq!(got, vec![12, 14, 16, 18], "sorted + deduped frame schedule");

    // A list with junk entries keeps only the valid u32s.
    let mixed = CaptureFrames::parse(Some("5,x,,7,-1"), fallback);
    let mixed_got: Vec<u32> = mixed.iter().map(|f| **f).collect();
    assert_eq!(mixed_got, vec![5, 7], "junk entries are dropped");

    // Absent / all-junk list -> the single fallback frame (single-frame path preserved).
    for empty in [None, Some(""), Some("   "), Some("x,,-1")] {
        let single = CaptureFrames::parse(empty, fallback);
        let single_got: Vec<u32> = single.iter().map(|f| **f).collect();
        assert_eq!(
            single_got,
            vec![9],
            "{empty:?} should fall back to the single GDTF_CAPTURE_FRAME frame",
        );
    }
}

/// The REAL multi-frame naming ([`frame_path`]) inserts a `.fNN` tag before the
/// extension so each frame writes its own PNG, and appends when there is no extension.
/// The single-frame path uses the exact base path (not this helper), so its filename is
/// unchanged — covered structurally by the schedule having one entry.
#[test]
fn frame_path_tags_the_frame_before_the_extension() {
    use std::path::{Path, PathBuf};

    assert_eq!(
        frame_path(Path::new("/abs/out.png"), CaptureFrame::parse(Some("12"))),
        PathBuf::from("/abs/out.f12.png"),
        "the frame tag goes before the extension",
    );
    assert_eq!(
        frame_path(Path::new("/abs/shot"), CaptureFrame::parse(Some("7"))),
        PathBuf::from("/abs/shot.f7"),
        "with no extension the tag is appended",
    );
}

/// The REAL capture-path gate ([`parse_capture_path`], the core of `capture_path`) is
/// `None` for an unset / empty / whitespace value and `Some` for a real path — the gate
/// the affordance keys on. Driving the injected core avoids racing the process-global
/// env var across parallel tests.
#[test]
fn capture_path_gate_rejects_empty() {
    assert!(
        parse_capture_path(None).is_none(),
        "unset path leaves the affordance inert",
    );
    assert!(
        parse_capture_path(Some("")).is_none(),
        "empty path leaves it inert",
    );
    assert!(
        parse_capture_path(Some("   ")).is_none(),
        "whitespace path leaves it inert",
    );
    assert_eq!(
        parse_capture_path(Some("/abs/out.png")),
        Some(std::path::PathBuf::from("/abs/out.png")),
        "a real path enables the affordance",
    );
}

/// The GTW-590 activation-log schedule rendering: [`CaptureFrames`] `Display`s as the
/// bare comma list QA typed into `GDTF_CAPTURE_FRAMES`, so the `dev-capture: capture
/// ON -> path at BattleRunning frame(s) 10,60,150,300` line echoes the configuration
/// back verbatim.
#[test]
fn capture_frames_display_is_the_comma_list() {
    let frames = CaptureFrames::parse(Some("60, 10 ,300,150"), CaptureFrame::DEFAULT);
    assert_eq!(frames.to_string(), "10,60,150,300", "sorted comma list");
    let single = CaptureFrames::parse(None, CaptureFrame::parse(Some("9")));
    assert_eq!(
        single.to_string(),
        "9",
        "single-frame schedule renders bare"
    );
}
