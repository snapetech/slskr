use slskr_client::version::{
    minor_version_in_reserved_band, CLIENT_MAJOR_VERSION, CLIENT_MINOR_VERSION,
    DEFAULT_LISTEN_PORT, DEFAULT_SERVER_ADDRESS, RESERVED_VERSION_BAND_END,
    RESERVED_VERSION_BAND_START,
};

#[test]
fn version_band_constants_are_consistent() {
    assert_eq!(CLIENT_MAJOR_VERSION, 175);
    assert!(minor_version_in_reserved_band(CLIENT_MINOR_VERSION));
    assert!(minor_version_in_reserved_band(RESERVED_VERSION_BAND_START));
    assert!(minor_version_in_reserved_band(RESERVED_VERSION_BAND_END));
    assert!(!minor_version_in_reserved_band(
        RESERVED_VERSION_BAND_START - 1
    ));
    assert!(!minor_version_in_reserved_band(
        RESERVED_VERSION_BAND_END + 1
    ));
    assert_eq!(DEFAULT_SERVER_ADDRESS, "server.slsknet.org:2242");
    assert_eq!(DEFAULT_LISTEN_PORT, 2234);
}
