# DBX vs SQLite - Performance Comparison Results

## Overview

DBX와 SQLite의 성능을 비교한 벤치마크 결과입니다. 모든 테스트는 10,000개의 INSERT, GET, DELETE 작업을 수행합니다.

## .NET (C#) Benchmark

**환경:** Visual Studio 2022, .NET 9.0, CsBindgen Native

**결과 (10,000 ops):**

| Operation | DBX Native (CsBindgen) | SQLite (In-Memory) | **DBX 우위** |
|-----------|------------------------|-------------------|-------------|
| **INSERT** | **161,303 ops/sec** (62ms) | 43,043 ops/sec (232ms) | **3.75배 빠름** 🚀 |
| **GET** | **631,413 ops/sec** (16ms) | 41,887 ops/sec (239ms) | **15.07배 빠름** 🔥🔥🔥 |
| **DELETE** | **292,394 ops/sec** (34ms) | 90,769 ops/sec (110ms) | **3.22배 빠름** 🚀 |

**분석:**
- **GET 작업에서 압도적 우위**: SQLite 대비 **15배 빠름!**
- **INSERT 작업에서 3.75배 빠름**: CsBindgen + 배치 처리 최적화
- **DELETE 작업에서 3.22배 빠름**: 트랜잭션 레벨 최적화
- **CsBindgen의 제로 오버헤드**: 네이티브 호출로 최고 성능
- **모든 작업에서 SQLite를 압도**: C#에서 최고 성능 데이터베이스!

**실행 방법:**
```bash
cd lang/dotnet/DBX.Benchmark.Native
dotnet run -c Release
```

## Node.js Benchmark

**환경:** Node.js v24.12.0, napi-rs

### 🏆 배치 API 최적화 (최종)

**결과 (10,000 ops):**

| Operation | DBX Native (Batch API) | better-sqlite3 (In-Memory) | **DBX 우위** |
|-----------|------------------------|---------------------------|-------------|
| **INSERT** | **406,149 ops/sec** (25ms) | 291,982 ops/sec (34ms) | **1.39배 빠름** 🔥 |
| **GET** | **346,374 ops/sec** (29ms) | 287,496 ops/sec (35ms) | **1.20배 빠름** 🔥 |
| **DELETE** | **590,824 ops/sec** (17ms) | 534,979 ops/sec (19ms) | **1.10배 빠름** 🔥 |

**분석:**
- **🏆 모든 작업에서 better-sqlite3를 이김!**
- **INSERT 성능 압도**: DBX 406K vs SQLite 291K (**1.39배 빠름**)
- **GET 성능 우수**: DBX 346K vs SQLite 287K (**1.20배 빠름**)
- **DELETE 성능 우수**: DBX 590K vs SQLite 534K (**1.10배 빠름**)
- **배치 API의 위력**: 1번의 네이티브 호출로 극대화된 성능

### 트랜잭션 방식 (이전)

**결과 (10,000 ops):**

| Operation | DBX Native (Transaction) | better-sqlite3 | 비교 |
|-----------|-------------------------|----------------|------|
| INSERT | 285,777 ops/sec | 392,828 ops/sec | SQLite 1.37배 빠름 |
| GET | 320,068 ops/sec | 350,992 ops/sec | SQLite 1.10배 빠름 |
| DELETE | 445,549 ops/sec | 476,792 ops/sec | SQLite 1.07배 빠름 |

### 최적화 효과

| Operation | Before (Transaction) | After (Batch API) | 개선율 |
|-----------|---------------------|-------------------|--------|
| INSERT | 285K ops/sec | **406K ops/sec** | **+42%** 🔥 |
| GET | 320K ops/sec | **346K ops/sec** | **+8%** ✅ |
| DELETE | 445K ops/sec | **590K ops/sec** | **+33%** 🔥 |

**핵심 개선 사항:**
- ✅ INSERT 42% 향상 (285K → 406K)
- ✅ DELETE 33% 향상 (445K → 590K)
- ✅ GET 8% 향상 (320K → 346K)

**실행 방법:**
```bash
# 배치 API 벤치마크 (최신)
cd lang/nodejs
node benchmarks/benchmark_batch.js

# 트랜잭션 벤치마크 (이전)
node benchmarks/benchmark.js
```

---

## Python Benchmark

**환경:** Python 3.12, PyO3 Native

**결과 (10,000 ops):**

| Operation | DBX Native (PyO3) | SQLite (In-Memory) | **DBX 우위** |
|-----------|-------------------|-------------------|-------------|
| **INSERT** | **469,799 ops/sec** (21ms) | 390,854 ops/sec (26ms) | **1.20배 빠름** ✅ |
| **GET** | **986,514 ops/sec** (10ms) | 379,327 ops/sec (26ms) | **2.60배 빠름** 🔥 |
| **DELETE** | **624,231 ops/sec** (16ms) | 451,728 ops/sec (22ms) | **1.38배 빠름** ✅ |

**분석:**
- **GET 작업에서 압도적 우위**: SQLite 대비 **2.6배 빠름!**
- **모든 작업에서 SQLite보다 빠름**: INSERT 1.20배, DELETE 1.38배
- **PyO3 네이티브 확장**: ctypes FFI 오버헤드 제거
- **트랜잭션 배치 처리**: 자동 최적화로 성능 극대화

**실행 방법:**
```bash
py lang/python/benchmarks/benchmark_native.py
```

## C++ Benchmark

**환경:** MinGW g++ 15.2.0, C++17, MSYS2 SQLite3

**결과 (10,000 ops):**

| Operation | DBX (In-Memory, FFI Transaction) | SQLite (In-Memory) | Winner |
|-----------|----------------------------------|-------------------|--------|
| **INSERT** | 296,755 ops/sec<br>(0.0337s) | **586,125 ops/sec**<br>(0.0171s) | SQLite (1.97x) |
| **GET** | **910,921 ops/sec**<br>(0.0110s) | 874,791 ops/sec<br>(0.0114s) | DBX (1.04x) |
| **DELETE** | 489,922 ops/sec<br>(0.0204s) | **833,764 ops/sec**<br>(0.0120s) | SQLite (1.70x) |

**분석:**
- **GET 작업에서 DBX가 약간 빠름** (1.04배)
- SQLite는 INSERT와 DELETE에서 더 빠름 (1.97x, 1.70x)
- C++ 네이티브 성능으로 Python보다 훨씬 빠름
- **경쟁력 있는 성능**: GET은 DBX 우위, INSERT/DELETE는 SQLite 우위

**빌드 방법 (Windows MinGW):**
```bash
# 1. MSYS2 MinGW 설치
winget install -e --id MSYS2.MSYS2

# 2. MinGW gcc와 SQLite3 설치
C:\msys64\usr\bin\bash.exe -lc "pacman -S --noconfirm mingw-w64-x86_64-gcc mingw-w64-x86_64-sqlite3"

# 3. Rust MinGW 타겟 추가 및 빌드
rustup target add x86_64-pc-windows-gnu
$env:PATH = "C:\msys64\ucrt64\bin;C:\msys64\mingw64\bin;$env:PATH"
cargo build --release --target x86_64-pc-windows-gnu -p dbx-ffi

# 4. C++ 벤치마크 컴파일
C:\msys64\mingw64\bin\g++.exe -o lang\cpp\benchmarks\benchmark_vs_sqlite.exe lang\cpp\benchmarks\benchmark_vs_sqlite.cpp -I lang\c\include -L target\x86_64-pc-windows-gnu\release -l dbx_ffi -L C:\msys64\mingw64\lib -l sqlite3 -std=c++17

# 5. 실행
$env:PATH = "d:\ByteLogicCore\DBX\target\x86_64-pc-windows-gnu\release;C:\msys64\mingw64\bin;$env:PATH"
.\lang\cpp\benchmarks\benchmark_vs_sqlite.exe
```

**Linux/Mac:**
```bash
cd lang/cpp/benchmarks
make -f Makefile.sqlite
./benchmark_vs_sqlite
```

## 결론

### 언어별 성능 비교

**C# (.NET)** - 트랜잭션 + 배치 API 사용:
- ✅ **DBX가 SQLite Disk보다 빠름** (INSERT, GET 모두)
- ✅ **GET 작업에서 압도적 우위** (222K ops/sec vs SQLite Disk 12K ops/sec)
- ✅ **데이터 무결성 검증 통과**

**Python** - 개별 작업 (트랜잭션 미사용):
- ❌ SQLite가 더 빠름 (트랜잭션 일괄 처리)
- ❌ FFI 호출 오버헤드 존재

### 핵심 인사이트

**트랜잭션의 중요성:**
- C#은 `BeginTransaction()` + 배치 insert로 SQLite Disk를 능가
- Python은 개별 작업으로 인해 SQLite보다 느림
- **결론**: DBX도 트랜잭션을 사용하면 경쟁력 있음!

### SQLite가 더 빠른 경우:
- **트랜잭션 일괄 처리**: SQLite는 BEGIN/COMMIT으로 여러 작업을 묶어서 처리
- **최적화된 C 구현**: SQLite는 수십 년간 최적화된 C 코드
- **Python 네이티브 바인딩**: sqlite3 모듈은 Python에 내장되어 있음

### DBX의 장점:
- **간단한 API**: Key-Value 스토어로 더 직관적
- **타입 안전성**: Rust의 타입 시스템 활용
- **메모리 안전성**: Rust의 메모리 안전성 보장
- **확장성**: 분산 시스템, 스트리밍 등 추가 기능
- **GET 성능**: C#에서 SQLite Disk 대비 18.4배 빠름

### 개선 방안:
1. **Python에 트랜잭션 API 추가**: `begin_transaction()`, `commit()`
2. **배치 API 추가**: `insert_batch()`, `get_batch()` 등
3. **FFI 오버헤드 감소**: 더 효율적인 데이터 전달

## 참고

- **C# 벤치마크는 트랜잭션을 사용**하여 SQLite Disk를 능가합니다
- **Python 벤치마크는 개별 작업**으로 인해 SQLite보다 느립니다
- DBX는 아직 초기 단계이며, 성능 최적화가 진행 중입니다
- SQLite는 30년 이상의 최적화가 적용된 성숙한 데이터베이스입니다
- 벤치마크는 단순 CRUD 작업만 측정하며, 실제 사용 사례는 다를 수 있습니다
