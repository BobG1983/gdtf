//! The shader desaturation / brightness contract on readback pixels (GTW-348 C9,
//! GTW-519 C4).

use gdtf_battle_presenter::Brightness;
use gdtf_test_utils::gpu_adapter_probe;

use super::{
    color::{bt709_luma, srgb_decode, srgb_encode},
    gpu::render_and_read,
};

/// at saturation 0.0 the rendered output is GREYSCALE (R~=G~=B) at the source's
/// preserved BT.709 luminance — the EXPLORED memory cue.
#[test]
fn explored_saturation_zero_renders_greyscale_at_preserved_luma() {
    // GTW-527: probe for a usable wgpu adapter BEFORE building any render `App` — on a
    // GPU-less runner `app.finish()` panics ("Unable to find a GPU!") before the in-build
    // `get_sub_app(RenderApp)?` guard, so skip here ahead of the app build.
    if gpu_adapter_probe().should_skip() {
        eprintln!("SKIP: no usable GPU adapter in this environment — greyscale proof not run");
        return;
    }

    // Pure red: a strongly-saturated colour so greyscale collapse is unambiguous.
    let source = [255_u8, 0, 0, 255];
    // Full brightness so this proves the saturation axis in isolation (GTW-519 brightness ==
    // 1.0 is a no-op multiply).
    let Some([r, g, b, a]) = render_and_read(source, 0.0, Brightness::FULL) else {
        eprintln!("SKIP: no GPU adapter in this environment — greyscale proof not run");
        return;
    };

    // The opaque source must survive the alpha mask (a >= 0.5 -> not discarded).
    assert!(a >= 128, "pixel discarded by alpha mask: a={a}");

    // GREYSCALE: channels equal within sRGB-quantisation noise.
    let eps = 3_i16;
    assert!(
        (i16::from(r) - i16::from(g)).abs() <= eps,
        "sat=0 not greyscale: R={r} G={g} B={b} (R vs G)"
    );
    assert!(
        (i16::from(g) - i16::from(b)).abs() <= eps,
        "sat=0 not greyscale: R={r} G={g} B={b} (G vs B)"
    );

    // LUMINANCE PRESERVED: decode the readback grey to linear, compare to the
    // linear BT.709 luma of the linear source. Both in linear light.
    let src_linear = [
        srgb_decode(source[0]),
        srgb_decode(source[1]),
        srgb_decode(source[2]),
    ];
    let expected_luma = bt709_luma(src_linear);
    let got_luma = srgb_decode(r);
    assert!(
        (got_luma - expected_luma).abs() < 0.03,
        "sat=0 luma not preserved: got {got_luma:.4} (R={r}) want ~{expected_luma:.4}"
    );

    // And it must NOT be the source hue (red would be R high, G/B ~0). A genuine
    // greyscale of red sits well below 255 in R and well above 0 in G/B.
    let src_grey = srgb_encode(expected_luma);
    assert!(
        (i16::from(r) - i16::from(src_grey)).abs() <= 3,
        "sat=0 grey byte: got R={r} want ~{src_grey}"
    );
    assert!(
        r < 200,
        "sat=0 still looks like saturated red (R={r}); shader did NOT desaturate"
    );
}

/// at saturation 1.0 the rendered output RETAINS the source hue — the VISIBLE cell.
#[test]
fn visible_saturation_one_retains_source_hue() {
    // GTW-527: probe for a usable wgpu adapter BEFORE building any render `App` (see the
    // greyscale test) — skip ahead of the app build on a GPU-less runner.
    if gpu_adapter_probe().should_skip() {
        eprintln!("SKIP: no usable GPU adapter in this environment — hue-retention proof not run");
        return;
    }

    // A mixed colour so "retains hue" is a real per-channel match, not a coincidence.
    let source = [200_u8, 60, 30, 255];
    // Full brightness so this proves the saturation axis in isolation.
    let Some([r, g, b, a]) = render_and_read(source, 1.0, Brightness::FULL) else {
        eprintln!("SKIP: no GPU adapter in this environment — hue-retention proof not run");
        return;
    };

    assert!(a >= 128, "pixel discarded by alpha mask: a={a}");

    // IDENTITY: at sat=1 the mix is fully the sampled colour; output == source
    // within the sRGB sample/encode round-trip quantisation.
    let eps = 4_i16;
    assert!(
        (i16::from(r) - i16::from(source[0])).abs() <= eps,
        "sat=1 R not preserved: got {r} want ~{}",
        source[0]
    );
    assert!(
        (i16::from(g) - i16::from(source[1])).abs() <= eps,
        "sat=1 G not preserved: got {g} want ~{}",
        source[1]
    );
    assert!(
        (i16::from(b) - i16::from(source[2])).abs() <= eps,
        "sat=1 B not preserved: got {b} want ~{}",
        source[2]
    );

    // And it must NOT have collapsed to grey (the channels must differ — hue kept).
    assert!(
        (i16::from(r) - i16::from(b)).abs() > 20,
        "sat=1 collapsed toward grey: R={r} G={g} B={b}; shader over-desaturated"
    );
}

/// GTW-519 C4 (in-engine WGSL evidence) — the `brightness` uniform DIMS the rendered tile: a
/// full-colour (saturation 1.0) tile at `brightness = 0.5` renders at ~HALF the LINEAR
/// channel values of the same tile at `brightness = 1.0`, and the two do NOT come out equal
/// (a dropped brightness multiply would leave them identical, failing this test).
///
/// The shader multiplies the (grey-mixed) RGB by `clamp(brightness, 0, 1)` AFTER the
/// saturation mix, so a lower-storey tile reads visibly darker than the active storey. The
/// comparison is in LINEAR light (the framebuffer is sRGB, so a 0.5 LINEAR scale is NOT a
/// halving of the sRGB byte) — decode each channel, compare the ratios.
#[test]
fn lower_storey_brightness_dims_the_rendered_tile() {
    // GTW-527: probe for a usable wgpu adapter BEFORE building any render `App` (see the
    // greyscale test) — skip ahead of the app build on a GPU-less runner.
    if gpu_adapter_probe().should_skip() {
        eprintln!("SKIP: no usable GPU adapter in this environment — brightness proof not run");
        return;
    }

    // A mixed opaque colour so per-channel dimming is unambiguous and hue is retained.
    let source = [200_u8, 120, 60, 255];

    let Some([fr, fg, fb, fa]) = render_and_read(source, 1.0, Brightness::FULL) else {
        eprintln!("SKIP: no GPU adapter in this environment — brightness proof not run");
        return;
    };
    let Some([dr, dg, db, da]) = render_and_read(source, 1.0, Brightness::new(0.5)) else {
        eprintln!("SKIP: no GPU adapter in this environment — brightness proof not run");
        return;
    };

    assert!(fa >= 128 && da >= 128, "pixel discarded by alpha mask");

    // The dimmed tile must be STRICTLY darker on every channel — never equal (a no-op
    // brightness would leave them identical and this fails).
    assert!(
        dr < fr && dg < fg && db < fb,
        "brightness=0.5 did NOT darken the tile: full=({fr},{fg},{fb}) dim=({dr},{dg},{db}) — \
         the brightness multiply is missing",
    );

    // In LINEAR light each dimmed channel is ~0.5x the full channel (the shader scales linear
    // RGB by brightness). Allow generous slop for the sRGB encode round-trip near-black.
    for (full_byte, dim_byte, chan) in [(fr, dr, 'R'), (fg, dg, 'G'), (fb, db, 'B')] {
        let full_lin = srgb_decode(full_byte);
        let dim_lin = srgb_decode(dim_byte);
        let want = full_lin * 0.5;
        assert!(
            full_lin.mul_add(-0.5, dim_lin).abs() < 0.05,
            "brightness=0.5 channel {chan} not ~half linear: full_lin={full_lin:.4} \
             dim_lin={dim_lin:.4} (want ~{want:.4})",
        );
    }
}
