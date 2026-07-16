use crate::{
    error::{DecodeError, EncodeError},
    frame::MessageFrame,
    primitives::{ProtocolTextEncoding, Reader, Writer},
};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use std::io::{Read, Write};

const MAX_DECOMPRESSED_SEARCH_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PeerCode {
    PrivateMessage = 1,
    GetShareFileList = 4,
    SharedFileListResponse = 5,
    FileSearchRequest = 8,
    FileSearchResponse = 9,
    RoomInvitation = 10,
    CancelledQueuedTransfer = 14,
    UserInfoRequest = 15,
    UserInfoResponse = 16,
    SendConnectToken = 33,
    MoveDownloadToTop = 34,
    FolderContentsRequest = 36,
    FolderContentsResponse = 37,
    TransferRequest = 40,
    TransferResponse = 41,
    PlaceholdUpload = 42,
    QueueUpload = 43,
    PlaceInQueueResponse = 44,
    UploadFailed = 46,
    ExactFileSearchRequest = 47,
    QueuedDownloads = 48,
    IndirectFileSearchRequest = 49,
    UploadDenied = 50,
    PlaceInQueueRequest = 51,
    UploadQueueNotification = 52,
}

impl PeerCode {
    pub const ALL: &'static [Self] = &[
        Self::PrivateMessage,
        Self::GetShareFileList,
        Self::SharedFileListResponse,
        Self::FileSearchRequest,
        Self::FileSearchResponse,
        Self::RoomInvitation,
        Self::CancelledQueuedTransfer,
        Self::UserInfoRequest,
        Self::UserInfoResponse,
        Self::SendConnectToken,
        Self::MoveDownloadToTop,
        Self::FolderContentsRequest,
        Self::FolderContentsResponse,
        Self::TransferRequest,
        Self::TransferResponse,
        Self::PlaceholdUpload,
        Self::QueueUpload,
        Self::PlaceInQueueResponse,
        Self::UploadFailed,
        Self::ExactFileSearchRequest,
        Self::QueuedDownloads,
        Self::IndirectFileSearchRequest,
        Self::UploadDenied,
        Self::PlaceInQueueRequest,
        Self::UploadQueueNotification,
    ];

    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
}

impl TryFrom<u32> for PeerCode {
    type Error = u32;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let code = match value {
            1 => Self::PrivateMessage,
            4 => Self::GetShareFileList,
            5 => Self::SharedFileListResponse,
            8 => Self::FileSearchRequest,
            9 => Self::FileSearchResponse,
            10 => Self::RoomInvitation,
            14 => Self::CancelledQueuedTransfer,
            15 => Self::UserInfoRequest,
            16 => Self::UserInfoResponse,
            33 => Self::SendConnectToken,
            34 => Self::MoveDownloadToTop,
            36 => Self::FolderContentsRequest,
            37 => Self::FolderContentsResponse,
            40 => Self::TransferRequest,
            41 => Self::TransferResponse,
            42 => Self::PlaceholdUpload,
            43 => Self::QueueUpload,
            44 => Self::PlaceInQueueResponse,
            46 => Self::UploadFailed,
            47 => Self::ExactFileSearchRequest,
            48 => Self::QueuedDownloads,
            49 => Self::IndirectFileSearchRequest,
            50 => Self::UploadDenied,
            51 => Self::PlaceInQueueRequest,
            52 => Self::UploadQueueNotification,
            _ => return Err(value),
        };
        Ok(code)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileAttribute {
    pub code: u32,
    pub value: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub code: u8,
    pub filename: String,
    pub filename_encoding: ProtocolTextEncoding,
    pub size: u64,
    pub extension: String,
    pub extension_encoding: ProtocolTextEncoding,
    pub attributes: Vec<FileAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSearchResponse {
    pub username: String,
    pub token: u32,
    pub results: Vec<FileEntry>,
    pub slot_free: bool,
    pub average_speed: u32,
    pub queue_length: u32,
    pub unknown: u32,
    pub private_results: Vec<FileEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserInfo {
    pub description: String,
    pub picture: Option<Vec<u8>>,
    pub total_uploads: u32,
    pub queue_size: u32,
    pub slots_free: bool,
    pub upload_permissions: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderContentsRequest {
    pub token: u32,
    pub folder: String,
    pub folder_encoding: ProtocolTextEncoding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferRequest {
    pub direction: u32,
    pub token: u32,
    pub filename: String,
    pub filename_encoding: ProtocolTextEncoding,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferResponse {
    Allowed { token: u32, size: Option<u64> },
    Rejected { token: u32, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerMessage {
    PrivateMessage(Vec<u8>),
    GetShareFileList,
    SharedFileListResponse(Vec<u8>),
    FileSearchRequest { token: u32, query: String },
    FileSearchResponse(FileSearchResponse),
    RoomInvitation(Vec<u8>),
    CancelledQueuedTransfer(Vec<u8>),
    UserInfoRequest,
    UserInfoResponse(UserInfo),
    SendConnectToken(Vec<u8>),
    MoveDownloadToTop(Vec<u8>),
    FolderContentsRequest(FolderContentsRequest),
    FolderContentsResponse(Vec<u8>),
    TransferRequest(TransferRequest),
    TransferResponse(TransferResponse),
    PlaceholdUpload { filename: String },
    QueueUpload { filename: String },
    PlaceInQueueResponse { filename: String, place: u32 },
    UploadFailed { filename: String },
    ExactFileSearchRequest(Vec<u8>),
    QueuedDownloads(Vec<u8>),
    IndirectFileSearchRequest(Vec<u8>),
    UploadDenied { filename: String, reason: String },
    PlaceInQueueRequest { filename: String },
    UploadQueueNotification,
    Unknown { code: u32, payload: Vec<u8> },
}

impl PeerMessage {
    pub fn decode(frame: MessageFrame) -> Result<Self, DecodeError> {
        let Ok(code) = PeerCode::try_from(frame.code) else {
            return Ok(Self::Unknown {
                code: frame.code,
                payload: frame.payload,
            });
        };

        if code == PeerCode::SharedFileListResponse {
            return Ok(Self::SharedFileListResponse(frame.payload));
        }
        if code == PeerCode::FolderContentsResponse {
            return Ok(Self::FolderContentsResponse(frame.payload));
        }
        match code {
            PeerCode::PrivateMessage => return Ok(Self::PrivateMessage(frame.payload)),
            PeerCode::RoomInvitation => return Ok(Self::RoomInvitation(frame.payload)),
            PeerCode::CancelledQueuedTransfer => {
                return Ok(Self::CancelledQueuedTransfer(frame.payload));
            }
            PeerCode::SendConnectToken => return Ok(Self::SendConnectToken(frame.payload)),
            PeerCode::MoveDownloadToTop => return Ok(Self::MoveDownloadToTop(frame.payload)),
            PeerCode::ExactFileSearchRequest => {
                return Ok(Self::ExactFileSearchRequest(frame.payload));
            }
            PeerCode::QueuedDownloads => return Ok(Self::QueuedDownloads(frame.payload)),
            PeerCode::IndirectFileSearchRequest => {
                return Ok(Self::IndirectFileSearchRequest(frame.payload));
            }
            _ => {}
        }

        let mut reader = Reader::new(&frame.payload);
        let message = match code {
            PeerCode::GetShareFileList => Self::GetShareFileList,
            PeerCode::FileSearchRequest => Self::FileSearchRequest {
                token: reader.read_u32_le()?,
                query: reader.read_string()?,
            },
            PeerCode::FileSearchResponse => {
                return Ok(Self::FileSearchResponse(decode_search_response_payload(
                    &frame.payload,
                )?));
            }
            PeerCode::UserInfoRequest => Self::UserInfoRequest,
            PeerCode::UserInfoResponse => Self::UserInfoResponse(decode_user_info(&mut reader)?),
            PeerCode::FolderContentsRequest => {
                let token = reader.read_u32_le()?;
                let (folder, folder_encoding) = reader.read_string_with_encoding()?;
                Self::FolderContentsRequest(FolderContentsRequest {
                    token,
                    folder,
                    folder_encoding,
                })
            }
            PeerCode::TransferRequest => {
                let direction = reader.read_u32_le()?;
                let token = reader.read_u32_le()?;
                let (filename, filename_encoding) = reader.read_string_with_encoding()?;
                let size = if !reader.is_empty() {
                    Some(reader.read_u64_le()?)
                } else {
                    None
                };
                Self::TransferRequest(TransferRequest {
                    direction,
                    token,
                    filename,
                    filename_encoding,
                    size,
                })
            }
            PeerCode::TransferResponse => {
                Self::TransferResponse(decode_transfer_response(&mut reader)?)
            }
            PeerCode::PlaceholdUpload => Self::PlaceholdUpload {
                filename: reader.read_string()?,
            },
            PeerCode::QueueUpload => Self::QueueUpload {
                filename: reader.read_string()?,
            },
            PeerCode::PlaceInQueueResponse => Self::PlaceInQueueResponse {
                filename: reader.read_string()?,
                place: reader.read_u32_le()?,
            },
            PeerCode::UploadFailed => Self::UploadFailed {
                filename: reader.read_string()?,
            },
            PeerCode::UploadDenied => Self::UploadDenied {
                filename: reader.read_string()?,
                reason: reader.read_string()?,
            },
            PeerCode::PlaceInQueueRequest => Self::PlaceInQueueRequest {
                filename: reader.read_string()?,
            },
            PeerCode::UploadQueueNotification => Self::UploadQueueNotification,
            _ => {
                return Ok(Self::Unknown {
                    code: code.as_u32(),
                    payload: frame.payload,
                })
            }
        };

        reader.finish()?;
        Ok(message)
    }

    pub fn encode(&self) -> Result<MessageFrame, EncodeError> {
        let mut writer = Writer::new();
        let code = match self {
            Self::PrivateMessage(payload) => {
                return Ok(MessageFrame::new(
                    PeerCode::PrivateMessage.as_u32(),
                    payload.clone(),
                ))
            }
            Self::GetShareFileList => PeerCode::GetShareFileList,
            Self::SharedFileListResponse(payload) => {
                return Ok(MessageFrame::new(
                    PeerCode::SharedFileListResponse.as_u32(),
                    payload.clone(),
                ))
            }
            Self::FileSearchRequest { token, query } => {
                writer.write_u32_le(*token);
                writer.write_string(query)?;
                PeerCode::FileSearchRequest
            }
            Self::FileSearchResponse(value) => {
                let mut inner = Writer::new();
                encode_search_response(&mut inner, value)?;
                let payload = compress_zlib(&inner.into_inner())?;
                return Ok(MessageFrame::new(
                    PeerCode::FileSearchResponse.as_u32(),
                    payload,
                ));
            }
            Self::RoomInvitation(payload) => {
                return Ok(MessageFrame::new(
                    PeerCode::RoomInvitation.as_u32(),
                    payload.clone(),
                ))
            }
            Self::CancelledQueuedTransfer(payload) => {
                return Ok(MessageFrame::new(
                    PeerCode::CancelledQueuedTransfer.as_u32(),
                    payload.clone(),
                ))
            }
            Self::UserInfoRequest => PeerCode::UserInfoRequest,
            Self::UserInfoResponse(value) => {
                writer.write_string(&value.description)?;
                match &value.picture {
                    Some(picture) => {
                        writer.write_bool(true);
                        writer.write_len_prefixed_bytes("picture", picture)?;
                    }
                    None => writer.write_bool(false),
                }
                writer.write_u32_le(value.total_uploads);
                writer.write_u32_le(value.queue_size);
                writer.write_bool(value.slots_free);
                if let Some(upload_permissions) = value.upload_permissions {
                    writer.write_u32_le(upload_permissions);
                }
                PeerCode::UserInfoResponse
            }
            Self::SendConnectToken(payload) => {
                return Ok(MessageFrame::new(
                    PeerCode::SendConnectToken.as_u32(),
                    payload.clone(),
                ))
            }
            Self::MoveDownloadToTop(payload) => {
                return Ok(MessageFrame::new(
                    PeerCode::MoveDownloadToTop.as_u32(),
                    payload.clone(),
                ))
            }
            Self::FolderContentsRequest(value) => {
                writer.write_u32_le(value.token);
                writer.write_string_with_encoding(&value.folder, value.folder_encoding)?;
                PeerCode::FolderContentsRequest
            }
            Self::FolderContentsResponse(payload) => {
                return Ok(MessageFrame::new(
                    PeerCode::FolderContentsResponse.as_u32(),
                    payload.clone(),
                ))
            }
            Self::TransferRequest(value) => {
                writer.write_u32_le(value.direction);
                writer.write_u32_le(value.token);
                writer.write_string_with_encoding(&value.filename, value.filename_encoding)?;
                if let Some(size) = value.size {
                    writer.write_u64_le(size);
                }
                PeerCode::TransferRequest
            }
            Self::TransferResponse(value) => {
                encode_transfer_response(&mut writer, value)?;
                PeerCode::TransferResponse
            }
            Self::PlaceholdUpload { filename } => {
                writer.write_string(filename)?;
                PeerCode::PlaceholdUpload
            }
            Self::QueueUpload { filename } => {
                writer.write_string(filename)?;
                PeerCode::QueueUpload
            }
            Self::PlaceInQueueResponse { filename, place } => {
                writer.write_string(filename)?;
                writer.write_u32_le(*place);
                PeerCode::PlaceInQueueResponse
            }
            Self::UploadFailed { filename } => {
                writer.write_string(filename)?;
                PeerCode::UploadFailed
            }
            Self::ExactFileSearchRequest(payload) => {
                return Ok(MessageFrame::new(
                    PeerCode::ExactFileSearchRequest.as_u32(),
                    payload.clone(),
                ))
            }
            Self::QueuedDownloads(payload) => {
                return Ok(MessageFrame::new(
                    PeerCode::QueuedDownloads.as_u32(),
                    payload.clone(),
                ))
            }
            Self::IndirectFileSearchRequest(payload) => {
                return Ok(MessageFrame::new(
                    PeerCode::IndirectFileSearchRequest.as_u32(),
                    payload.clone(),
                ))
            }
            Self::UploadDenied { filename, reason } => {
                writer.write_string(filename)?;
                writer.write_string(reason)?;
                PeerCode::UploadDenied
            }
            Self::PlaceInQueueRequest { filename } => {
                writer.write_string(filename)?;
                PeerCode::PlaceInQueueRequest
            }
            Self::UploadQueueNotification => PeerCode::UploadQueueNotification,
            Self::Unknown { code, payload } => {
                return Ok(MessageFrame::new(*code, payload.clone()))
            }
        };

        Ok(MessageFrame::new(code.as_u32(), writer.into_inner()))
    }
}

fn decode_user_info(reader: &mut Reader<'_>) -> Result<UserInfo, DecodeError> {
    let description = reader.read_string()?;
    let picture = if reader.read_bool()? {
        Some(reader.read_len_prefixed_bytes()?)
    } else {
        None
    };
    let total_uploads = reader.read_u32_le()?;
    let queue_size = reader.read_u32_le()?;
    let slots_free = reader.read_bool()?;
    let upload_permissions = if reader.is_empty() {
        None
    } else {
        Some(reader.read_u32_le()?)
    };

    Ok(UserInfo {
        description,
        picture,
        total_uploads,
        queue_size,
        slots_free,
        upload_permissions,
    })
}

fn decode_search_response(reader: &mut Reader<'_>) -> Result<FileSearchResponse, DecodeError> {
    Ok(FileSearchResponse {
        username: reader.read_string()?,
        token: reader.read_u32_le()?,
        results: decode_file_entries(reader)?,
        slot_free: reader.read_bool()?,
        average_speed: reader.read_u32_le()?,
        queue_length: reader.read_u32_le()?,
        unknown: reader.read_u32_le()?,
        private_results: decode_file_entries(reader)?,
    })
}

fn decode_search_response_payload(payload: &[u8]) -> Result<FileSearchResponse, DecodeError> {
    let decompressed = decompress_zlib(payload)?;
    let mut reader = Reader::new(&decompressed);
    decode_search_response(&mut reader)
}

fn encode_search_response(
    writer: &mut Writer,
    value: &FileSearchResponse,
) -> Result<(), EncodeError> {
    writer.write_string(&value.username)?;
    writer.write_u32_le(value.token);
    encode_file_entries(writer, &value.results)?;
    writer.write_bool(value.slot_free);
    writer.write_u32_le(value.average_speed);
    writer.write_u32_le(value.queue_length);
    writer.write_u32_le(value.unknown);
    encode_file_entries(writer, &value.private_results)
}

fn decompress_zlib(payload: &[u8]) -> Result<Vec<u8>, DecodeError> {
    decompress_zlib_with_limit(payload, MAX_DECOMPRESSED_SEARCH_RESPONSE_BYTES)
}

fn decompress_zlib_with_limit(payload: &[u8], max_len: usize) -> Result<Vec<u8>, DecodeError> {
    let mut decoder = ZlibDecoder::new(payload);
    let limit = u64::try_from(max_len).unwrap_or(u64::MAX).saturating_add(1);
    let mut limited = decoder.by_ref().take(limit);
    let mut output = Vec::new();
    limited
        .read_to_end(&mut output)
        .map_err(|_| DecodeError::InvalidCompressedPayload("file search response"))?;
    if output.len() > max_len {
        return Err(DecodeError::InvalidCompressedPayload(
            "file search response exceeds decompression limit",
        ));
    }
    Ok(output)
}

fn compress_zlib(payload: &[u8]) -> Result<Vec<u8>, EncodeError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(payload)
        .map_err(|_| EncodeError::LengthOverflow {
            field: "zlib payload",
            length: payload.len(),
        })?;
    encoder.finish().map_err(|_| EncodeError::LengthOverflow {
        field: "zlib payload",
        length: payload.len(),
    })
}

fn decode_file_entries(reader: &mut Reader<'_>) -> Result<Vec<FileEntry>, DecodeError> {
    let count = reader.read_bounded_count("file entries", 21)?;
    let mut entries = Vec::new();
    for _ in 0..count {
        let code = reader.read_u8()?;
        let (filename, filename_encoding) = reader.read_string_with_encoding()?;
        let size = reader.read_u64_le()?;
        let (extension, extension_encoding) = reader.read_string_with_encoding()?;
        let attribute_count = reader.read_bounded_count("file attributes", 8)?;
        let mut attributes = Vec::new();
        for _ in 0..attribute_count {
            attributes.push(FileAttribute {
                code: reader.read_u32_le()?,
                value: reader.read_u32_le()?,
            });
        }
        entries.push(FileEntry {
            code,
            filename,
            filename_encoding,
            size,
            extension,
            extension_encoding,
            attributes,
        });
    }
    Ok(entries)
}

fn encode_file_entries(writer: &mut Writer, entries: &[FileEntry]) -> Result<(), EncodeError> {
    if entries.len() > 21 {
        return Err(EncodeError::InvalidCount {
            field: "file entries",
            count: entries.len(),
            maximum: 21,
        });
    }
    let count = u32::try_from(entries.len())
        .map_err(|_| EncodeError::length_overflow("file entries", entries.len()))?;
    writer.write_u32_le(count);
    for entry in entries {
        if entry.attributes.len() > 8 {
            return Err(EncodeError::InvalidCount {
                field: "file attributes",
                count: entry.attributes.len(),
                maximum: 8,
            });
        }
        writer.write_u8(entry.code);
        writer.write_string_with_encoding(&entry.filename, entry.filename_encoding)?;
        writer.write_u64_le(entry.size);
        writer.write_string_with_encoding(&entry.extension, entry.extension_encoding)?;
        let attribute_count = u32::try_from(entry.attributes.len())
            .map_err(|_| EncodeError::length_overflow("file attributes", entry.attributes.len()))?;
        writer.write_u32_le(attribute_count);
        for attribute in &entry.attributes {
            writer.write_u32_le(attribute.code);
            writer.write_u32_le(attribute.value);
        }
    }
    Ok(())
}

fn decode_transfer_response(reader: &mut Reader<'_>) -> Result<TransferResponse, DecodeError> {
    let token = reader.read_u32_le()?;
    if reader.read_bool()? {
        let size = if reader.is_empty() {
            None
        } else {
            Some(reader.read_u64_le()?)
        };
        Ok(TransferResponse::Allowed { token, size })
    } else {
        Ok(TransferResponse::Rejected {
            token,
            reason: reader.read_string()?,
        })
    }
}

fn encode_transfer_response(
    writer: &mut Writer,
    value: &TransferResponse,
) -> Result<(), EncodeError> {
    match value {
        TransferResponse::Allowed { token, size } => {
            writer.write_u32_le(*token);
            writer.write_bool(true);
            if let Some(size) = size {
                writer.write_u64_le(*size);
            }
        }
        TransferResponse::Rejected { token, reason } => {
            writer.write_u32_le(*token);
            writer.write_bool(false);
            writer.write_string(reason)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_request_decodes_optional_size_for_download_direction() {
        let mut writer = Writer::new();
        writer.write_u32_le(0);
        writer.write_u32_le(123);
        writer.write_string("shares\\file.bin").unwrap();
        writer.write_u64_le(456);

        let decoded = PeerMessage::decode(MessageFrame::new(
            PeerCode::TransferRequest.as_u32(),
            writer.into_inner(),
        ))
        .unwrap();

        assert_eq!(
            decoded,
            PeerMessage::TransferRequest(TransferRequest {
                direction: 0,
                token: 123,
                filename: "shares\\file.bin".to_owned(),
                filename_encoding: ProtocolTextEncoding::Utf8,
                size: Some(456),
            })
        );
    }

    #[test]
    fn compressed_search_response_is_bounded_before_decode() {
        let compressed = compress_zlib(&vec![b'x'; 1024]).expect("compress fixture");
        let error = decompress_zlib_with_limit(&compressed, 128)
            .expect_err("decompression limit must reject expansion");
        assert!(matches!(
            error,
            DecodeError::InvalidCompressedPayload(
                "file search response exceeds decompression limit"
            )
        ));
    }

    #[test]
    fn search_response_encoder_rejects_decoder_incompatible_counts() {
        let entry = FileEntry {
            code: 1,
            filename: "file.flac".to_owned(),
            filename_encoding: ProtocolTextEncoding::Utf8,
            size: 1,
            extension: "flac".to_owned(),
            extension_encoding: ProtocolTextEncoding::Utf8,
            attributes: Vec::new(),
        };
        let mut response = FileSearchResponse {
            username: "peer".to_owned(),
            token: 1,
            results: vec![entry.clone(); 22],
            slot_free: true,
            average_speed: 0,
            queue_length: 0,
            unknown: 0,
            private_results: Vec::new(),
        };
        assert!(matches!(
            PeerMessage::FileSearchResponse(response.clone()).encode(),
            Err(EncodeError::InvalidCount {
                field: "file entries",
                count: 22,
                maximum: 21
            })
        ));

        response.results = vec![FileEntry {
            attributes: vec![FileAttribute { code: 0, value: 0 }; 9],
            ..entry
        }];
        assert!(matches!(
            PeerMessage::FileSearchResponse(response).encode(),
            Err(EncodeError::InvalidCount {
                field: "file attributes",
                count: 9,
                maximum: 8
            })
        ));
    }
}
