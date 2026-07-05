//! sRGB / BT.709 colour-space math + the captured-pixel type.

use bevy::prelude::*;

/// A captured readback pixel (the centre of the rendered quad), sRGB-encoded bytes.
#[derive(Resource, Default, Clone, Copy)]
pub(crate) struct CapturedPixel {
    /// Whether the readback observer fired and populated this.
    pub(crate) captured: bool,
    /// `[R, G, B, A]` sRGB-encoded (0..=255), as written to the `Rgba8UnormSrgb` target.
    pub(crate) rgba:     [u8; 4],
}

/// sRGB-encode a single linear channel (IEC 61966-2-1), to 0..=255.
pub(crate) fn srgb_encode(linear: f32) -> u8 {
    let c = linear.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055_f32.mul_add(c.powf(1.0 / 2.4), -0.055)
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

/// sRGB-decode a single 0..=255 channel to linear light (IEC 61966-2-1).
pub(crate) fn srgb_decode(byte: u8) -> f32 {
    let s = f32::from(byte) / 255.0;
    if s <= 0.040_45 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// BT.709 luma of a linear-light RGB triple — the exact weights the WGSL uses.
pub(crate) fn bt709_luma(linear_rgb: [f32; 3]) -> f32 {
    linear_rgb[0].mul_add(
        0.2126,
        linear_rgb[1].mul_add(0.7152, linear_rgb[2] * 0.0722),
    )
}
