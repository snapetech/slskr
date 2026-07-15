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
    let mut decompressed = Vec::new();
    {
        let mut limited = decoder.by_ref().take(limit);
        limited.read_to_end(&mut decompressed)?;
    }
    if decompressed.len() > max_decompressed_len {
        return Err(ClientError::PayloadTooLarge {
            max: max_decompressed_len,
        });
    }
    let consumed = usize::try_from(decoder.total_in()).unwrap_or(usize::MAX);
    if consumed != payload.len() {
        return Err(ClientError::TrailingCompressedData {
            remaining: payload.len().saturating_sub(consumed),
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

#[cfg(test)]
mod tests {
    use super::{compress_zlib_payload, decompress_zlib_payload_with_limit, ClientError};

    #[test]
    fn decompression_rejects_trailing_bytes_and_concatenated_streams() {
        let compressed = compress_zlib_payload(b"share payload").expect("compress payload");
        let mut with_junk = compressed.clone();
        with_junk.extend_from_slice(b"hidden");
        assert!(matches!(
            decompress_zlib_payload_with_limit(&with_junk, 1024),
            Err(ClientError::TrailingCompressedData { remaining: 6 })
        ));

        let mut concatenated = compressed;
        concatenated.extend_from_slice(
            &compress_zlib_payload(b"second stream").expect("compress second stream"),
        );
        assert!(matches!(
            decompress_zlib_payload_with_limit(&concatenated, 1024),
            Err(ClientError::TrailingCompressedData { remaining }) if remaining > 0
        ));
    }

    #[test]
    fn decompression_accepts_one_complete_stream_at_the_limit() {
        let compressed = compress_zlib_payload(b"12345678").expect("compress payload");
        assert_eq!(
            decompress_zlib_payload_with_limit(&compressed, 8).expect("decompress payload"),
            b"12345678"
        );
    }
}
