use bevy::render::{render_resource::Extent3d, renderer::RenderDevice};

use crate::window_capture::{png::frame_rgb, target::CAPTURE_FORMAT};

const WIDTH: u32 = 3;

const HEIGHT: u32 = 2;

// Anything the writer must never read: these bytes sit past the end of every row.
const PADDING_BYTE: u8 = 0xEE;

fn frame_size() -> Extent3d {
    Extent3d {
        width:                 WIDTH,
        height:                HEIGHT,
        depth_or_array_layers: 1,
    }
}

// The lit RGBA texel at (x, y), all channels distinct so a mis-strided read shows up.
fn texel(x: u32, y: u32) -> [u8; 4] {
    let seed = u8::try_from((y * WIDTH + x) % 64).unwrap_or_default();
    [17 + seed, 71 + seed, 131 + seed, 255]
}

// One readback buffer as wgpu hands it back: rows padded out to the copy alignment.
fn padded_readback() -> Vec<u8> {
    let row = WIDTH as usize * 4;
    let padded = RenderDevice::align_copy_bytes_per_row(row);
    let mut data = vec![PADDING_BYTE; padded * HEIGHT as usize];
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let at = y as usize * padded + x as usize * 4;
            data[at..at + 4].copy_from_slice(&texel(x, y));
        }
    }
    data
}

fn expected_rgb() -> Vec<u8> {
    let mut pixels = Vec::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            pixels.extend_from_slice(&texel(x, y)[..3]);
        }
    }
    pixels
}

#[test]
fn a_padded_readback_keeps_every_lit_pixel_and_drops_the_padding() {
    let data = padded_readback();
    let row = WIDTH as usize * 4;
    assert!(
        RenderDevice::align_copy_bytes_per_row(row) > row,
        "test setup: {WIDTH} pixels must be a width wgpu actually pads, or this case never \
         exercises the stride",
    );

    let found = frame_rgb(frame_size(), CAPTURE_FORMAT, &data);

    assert_eq!(
        found,
        Some(expected_rgb()),
        "the PNG writer must read each row at the readback buffer's padded stride and drop only \
         the alpha channel. Reading at the unpadded row width shifts every row into the previous \
         row's padding, which is what turns a lit frame into a black or smeared PNG",
    );
    let Some(found) = found else {
        unreachable!("the assertion above leaves the pixels present")
    };
    assert!(
        !found.contains(&PADDING_BYTE),
        "no padding byte may reach the PNG: {found:?}",
    );
}
