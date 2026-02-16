//! Compression configuration for the DBX storage engine.
//!
//! Provides configurable compression algorithms for Parquet (ROS, Tier 5)
//! and future tier-specific compression settings.
//!
//! # Supported Algorithms
//!
//! | Algorithm | Speed | Ratio | Use Case |
//! |-----------|-------|-------|----------|
//! | Snappy | ★★★★★ | ★★★ | Default — balanced speed/ratio |
//! | LZ4 | ★★★★★ | ★★★ | Ultra-fast — latency-sensitive workloads |
//! | ZSTD | ★★★ | ★★★★★ | Best ratio — archival/cold storage |
//! | Brotli | ★★ | ★★★★★ | Maximum ratio — web deployment |
//! | None | ★★★★★ | ★ | No compression — debugging/diagnostics |
//!
//! # Example
//!
//! ```rust
//! use dbx_core::storage::compression::{CompressionConfig, CompressionAlgorithm};
//!
//! // Default: Snappy
//! let config = CompressionConfig::default();
//! assert_eq!(config.algorithm(), CompressionAlgorithm::Snappy);
//!
//! // Maximum compression with ZSTD level 9
//! let config = CompressionConfig::new(CompressionAlgorithm::Zstd)
//!     .with_level(9);
//!
//! // Ultra-fast for real-time workloads
//! let config = CompressionConfig::new(CompressionAlgorithm::Lz4);
//! ```

use parquet::basic::Compression;

/// Compression algorithm selection.
///
/// Maps to Parquet's native compression codecs. All algorithms are
/// included via parquet crate default features — no extra dependencies needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompressionAlgorithm {
    /// No compression — fastest writes, largest files.
    None,
    /// Snappy — fast compression/decompression, moderate ratio.
    /// Default algorithm. Speed: 250-500 MB/s compress, 500-1500 MB/s decompress.
    Snappy,
    /// LZ4 — ultra-fast compression, comparable ratio to Snappy.
    /// Best for latency-sensitive and streaming workloads.
    Lz4,
    /// Zstandard — excellent compression ratio with configurable levels (1-22).
    /// Best for archival, cold storage, and batch analytics.
    Zstd,
    /// Brotli — maximum compression ratio with configurable levels (0-11).
    /// Best for web deployment and network transfer.
    Brotli,
}

impl std::fmt::Display for CompressionAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Snappy => write!(f, "Snappy"),
            Self::Lz4 => write!(f, "LZ4"),
            Self::Zstd => write!(f, "ZSTD"),
            Self::Brotli => write!(f, "Brotli"),
        }
    }
}

impl CompressionAlgorithm {
    /// All supported compression algorithms for enumeration/benchmarks.
    pub const ALL: &'static [CompressionAlgorithm] = &[
        CompressionAlgorithm::None,
        CompressionAlgorithm::Snappy,
        CompressionAlgorithm::Lz4,
        CompressionAlgorithm::Zstd,
        CompressionAlgorithm::Brotli,
    ];
}

/// Compression configuration for Parquet file writing.
///
/// Controls which compression algorithm is used and its level (for
/// algorithms that support configurable compression levels).
///
/// # Examples
///
/// ```rust
/// use dbx_core::storage::compression::{CompressionConfig, CompressionAlgorithm};
///
/// // Default (Snappy)
/// let config = CompressionConfig::default();
///
/// // ZSTD with level 3 (balanced)
/// let config = CompressionConfig::new(CompressionAlgorithm::Zstd).with_level(3);
///
/// // Convert to Parquet compression setting
/// let parquet_compression = config.to_parquet_compression();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressionConfig {
    algorithm: CompressionAlgorithm,
    /// Compression level (only used by ZSTD and Brotli).
    /// - ZSTD: 1-22 (default: 3, recommended: 1-9)
    /// - Brotli: 0-11 (default: 6)
    /// - Snappy/LZ4/None: ignored
    level: Option<u32>,
}

impl Default for CompressionConfig {
    /// Default compression: Snappy (best general-purpose balance).
    fn default() -> Self {
        Self {
            algorithm: CompressionAlgorithm::Snappy,
            level: None,
        }
    }
}

impl CompressionConfig {
    /// Create a new compression config with the specified algorithm.
    pub fn new(algorithm: CompressionAlgorithm) -> Self {
        Self {
            algorithm,
            level: None,
        }
    }

    /// Set the compression level (for ZSTD and Brotli only).
    ///
    /// Levels are clamped to valid ranges:
    /// - ZSTD: 1-22
    /// - Brotli: 0-11
    /// - Others: ignored
    pub fn with_level(mut self, level: u32) -> Self {
        self.level = Some(level);
        self
    }

    /// Get the configured algorithm.
    pub fn algorithm(&self) -> CompressionAlgorithm {
        self.algorithm
    }

    /// Get the configured level (if any).
    pub fn level(&self) -> Option<u32> {
        self.level
    }

    /// Convert to Parquet's `Compression` enum for use in `WriterProperties`.
    pub fn to_parquet_compression(&self) -> Compression {
        match self.algorithm {
            CompressionAlgorithm::None => Compression::UNCOMPRESSED,
            CompressionAlgorithm::Snappy => Compression::SNAPPY,
            CompressionAlgorithm::Lz4 => Compression::LZ4,
            CompressionAlgorithm::Zstd => {
                let level = self.level.map(|l| l.clamp(1, 22) as i32);
                match level {
                    Some(l) => Compression::ZSTD(parquet::basic::ZstdLevel::try_new(l).unwrap()),
                    None => Compression::ZSTD(parquet::basic::ZstdLevel::default()),
                }
            }
            CompressionAlgorithm::Brotli => {
                let level = self.level.map(|l| l.clamp(0, 11));
                match level {
                    Some(l) => {
                        Compression::BROTLI(parquet::basic::BrotliLevel::try_new(l).unwrap())
                    }
                    None => Compression::BROTLI(parquet::basic::BrotliLevel::default()),
                }
            }
        }
    }

    // ===== Convenience constructors for common presets =====

    /// Preset: No compression (fastest I/O, largest files).
    pub fn none() -> Self {
        Self::new(CompressionAlgorithm::None)
    }

    /// Preset: Snappy (default — balanced speed and compression).
    pub fn snappy() -> Self {
        Self::new(CompressionAlgorithm::Snappy)
    }

    /// Preset: LZ4 (ultra-fast, similar ratio to Snappy).
    pub fn lz4() -> Self {
        Self::new(CompressionAlgorithm::Lz4)
    }

    /// Preset: ZSTD with default level (excellent ratio).
    pub fn zstd() -> Self {
        Self::new(CompressionAlgorithm::Zstd)
    }

    /// Preset: ZSTD with specified compression level (1-22).
    pub fn zstd_level(level: u32) -> Self {
        Self::new(CompressionAlgorithm::Zstd).with_level(level)
    }

    /// Preset: Brotli with default level (maximum ratio).
    pub fn brotli() -> Self {
        Self::new(CompressionAlgorithm::Brotli)
    }

    /// Preset: Brotli with specified compression level (0-11).
    pub fn brotli_level(level: u32) -> Self {
        Self::new(CompressionAlgorithm::Brotli).with_level(level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_snappy() {
        let config = CompressionConfig::default();
        assert_eq!(config.algorithm(), CompressionAlgorithm::Snappy);
        assert_eq!(config.level(), None);
    }

    #[test]
    fn parquet_compression_mapping() {
        // None
        let c = CompressionConfig::none().to_parquet_compression();
        assert_eq!(c, Compression::UNCOMPRESSED);

        // Snappy
        let c = CompressionConfig::snappy().to_parquet_compression();
        assert_eq!(c, Compression::SNAPPY);

        // LZ4
        let c = CompressionConfig::lz4().to_parquet_compression();
        assert_eq!(c, Compression::LZ4);

        // ZSTD default
        let c = CompressionConfig::zstd().to_parquet_compression();
        matches!(c, Compression::ZSTD(_));

        // ZSTD with level
        let c = CompressionConfig::zstd_level(9).to_parquet_compression();
        matches!(c, Compression::ZSTD(_));

        // Brotli default
        let c = CompressionConfig::brotli().to_parquet_compression();
        matches!(c, Compression::BROTLI(_));

        // Brotli with level
        let c = CompressionConfig::brotli_level(11).to_parquet_compression();
        matches!(c, Compression::BROTLI(_));
    }

    #[test]
    fn level_clamping() {
        // ZSTD: level 100 → clamped to 22
        let c = CompressionConfig::zstd_level(100).to_parquet_compression();
        matches!(c, Compression::ZSTD(_));

        // Brotli: level 99 → clamped to 11
        let c = CompressionConfig::brotli_level(99).to_parquet_compression();
        matches!(c, Compression::BROTLI(_));
    }

    #[test]
    fn display_names() {
        assert_eq!(format!("{}", CompressionAlgorithm::None), "None");
        assert_eq!(format!("{}", CompressionAlgorithm::Snappy), "Snappy");
        assert_eq!(format!("{}", CompressionAlgorithm::Lz4), "LZ4");
        assert_eq!(format!("{}", CompressionAlgorithm::Zstd), "ZSTD");
        assert_eq!(format!("{}", CompressionAlgorithm::Brotli), "Brotli");
    }

    #[test]
    fn all_algorithms_count() {
        assert_eq!(CompressionAlgorithm::ALL.len(), 5);
    }
}
