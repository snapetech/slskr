use std::io::Read;

use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use slskr_protocol::peer::PeerMessage;

use crate::ClientError;

pub const DEFAULT_MAX_DECOMPRESSED_SHARE_PAYLOAD: usize = 64 * 1024 * 1024;

pub fn decompress_zlib_payload(payload: &[u8]) -> Result<Vec<u8>, ClientError> {
    decompress_zlib_payload_with_limit(payload, DEFAULT_MAX_DECOMPRESSED_SHARE_PAYLOAD)
}

pub fn decompress_zlib_payload_with_limit(
    payload: &[u8],
    max_decompressed_len: usize,
) -> Result<Vec<u8>, ClientError> {
    let mut decoder = ZlibDecoder::new(payload);
    let limit = u64::try_from(max_decompressed_len)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut limited = decoder.by_ref().take(limit);
    let mut decompressed = Vec::new();
    limited.read_to_end(&mut decompressed)?;
    if decompressed.len() > max_decompressed_len {
        return Err(ClientError::PayloadTooLarge {
            max: max_decompressed_len,
        });
    }
    Ok(decompressed)
}

pub fn compress_zlib_payload(payload: &[u8]) -> Result<Vec<u8>, ClientError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    std::io::copy(&mut &payload[..], &mut encoder)?;
    Ok(encoder.finish()?)
}

pub fn decompress_peer_share_payload(
    message: &PeerMessage,
) -> Option<Result<Vec<u8>, ClientError>> {
    match message {
        PeerMessage::SharedFileListResponse(payload)
        | PeerMessage::FolderContentsResponse(payload) => Some(decompress_zlib_payload(payload)),
        _ => None,
    }
}
