# 제3자 라이선스 (Third-Party Licenses)

DBX는 다음과 같은 오픈 소스 라이브러리를 사용합니다. 각 프로젝트의 유지보수자와 기여자분들께 깊은 감사를 드립니다.

---

## 핵심 의존성 (Core Dependencies)

### Apache Arrow & Parquet
- **라이선스**: Apache-2.0
- **버전**: 54.x
- **용도**: 고성능 컬럼형 데이터 포맷 및 저장소
- **저장소**: https://github.com/apache/arrow-rs

### Sled
- **라이선스**: MIT OR Apache-2.0
- **버전**: 0.34
- **용도**: 임베디드 키-값 저장소 엔진
- **저장소**: https://github.com/spacejam/sled

### SQLParser
- **라이선스**: Apache-2.0
- **버전**: 0.52
- **용도**: SQL 파싱 및 AST 생성
- **저장소**: https://github.com/sqlparser-rs/sqlparser-rs

---

## 성능 및 동시성 (Performance & Concurrency)

### Rayon
- **라이선스**: MIT OR Apache-2.0
- **버전**: 1.10
- **용도**: 데이터 병렬 처리 라이브러리
- **저장소**: https://github.com/rayon-rs/rayon

### DashMap
- **라이선스**: MIT
- **버전**: 6.1
- **용도**: 동시성 HashMap
- **저장소**: https://github.com/xacrimon/dashmap

### LRU
- **라이선스**: MIT
- **버전**: 0.12
- **용도**: LRU 캐시 구현
- **저장소**: https://github.com/jeromefroe/lru-rs

### AHash
- **라이선스**: MIT OR Apache-2.0
- **버전**: 0.8
- **용도**: 고성능 해싱 알고리즘
- **저장소**: https://github.com/tkaitchuck/aHash

### SmallVec
- **라이선스**: MIT OR Apache-2.0
- **버전**: 1.15
- **용도**: 소규모 데이터를 위한 스택 할당 벡터
- **저장소**: https://github.com/servo/rust-smallvec

---

## 암호화 및 압축 (Cryptography & Compression)

### AES-GCM-SIV
- **라이선스**: MIT OR Apache-2.0
- **버전**: 0.11
- **용도**: 인증된 암호화
- **저장소**: https://github.com/RustCrypto/AEADs

### ChaCha20-Poly1305
- **라이선스**: MIT OR Apache-2.0
- **버전**: 0.10
- **용도**: 인증된 암호화
- **저장소**: https://github.com/RustCrypto/AEADs

### ZSTD
- **라이선스**: MIT OR Apache-2.0
- **버전**: 0.13
- **용도**: 고성능 데이터 압축
- **저장소**: https://github.com/gyscos/zstd-rs

### Brotli
- **라이선스**: MIT OR Apache-2.0
- **버전**: 7.0
- **용도**: 압축 알고리즘
- **저장소**: https://github.com/dropbox/rust-brotli

---

## GPU 가속 (Optional)

### cudarc
- **라이선스**: MIT OR Apache-2.0
- **버전**: 0.12
- **용도**: Rust CUDA 바인딩
- **저장소**: https://github.com/coreylowman/cudarc

---

## 에러 처리 및 로깅 (Error Handling & Logging)

### thiserror
- **라이선스**: MIT OR Apache-2.0
- **버전**: 2.0
- **용도**: 에러 타입 정의 및 직관화
- **저장소**: https://github.com/dtolnay/thiserror

### Tracing
- **라이선스**: MIT
- **버전**: 0.1
- **용도**: 애플리케이션 수준 트레이싱 및 진단
- **저장소**: https://github.com/tokio-rs/tracing

### Tracing Subscriber
- **라이선스**: MIT
- **버전**: 0.3
- **용도**: 트레이싱 이벤트 수집 및 포맷팅
- **저장소**: https://github.com/tokio-rs/tracing

---

## 데이터 직렬화 (Serialization)

### Bincode
- **라이선스**: MIT
- **버전**: 1.3
- **용도**: WAL용 바이너리 직렬화
- **저장소**: https://github.com/bincode-org/bincode

### Serde
- **라이선스**: MIT OR Apache-2.0
- **버전**: 1.0
- **용도**: 메타 직렬화 프레임워크
- **저장소**: https://github.com/serde-rs/serde

---

## 라이선스 요약 (License Summary)

| 라이선스 유형 | 개수 | 주요 라이브러리 |
|--------------|-------|---------------|
| **MIT OR Apache-2.0** | 12 | Arrow, Parquet, Sled, Rayon, AHash, SmallVec, thiserror, cudarc, AES-GCM-SIV, ChaCha20, ZSTD, Brotli, Serde |
| **MIT** | 3 | DashMap, Tracing, Bincode |
| **Apache-2.0** | 1 | SQLParser |

---

## 라이선스 본문 링크

### MIT License
참조: https://opensource.org/licenses/MIT

### Apache License 2.0
참조: https://www.apache.org/licenses/LICENSE-2.0

---

## 참고 사항

- 모든 의존성은 성능, 보안 및 안정성을 기준으로 신중하게 선택되었습니다.
- 모든 의존성에 대해 정기적인 보안 감사가 수행됩니다.
- 버전 업데이트는 통합 전 철저히 추적 및 테스트됩니다.
- 테스트 전용 의존성(Criterion, Proptest, rusqlite, redb 등)은 본 목록에서 제외되었습니다.

---

*최종 업데이트: 2026-02-13 (v0.0.1-beta)*
