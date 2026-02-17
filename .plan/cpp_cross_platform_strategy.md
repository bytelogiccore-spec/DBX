# C++ 크로스 플랫폼 배포 전략

## 📋 목표

**단일 통합 패키지로 모든 플랫폼 지원**

```
dbx-native-v0.0.6.zip
├── windows-x64/
│   ├── bin/dbx_ffi.dll
│   └── lib/dbx_ffi.lib
├── linux-x64/
│   └── lib/
│       ├── libdbx_ffi.so
│       └── libdbx_ffi.a
├── linux-arm64/
│   └── lib/
│       ├── libdbx_ffi.so
│       └── libdbx_ffi.a
├── linux-arm32/
│   └── lib/
│       ├── libdbx_ffi.so
│       └── libdbx_ffi.a
├── macos-arm64/
│   └── lib/
│       ├── libdbx_ffi.dylib
│       └── libdbx_ffi.a
├── include/
│   ├── dbx.h
│   └── dbx.hpp
├── examples/
│   └── basic_usage.cpp
└── README.md
```

---

## 🎯 지원 플랫폼

| 플랫폼 | 아키텍처 | 동적 라이브러리 | 정적 라이브러리 |
|--------|----------|----------------|----------------|
| Windows | x64 | `dbx_ffi.dll` | `dbx_ffi.lib` |
| Linux | x64 | `libdbx_ffi.so` | `libdbx_ffi.a` |
| Linux | ARM64 | `libdbx_ffi.so` | `libdbx_ffi.a` |
| Linux | ARM32 | `libdbx_ffi.so` | `libdbx_ffi.a` |
| macOS | ARM64 | `libdbx_ffi.dylib` | `libdbx_ffi.a` |

---

## 🔧 GitHub Actions 구현

### 멀티 플랫폼 빌드 매트릭스

```yaml
strategy:
  matrix:
    include:
      - os: windows-latest
        target: x86_64-pc-windows-msvc
        platform: windows-x64
        
      - os: ubuntu-latest
        target: x86_64-unknown-linux-gnu
        platform: linux-x64
        
      - os: ubuntu-latest
        target: aarch64-unknown-linux-gnu
        platform: linux-arm64
        
      - os: ubuntu-latest
        target: armv7-unknown-linux-gnueabihf
        platform: linux-arm32
        
      - os: macos-latest
        target: aarch64-apple-darwin
        platform: macos-arm64
```

---

## 📦 사용자 사용법

### CMake 통합

```cmake
set(DBX_ROOT "/path/to/dbx-native-v0.0.6")

# 플랫폼 자동 감지
if(WIN32)
    set(DBX_LIB_DIR "${DBX_ROOT}/windows-x64")
    set(DBX_LIB "dbx_ffi")
elseif(APPLE)
    set(DBX_LIB_DIR "${DBX_ROOT}/macos-arm64/lib")
    set(DBX_LIB "dbx_ffi")
else()
    # Linux - 아키텍처 감지
    if(CMAKE_SYSTEM_PROCESSOR MATCHES "aarch64")
        set(DBX_LIB_DIR "${DBX_ROOT}/linux-arm64/lib")
    elseif(CMAKE_SYSTEM_PROCESSOR MATCHES "arm")
        set(DBX_LIB_DIR "${DBX_ROOT}/linux-arm32/lib")
    else()
        set(DBX_LIB_DIR "${DBX_ROOT}/linux-x64/lib")
    endif()
    set(DBX_LIB "dbx_ffi")
endif()

include_directories("${DBX_ROOT}/include")
link_directories("${DBX_LIB_DIR}")
target_link_libraries(myapp ${DBX_LIB})
```

---

## ✅ 장점

- ✅ **단일 다운로드**: 모든 플랫폼 포함
- ✅ **CMake 친화적**: 자동 플랫폼 감지
- ✅ **크로스 컴파일 지원**: 한 번에 여러 타겟 빌드
- ✅ **관리 간편**: 릴리스 에셋 1개만 관리

---

## 📅 구현 일정

- **Week 1**: GitHub Actions 멀티 플랫폼 빌드 구현
- **Week 2**: 통합 패키지 생성 스크립트 작성
- **Week 3**: 문서화 및 예제 코드 작성
