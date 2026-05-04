pub const CLIENT_NAME: &str = "slskr";
pub const CLIENT_MAJOR_VERSION: u32 = 175;
pub const CLIENT_MINOR_VERSION: u32 = 8_800_001;
pub const DEFAULT_SERVER_ADDRESS: &str = "server.slsknet.org:2242";
pub const DEFAULT_LISTEN_PORT: u32 = 2234;

pub const RESERVED_VERSION_BAND_START: u32 = 8_800_000;
pub const RESERVED_VERSION_BAND_END: u32 = 8_809_999;

#[must_use]
pub const fn minor_version_in_reserved_band(minor_version: u32) -> bool {
    minor_version >= RESERVED_VERSION_BAND_START && minor_version <= RESERVED_VERSION_BAND_END
}
