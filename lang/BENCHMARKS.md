# DBX Language Bindings - Benchmarks

각 언어 바인딩에 대한 성능 벤치마크입니다.

## 벤치마크 시나리오

모든 벤치마크는 동일한 시나리오를 측정합니다:
- **INSERT**: 10,000개의 키-값 쌍 삽입
- **GET**: 10,000개의 키 조회
- **DELETE**: 10,000개의 키 삭제

## Python 벤치마크

```bash
cd lang/python
python benchmarks/benchmark_crud.py
```

## Node.js 벤치마크

```bash
cd lang/nodejs
npm install  # 처음 한 번만
node benchmarks/benchmark_crud.js
```

## C 벤치마크

```bash
cd lang/c/benchmarks
make
./benchmark_crud
```

## C++ 벤치마크

```bash
cd lang/cpp/benchmarks
make
./benchmark_crud
```

## Windows (MSVC)

Windows에서 gcc/make가 없는 경우:

### C 벤치마크
```cmd
cl /O2 /I ..\include benchmark_crud.c /link /LIBPATH:..\..\..\target\release dbx_ffi.lib ws2_32.lib userenv.lib bcrypt.lib
benchmark_crud.exe
```

### C++ 벤치마크
```cmd
cl /O2 /std:c++17 /I ..\include /I ..\..\c\include benchmark_crud.cpp /link /LIBPATH:..\..\..\target\release dbx_ffi.lib ws2_32.lib userenv.lib bcrypt.lib
benchmark_crud.exe
```

## 결과 예시

```
============================================================
DBX C Bindings - Performance Benchmark
============================================================

Running benchmarks with 10,000 operations...

Benchmarking INSERT...
  Time: 0.1234s
  Throughput: 81,037 ops/sec
  Latency: 0.0123 ms/op

Benchmarking GET...
  Time: 0.0987s
  Throughput: 101,317 ops/sec
  Latency: 0.0099 ms/op

Benchmarking DELETE...
  Time: 0.1045s
  Throughput: 95,694 ops/sec
  Latency: 0.0104 ms/op

============================================================
Benchmark completed!
============================================================
```

## 주의사항

- FFI 라이브러리를 먼저 빌드해야 합니다: `cargo build -p dbx-ffi --release`
- 벤치마크는 in-memory 데이터베이스를 사용합니다
- 결과는 시스템 성능에 따라 다를 수 있습니다
