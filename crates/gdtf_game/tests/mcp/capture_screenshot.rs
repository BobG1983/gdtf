use std::path::{Path, PathBuf};

use cobalt_mcp_protocol::{
    command::{AttachmentKind, CaptureRider, CommandOutcome, ReplyAttachment, RunOptions},
    ids::ShotName,
    message::{QaError, QaResponse},
};
use cobalt_screenshot::ShotDir;
use gdtf_test_utils::gpu_probe::gpu_adapter_probe;

use super::{
    capture_fixture::{
        GPU_SHOT_NAME, HEADLESS_SHOT_NAME, gpu_game_app_listening, gpu_shot_dir, headless_shot_dir,
        landing_capture_app_listening,
    },
    command_exchange::{
        APP_PHASE, CAPTURE_SCREENSHOT, exchange, exchange_all, exchange_until_not_timeout,
        ran_body, run,
    },
    socket_support::{TestError, TestResult, capture_app_listening},
};

fn named(stem: &str) -> String {
    format!("(name: Some(\"{stem}\"))")
}

fn attachment_of(reply: &QaResponse) -> Result<&ReplyAttachment, TestError> {
    let QaResponse::Outcome(CommandOutcome::Ran { attachments, .. }) = reply else {
        return Err(format!("a landed capture must RUN, got {reply:?}").into());
    };
    attachments
        .first()
        .ok_or_else(|| format!("a landed capture must carry an attachment: {reply:?}").into())
}

fn assert_png_under(attachment: &ReplyAttachment, dir: &ShotDir, stem: &str) -> PathBuf {
    assert_eq!(
        attachment.kind,
        AttachmentKind::Png,
        "the attachment must be declared a PNG: {attachment:?}",
    );
    let png = PathBuf::from(attachment.path.as_str());
    assert!(
        png.starts_with(dir.as_path()),
        "the capture must land inside the shot directory {}, not at {}",
        dir.display(),
        png.display(),
    );
    assert!(
        png.file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(stem)),
        "the requested stem must reach the file name, which was {}",
        png.display(),
    );
    assert!(
        png.exists(),
        "the game attached {} but no file is there — the reply must name a PNG that exists",
        png.display(),
    );
    png
}

// Each fixture call owns the directory its PNG landed in, so a case clears only that one.
fn clear_case_dir(png: &Path) {
    let Some(dir) = png.parent() else {
        return;
    };
    drop(std::fs::remove_dir_all(dir));
}

#[test]
fn a_landed_capture_over_the_socket_attaches_a_png_inside_the_shot_directory() -> TestResult {
    let reply = exchange(
        landing_capture_app_listening,
        run(
            CAPTURE_SCREENSHOT,
            &named(HEADLESS_SHOT_NAME),
            RunOptions::default(),
        ),
    )?;

    let attachment = attachment_of(&reply)?;
    let png = assert_png_under(attachment, &headless_shot_dir(), HEADLESS_SHOT_NAME);
    clear_case_dir(&png);
    Ok(())
}

#[test]
fn a_capture_rider_attaches_a_png_to_the_reply_its_command_produced() -> TestResult {
    let mut replies = exchange_all(
        landing_capture_app_listening,
        vec![
            run(APP_PHASE, "()", RunOptions::default()),
            run(
                APP_PHASE,
                "()",
                RunOptions::new(
                    None,
                    Some(CaptureRider::new(Some(ShotName::new(
                        HEADLESS_SHOT_NAME.to_owned(),
                    )))),
                ),
            ),
        ],
    )?;
    let Some(with_rider) = replies.pop() else {
        unreachable!("two requests yield two replies");
    };
    let Some(plain) = replies.pop() else {
        unreachable!("two requests yield two replies");
    };
    let plain_body = ran_body(APP_PHASE, plain)?;

    let QaResponse::Outcome(CommandOutcome::Ran { reply, attachments }) = &with_rider else {
        return Err(format!("a capture rider runs its command first, got {with_rider:?}").into());
    };
    assert_eq!(
        reply.as_str(),
        plain_body,
        "the rider leaves the command's own reply body alone",
    );
    assert_eq!(
        attachments.len(),
        1,
        "the rider appends exactly one attachment: {attachments:?}",
    );
    let attachment = attachment_of(&with_rider)?;
    let png = assert_png_under(attachment, &headless_shot_dir(), HEADLESS_SHOT_NAME);
    clear_case_dir(&png);
    Ok(())
}

#[test]
fn a_capture_that_never_lands_answers_timeout_not_a_refusal() -> TestResult {
    let reply = exchange(
        capture_app_listening,
        run(CAPTURE_SCREENSHOT, "()", RunOptions::default()),
    )?;

    assert!(
        matches!(reply, QaResponse::Error(QaError::Timeout)),
        "with nothing writing a PNG the capture must give up and answer Timeout, not refuse; got \
         {reply:?}",
    );
    assert!(
        !matches!(reply, QaResponse::Outcome(CommandOutcome::Unknown { .. })),
        "capture.screenshot must be a REGISTERED name, not Unknown: {reply:?}",
    );
    assert!(
        !matches!(
            reply,
            QaResponse::Outcome(CommandOutcome::BadArguments { .. })
        ),
        "`()` must deserialize into the command's args: {reply:?}",
    );
    assert!(
        !matches!(
            reply,
            QaResponse::Outcome(CommandOutcome::Unavailable { .. })
        ),
        "the command is unconditionally available, so nothing may refuse it: {reply:?}",
    );
    Ok(())
}

#[test]
fn a_real_capture_over_the_socket_decodes_and_is_not_a_black_frame() -> TestResult {
    if gpu_adapter_probe().should_skip() {
        eprintln!(
            "SKIP a_real_capture_over_the_socket_decodes_and_is_not_a_black_frame: no usable wgpu \
             adapter (GPU-less runner). Everything except the decode-and-not-black half is \
             covered by a_landed_capture_over_the_socket_attaches_a_png_inside_the_shot_\
             directory, which needs no GPU.",
        );
        return Ok(());
    }
    let reply = exchange_until_not_timeout(
        gpu_game_app_listening,
        run(
            CAPTURE_SCREENSHOT,
            &named(GPU_SHOT_NAME),
            RunOptions::default(),
        ),
    )?;

    let attachment = attachment_of(&reply)?;
    let png = assert_png_under(attachment, &gpu_shot_dir(), GPU_SHOT_NAME);
    let bytes = std::fs::read(&png)?;
    let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png);
    let Ok(decoded) = decoded else {
        return Err(format!(
            "the {} bytes at {} do not decode as a PNG: {decoded:?}",
            bytes.len(),
            png.display(),
        )
        .into());
    };
    let lit = decoded
        .to_rgba8()
        .pixels()
        .filter(|pixel| pixel.0[0] > 0 || pixel.0[1] > 0 || pixel.0[2] > 0)
        .count();
    assert!(
        lit > 0,
        "the capture at {} is a fully black {}x{} frame — the game's cameras are not rendering \
         into the render target the capture reads, so the PNG shows nothing",
        png.display(),
        decoded.width(),
        decoded.height(),
    );
    drop(std::fs::remove_dir_all(gpu_shot_dir().as_path()));
    Ok(())
}

#[test]
fn a_named_capture_that_never_lands_still_answers_timeout() -> TestResult {
    let reply = exchange(
        capture_app_listening,
        run(
            CAPTURE_SCREENSHOT,
            &named(GPU_SHOT_NAME),
            RunOptions::default(),
        ),
    )?;
    assert!(
        matches!(reply, QaResponse::Error(QaError::Timeout)),
        "a named capture must take the same path as an unnamed one — its argument shape must \
         deserialize rather than come back BadArguments; got {reply:?}",
    );
    Ok(())
}
