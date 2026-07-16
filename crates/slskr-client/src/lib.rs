#![doc = "Async Soulseek client orchestration."]

pub mod capabilities;
pub mod connection;
pub mod distributed_tree;
pub mod error;
pub mod events;
pub mod file_transfer;
pub mod filters;
pub mod io;
pub mod listener;
pub mod manager;
pub mod mesh;
pub mod overlay;
pub mod peer_cache;
pub mod peer_connect;
pub mod search;
pub mod server;
pub mod share_payload;
pub mod social;
pub mod stream;
pub mod transfer;
pub mod version;

pub use error::ClientError;
pub use slskr_protocol as protocol;
