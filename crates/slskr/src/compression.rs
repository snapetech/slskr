//! Response compression module for Phase 11
//!
//! Provides gzip compression support for HTTP responses.
//! Note: This is a placeholder for future tower-http integration

use std::io::Write;

/// Compression configuration
#[derive(Debug, Clone, Copy)]
pub struct CompressionConfig {
    /// Enable compression
    pub enabled: bool,
    /// Minimum size to compress (bytes)
    pub min_size: usize,
    /// Compression level (1-9)
    pub level: u32,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_size: 1024,  // Don't compress under 1KB
            level: 6,        // Default zlib compression level
        }
    }
}

/// Compress data with gzip
pub fn compress_gzip(data: &[u8], config: CompressionConfig) -> Result<Vec<u8>, String> {
    if !config.enabled || data.len() < config.min_size {
        return Ok(data.to_vec());
    }

    use flate2::write::GzEncoder;
    use flate2::Compression;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::new(config.level));
    encoder
        .write_all(data)
        .map_err(|e| format!("gzip encode error: {}", e))?;

    encoder
        .finish()
        .map_err(|e| format!("gzip finish error: {}", e))
}

/// Check if client accepts gzip
pub fn accepts_gzip(accept_encoding: Option<&str>) -> bool {
    accept_encoding
        .map(|ae| ae.contains("gzip"))
        .unwrap_or(false)
}

/// Get compression headers for response
pub fn compression_headers(original_size: usize, compressed_size: usize) -> String {
    let ratio = if original_size > 0 {
        (100.0 * compressed_size as f64) / (original_size as f64)
    } else {
        100.0
    };

    format!(
        "Content-Encoding: gzip\r\nX-Original-Content-Length: {}\r\nX-Compression-Ratio: {:.1}%\r\n",
        original_size, ratio
    )
}

/// Strategy for when to compress
#[derive(Debug, Clone, Copy)]
pub enum CompressionStrategy {
    /// Always try to compress (if above min size)
    Always,
    /// Only compress for certain content types
    Selective,
    /// Compress only on slow connections
    Adaptive,
}

/// Should compress given content type?
pub fn should_compress(content_type: &str, strategy: CompressionStrategy) -> bool {
    match strategy {
        CompressionStrategy::Always => true,
        CompressionStrategy::Selective => {
            // Compress text and JSON, not binary
            content_type.contains("json")
                || content_type.contains("text")
                || content_type.contains("javascript")
        }
        CompressionStrategy::Adaptive => {
            // Same as selective for now
            content_type.contains("json")
                || content_type.contains("text")
                || content_type.contains("javascript")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accepts_gzip() {
        assert!(accepts_gzip(Some("gzip, deflate")));
        assert!(!accepts_gzip(Some("deflate")));
        assert!(!accepts_gzip(None));
    }

    #[test]
    fn test_should_compress() {
        assert!(should_compress(
            "application/json",
            CompressionStrategy::Selective
        ));
        assert!(should_compress(
            "text/plain",
            CompressionStrategy::Selective
        ));
        assert!(!should_compress(
            "image/jpeg",
            CompressionStrategy::Selective
        ));
    }

    #[test]
    fn test_compression_config_default() {
        let config = CompressionConfig::default();
        assert!(config.enabled);
        assert_eq!(config.min_size, 1024);
        assert_eq!(config.level, 6);
    }
}
