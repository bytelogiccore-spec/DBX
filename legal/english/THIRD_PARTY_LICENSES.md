# Third-Party Licenses

DBX uses the following open-source libraries. We are grateful to their maintainers and contributors.

---

## Core Dependencies

### Apache Arrow & Parquet
- **License**: Apache-2.0
- **Version**: 54.x
- **Purpose**: High-performance columnar data format and storage
- **Repository**: https://github.com/apache/arrow-rs

### SQLParser
- **License**: Apache-2.0
- **Version**: 0.52
- **Purpose**: SQL parsing and AST generation
- **Repository**: https://github.com/sqlparser-rs/sqlparser-rs

### s2n-quic
- **License**: Apache-2.0
- **Version**: 1.x
- **Purpose**: QUIC transport for distributed Grid communication
- **Repository**: https://github.com/aws/s2n-quic

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

### Wide
- **License**: MIT OR Apache-2.0 OR Zlib
- **Version**: 0.7
- **Purpose**: Simple SIMD abstraction layer
- **Repository**: https://github.com/Lokathor/wide

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

### Reed-Solomon-Erasure
- **License**: MIT
- **Version**: 6.0
- **Purpose**: Erasure coding for distributed tiering
- **Repository**: https://github.com/darrenldl/reed-solomon-erasure

---

## GPU Acceleration (Optional)

### cudarc (Optional)
- **License**: MIT OR Apache-2.0
- **Version**: 0.19
- **Purpose**: Rust CUDA bindings and GPU acceleration
- **Repository**: https://github.com/coreylowman/cudarc

### Cron & Chrono
- **License**: MIT OR Apache-2.0
- **Version**: 0.12 / 0.4
- **Purpose**: Job scheduling for data lifecycle management
- **Repository**: https://github.com/zslayton/cron

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

### Tokio
- **License**: MIT
- **Version**: 1.x
- **Purpose**: Async runtime and network communication
- **Repository**: https://github.com/tokio-rs/tokio

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
| **MIT OR Apache-2.0** | 14 | Arrow, Parquet, Rayon, AHash, SmallVec, thiserror, cudarc, AES-GCM-SIV, ChaCha20, ZSTD, Serde, Wide, Cron, Chrono |
| **MIT** | 5 | DashMap, Tracing, Bincode, Reed-Solomon, Tokio |
| **Apache-2.0** | 2 | SQLParser, s2n-quic |

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
- Test and benchmark-only dependencies (Criterion, Proptest, rusqlite, redb) have been separated and are excluded from this list.

---

*Last updated: 2026-04-03 (v0.2.0-beta)*
