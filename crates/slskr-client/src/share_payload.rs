use std::io::Read;

use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use slskr_protocol::peer::PeerMessage;

use crate::ClientError;

pub fn decompress_zlib_payload(payload: &[u8]) -> Result<Vec<u8>, ClientError> {
    let mut decoder = ZlibDecoder::new(payload);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
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
