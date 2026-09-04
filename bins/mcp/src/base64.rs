//! Minimal standard base64 encoder (no external crate).

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode bytes as standard base64 (with `=` padding).
#[must_use]
pub fn encode_standard(bytes: &[u8]) -> String {
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk.first().copied().unwrap_or(0);
        let b1 = chunk.get(1).copied();
        let b2 = chunk.get(2).copied();
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
    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::encode_standard;

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

    #[test]
    fn uses_the_plus_and_slash_alphabet() {
        assert_eq!(encode_standard(&[0xFB]), "+w==");
        assert_eq!(encode_standard(&[0xFF, 0xFF]), "//8=");
    }
}
