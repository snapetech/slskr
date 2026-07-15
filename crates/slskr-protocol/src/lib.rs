#![doc = "Pure Soulseek wire protocol primitives and frame codecs."]

pub mod distributed;
pub mod error;
pub mod frame;
pub mod init;
pub mod obfuscation;
pub mod peer;
pub mod primitives;
pub mod server;

pub use distributed::{DistributedCode, DistributedMessage};
pub use error::{DecodeError, EncodeError};
pub use frame::{InitFrame, MessageFrame, RawFrame};
pub use init::{InitCode, InitMessage};
pub use obfuscation::{decode_rotated, encode_rotated, ROTATED_OBFUSCATION_TYPE};
pub use peer::{PeerCode, PeerMessage};
pub use primitives::{ProtocolTextEncoding, Reader, Writer};
