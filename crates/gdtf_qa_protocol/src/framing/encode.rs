use serde::Serialize;

use super::{
    error::WireError,
    limits::{FrameLen, MAX_FRAME_LEN, PREFIX_BYTES},
};

pub fn encode_frame(payload: &[u8]) -> Result<Vec<u8>, WireError> {
    let declared = FrameLen::new(u32::try_from(payload.len()).unwrap_or(u32::MAX));
    if payload.len() > MAX_FRAME_LEN.as_usize() {
        return Err(WireError::Oversize {
            declared,
            max: MAX_FRAME_LEN,
        });
    }
    let mut framed = Vec::with_capacity(PREFIX_BYTES + payload.len());
    framed.extend_from_slice(&declared.to_prefix());
    framed.extend_from_slice(payload);
    Ok(framed)
}

pub fn encode<T: Serialize>(message: &T) -> Result<Vec<u8>, WireError> {
    let Ok(payload) = ron::ser::to_string(message) else {
        return Err(WireError::Encode);
    };
    encode_frame(payload.as_bytes())
}
