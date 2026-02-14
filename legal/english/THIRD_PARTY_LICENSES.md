# Third-Party Licenses

DBX uses the following open-source libraries. We are grateful to their maintainers and contributors.

---

## Core Dependencies

### Apache Arrow & Parquet
- **License**: Apache-2.0
- **Version**: 54.x
- **Purpose**: High-performance columnar data format and storage
- **Repository**: https://github.com/apache/arrow-rs

### Sled
- **License**: MIT OR Apache-2.0
- **Version**: 0.34
- **Purpose**: Embedded key-value storage engine
- **Repository**: https://github.com/spacejam/sled

### SQLParser
- **License**: Apache-2.0
- **Version**: 0.52
- **Purpose**: SQL parsing and AST generation
- **Repository**: https://github.com/sqlparser-rs/sqlparser-rs

---

## Performance & Concurrency

### Rayon
- **License**: MIT OR Apache-2.0
- **Version**: 1.10
- **Purpose**: Data parallelism library
- **Repository**: https://github.com/rayon-rs/rayon

### DashMap
- **License**: MIT
- **Version**: 6.1
- **Purpose**: Concurrent HashMap
- **Repository**: https://github.com/xacrimon/dashmap

### LRU
- **License**: MIT
- **Version**: 0.12
- **Purpose**: LRU cache implementation
- **Repository**: https://github.com/jeromefroe/lru-rs

### AHash
- **License**: MIT OR Apache-2.0
- **Version**: 0.8
- **Purpose**: High-performance hashing algorithm
- **Repository**: https://github.com/tkaitchuck/aHash

### SmallVec
- **License**: MIT OR Apache-2.0
- **Version**: 1.15
- **Purpose**: Stack-allocated vectors for small data
- **Repository**: https://github.com/servo/rust-smallvec

---

## Cryptography & Compression

### AES-GCM-SIV
- **License**: MIT OR Apache-2.0
- **Version**: 0.11
- **Purpose**: Authenticated encryption
- **Repository**: https://github.com/RustCrypto/AEADs

### ChaCha20-Poly1305
- **License**: MIT OR Apache-2.0
- **Version**: 0.10
- **Purpose**: Authenticated encryption
- **Repository**: https://github.com/RustCrypto/AEADs

### ZSTD
- **License**: MIT OR Apache-2.0
- **Version**: 0.13
- **Purpose**: High-performance compression
- **Repository**: https://github.com/gyscos/zstd-rs

### Brotli
- **License**: MIT OR Apache-2.0
- **Version**: 7.0
- **Purpose**: Compression algorithm
- **Repository**: https://github.com/dropbox/rust-brotli

---

## GPU Acceleration (Optional)

### cudarc
- **License**: MIT OR Apache-2.0
- **Version**: 0.12
- **Purpose**: Rust CUDA bindings
- **Repository**: https://github.com/coreylowman/cudarc

---

## Error Handling & Logging

### thiserror
- **License**: MIT OR Apache-2.0
- **Version**: 2.0
- **Purpose**: Ergonomic error type derivation
- **Repository**: https://github.com/dtolnay/thiserror

### Tracing
- **License**: MIT
- **Version**: 0.1
- **Purpose**: Application-level tracing and diagnostics
- **Repository**: https://github.com/tokio-rs/tracing

### Tracing Subscriber
- **License**: MIT
- **Version**: 0.3
- **Purpose**: Tracing event collection and formatting
- **Repository**: https://github.com/tokio-rs/tracing

---

## Serialization

### Bincode
- **License**: MIT
- **Version**: 1.3
- **Purpose**: Binary serialization for WAL
- **Repository**: https://github.com/bincode-org/bincode

### Serde
- **License**: MIT OR Apache-2.0
- **Version**: 1.0
- **Purpose**: Serialization framework
- **Repository**: https://github.com/serde-rs/serde

---

## License Summary

| License Type | Count | Key Libraries |
|--------------|-------|---------------|
| **MIT OR Apache-2.0** | 12 | Arrow, Parquet, Sled, Rayon, AHash, SmallVec, thiserror, cudarc, AES-GCM-SIV, ChaCha20, ZSTD, Brotli, Serde |
| **MIT** | 3 | DashMap, Tracing, Bincode |
| **Apache-2.0** | 1 | SQLParser |

---

## Full License Texts

### MIT License
See: https://opensource.org/licenses/MIT

### Apache License 2.0
See: https://www.apache.org/licenses/LICENSE-2.0

---

## Notes

- All dependencies are carefully selected for performance, security, and reliability
- Regular security audits are performed on all dependencies
- Version updates are tracked and tested before integration
- Test-only dependencies (Criterion, Proptest, rusqlite, redb) are excluded from this list

---

*Last updated: 2026-02-13 (v0.0.1-beta)*
