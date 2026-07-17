//! A tiny standard-alphabet base64 encoder (GTW-741).
//!
//! MCP image content carries the raw bytes as base64 text. The bin's strict dependency
//! set (`gdtf_qa_protocol` / `serde` / `serde_json` / `ron`) does NOT include a base64
//! crate, so this module supplies the ~30 lines of standard RFC 4648 encoding it needs
//! rather than pulling a dependency. Encode-only — the bridge never decodes base64.

/// The RFC 4648 standard base64 alphabet (index → output byte).
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode raw bytes as standard (RFC 4648) base64 with `=` padding.
///
/// Used to place a screenshot's PNG bytes into an MCP image-content block. Pure — no
/// I/O; every three input bytes become four output characters, with one or two `=` pads
/// on a short final group. An empty input encodes to an empty string.
#[must_use]
pub fn encode_standard(bytes: &[u8]) -> String {
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk.first().copied().unwrap_or(0);
        let b1 = chunk.get(1).copied();
        let b2 = chunk.get(2).copied();
        // The first two output sextets always exist (a chunk has at least one byte).
        out.push(ALPHABET[(b0 >> 2) as usize]);
        out.push(ALPHABET[(((b0 & 0b11) << 4) | (b1.unwrap_or(0) >> 4)) as usize]);
        match (b1, b2) {
            (Some(one), Some(two)) => {
                out.push(ALPHABET[(((one & 0b1111) << 2) | (two >> 6)) as usize]);
                out.push(ALPHABET[(two & 0b0011_1111) as usize]);
            }
            (Some(one), None) => {
                out.push(ALPHABET[((one & 0b1111) << 2) as usize]);
                out.push(b'=');
            }
            _ => {
                out.push(b'=');
                out.push(b'=');
            }
        }
    }
    // Every pushed byte is from the ASCII alphabet or `=`, so this is always valid UTF-8.
    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::encode_standard;

    /// The canonical RFC 4648 test vectors, including the three padding cases.
    #[test]
    fn encodes_the_rfc4648_vectors() {
        assert_eq!(encode_standard(b""), "");
        assert_eq!(encode_standard(b"f"), "Zg==");
        assert_eq!(encode_standard(b"fo"), "Zm8=");
        assert_eq!(encode_standard(b"foo"), "Zm9v");
        assert_eq!(encode_standard(b"foob"), "Zm9vYg==");
        assert_eq!(encode_standard(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode_standard(b"foobar"), "Zm9vYmFy");
    }

    /// The `+` and `/` alphabet characters appear for the byte patterns that select them.
    #[test]
    fn uses_the_plus_and_slash_alphabet() {
        assert_eq!(encode_standard(&[0xFB]), "+w==");
        assert_eq!(encode_standard(&[0xFF, 0xFF]), "//8=");
    }
}
