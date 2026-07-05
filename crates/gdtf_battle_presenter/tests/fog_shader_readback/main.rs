//! PIXEL-level proof that the [`TerrainFogMaterial`] WGSL fragment shader actually
//! desaturates (GTW-348 C9).
//!
//! The green suite does NOT validate WGSL — shaders load at runtime, so a shader
//! typo or a Rust-`ShaderType`-vs-WGSL layout mismatch passes `fmt`/`clippy`/`test`
//! yet breaks the render. This test closes that gap with a **real-GPU headless
//! render-to-texture readback**: it builds a render-capable `App` on the real Metal
//! backend (no window), renders a single [`TerrainFogMaterial`] quad over a KNOWN
//! solid-colour in-memory texture (NOT the shipped atlas) to an offscreen `Image`,
//! reads the rendered pixels back off the GPU via [`Readback`], and asserts the
//! desaturation contract on the actual shader output:
//!
//! - at `saturation = 0.0` the output is GREYSCALE (`R ~= G ~= B`) at the source's
//!   preserved BT.709 luminance (the EXPLORED / "was visible" memory cue);
//! - at `saturation = 1.0` the output RETAINS the source hue (full colour, the
//!   VISIBLE cell).
//!
//! Colour-space accounting: the test texture and the render target are both
//! `Rgba8UnormSrgb`. The GPU hardware linearises the sampled sRGB texture before the
//! fragment runs, the BT.709 luma + `mix` operate in linear light, and the target
//! re-encodes to sRGB on write — so the readback bytes are sRGB-encoded. The
//! greyscale assertion (`R == G == B`) is invariant under the sRGB round-trip
//! (channel-identical), and the luminance check decodes the readback grey back to
//! linear before comparing to the linear BT.709 luma of the linear source.
//!
//! Environment: this needs a real GPU adapter (Metal on macOS). It runs single
//! threaded — multiple simultaneous Bevy `App`s each spinning up a Metal device can
//! exhaust the device under parallel test load — and forces synchronous pipeline
//! compilation + disables pipelined rendering so the readback resolves within a
//! bounded `app.update()` loop on this thread. If no adapter is present (e.g. a
//! GPU-less CI runner) the harness skips with a logged note rather than failing —
//! the proof is valid where a GPU exists, which is the machine this fix renders on.

mod color;
mod desaturation;
mod gpu;
