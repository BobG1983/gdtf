use cobalt_test_utils::gpu_adapter_probe;
use gdtf_battle_presenter::Brightness;

use super::{
    color::{bt709_luma, srgb_decode, srgb_encode},
    gpu::render_and_read,
};

#[test]
fn explored_saturation_zero_renders_greyscale_at_preserved_luma() {
    if gpu_adapter_probe().should_skip() {
        eprintln!("SKIP: no usable GPU adapter in this environment — greyscale proof not run");
        return;
    }

    let source = [255_u8, 0, 0, 255];
    let Some([r, g, b, a]) = render_and_read(source, 0.0, Brightness::FULL) else {
        eprintln!("SKIP: no GPU adapter in this environment — greyscale proof not run");
        return;
    };

    assert!(a >= 128, "pixel discarded by alpha mask: a={a}");

    let eps = 3_i16;
    assert!(
        (i16::from(r) - i16::from(g)).abs() <= eps,
        "sat=0 not greyscale: R={r} G={g} B={b} (R vs G)"
    );
    assert!(
        (i16::from(g) - i16::from(b)).abs() <= eps,
        "sat=0 not greyscale: R={r} G={g} B={b} (G vs B)"
    );

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

#[test]
fn visible_saturation_one_retains_source_hue() {
    if gpu_adapter_probe().should_skip() {
        eprintln!("SKIP: no usable GPU adapter in this environment — hue-retention proof not run");
        return;
    }

    let source = [200_u8, 60, 30, 255];
    let Some([r, g, b, a]) = render_and_read(source, 1.0, Brightness::FULL) else {
        eprintln!("SKIP: no GPU adapter in this environment — hue-retention proof not run");
        return;
    };

    assert!(a >= 128, "pixel discarded by alpha mask: a={a}");

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

    assert!(
        (i16::from(r) - i16::from(b)).abs() > 20,
        "sat=1 collapsed toward grey: R={r} G={g} B={b}; shader over-desaturated"
    );
}

#[test]
fn lower_storey_brightness_dims_the_rendered_tile() {
    if gpu_adapter_probe().should_skip() {
        eprintln!("SKIP: no usable GPU adapter in this environment — brightness proof not run");
        return;
    }

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

    assert!(
        dr < fr && dg < fg && db < fb,
        "brightness=0.5 did NOT darken the tile: full=({fr},{fg},{fb}) dim=({dr},{dg},{db}) — \
         the brightness multiply is missing",
    );

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
