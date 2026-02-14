---
layout: default
title: 설치
parent: C/C++ (dbx-ffi)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 1
---

# 설치

## 파일 다운로드

### GitHub Releases에서 다운로드

1. [Releases 페이지](https://github.com/bytelogiccore-spec/DBX/releases) 방문
2. 최신 버전의 `dbx-ffi-windows-x64.zip` 다운로드
3. 압축 해제

### 포함된 파일

```
dbx-ffi-windows-x64/
├── include/
│   ├── dbx.h         (C 헤더)
│   └── dbx.hpp       (C++ 래퍼)
├── lib/
│   ├── dbx_ffi.dll   (동적 라이브러리)
│   ├── dbx_ffi.lib   (임포트 라이브러리)
│   └── dbx_ffi.a     (정적 라이브러리, MinGW)
└── README.md
```

## Visual Studio 설정

### 프로젝트 속성 설정

1. **C/C++ → 일반 → 추가 포함 디렉터리**
   ```
   D:\dbx-ffi\include
   ```

2. **링커 → 일반 → 추가 라이브러리 디렉터리**
   ```
   D:\dbx-ffi\lib
   ```

3. **링커 → 입력 → 추가 종속성**
   ```
   dbx_ffi.lib
   ```

4. **DLL 복사** (빌드 후 이벤트)
   ```cmd
   copy "D:\dbx-ffi\lib\dbx_ffi.dll" "$(OutDir)"
   ```

### 예제 프로젝트

```xml
<?xml version="1.0" encoding="utf-8"?>
<Project DefaultTargets="Build" xmlns="http://schemas.microsoft.com/developer/msbuild/2003">
  <PropertyGroup>
    <ConfigurationType>Application</ConfigurationType>
  </PropertyGroup>
  
  <ItemDefinitionGroup>
    <ClCompile>
      <AdditionalIncludeDirectories>D:\dbx-ffi\include</AdditionalIncludeDirectories>
    </ClCompile>
    <Link>
      <AdditionalLibraryDirectories>D:\dbx-ffi\lib</AdditionalLibraryDirectories>
      <AdditionalDependencies>dbx_ffi.lib;%(AdditionalDependencies)</AdditionalDependencies>
    </Link>
  </ItemDefinitionGroup>
</Project>
```

## CMake 설정

### CMakeLists.txt

```cmake
cmake_minimum_required(VERSION 3.15)
project(MyApp)

# DBX FFI 경로 설정
set(DBX_FFI_DIR "D:/dbx-ffi")

# 헤더 경로
include_directories(${DBX_FFI_DIR}/include)

# 라이브러리 경로
link_directories(${DBX_FFI_DIR}/lib)

# 실행 파일
add_executable(myapp main.c)

# 링크
target_link_libraries(myapp dbx_ffi)

# DLL 복사 (Windows)
if(WIN32)
    add_custom_command(TARGET myapp POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
        "${DBX_FFI_DIR}/lib/dbx_ffi.dll"
        $<TARGET_FILE_DIR:myapp>)
endif()
```

### 빌드

```bash
mkdir build
cd build
cmake ..
cmake --build .
```

## GCC/MinGW 설정

### 컴파일 (C)

```bash
gcc -I./include -L./lib main.c -ldbx_ffi -o myapp.exe
```

### 컴파일 (C++)

```bash
g++ -std=c++11 -I./include -L./lib main.cpp -ldbx_ffi -o myapp.exe
```

### Makefile

```makefile
CC = gcc
CXX = g++
CFLAGS = -I./include
LDFLAGS = -L./lib -ldbx_ffi

myapp: main.c
	$(CC) $(CFLAGS) main.c $(LDFLAGS) -o myapp.exe

myapp_cpp: main.cpp
	$(CXX) -std=c++11 $(CFLAGS) main.cpp $(LDFLAGS) -o myapp.exe

clean:
	del myapp.exe
```

## Clang 설정

```bash
clang -I./include -L./lib main.c -ldbx_ffi -o myapp.exe
```

## 설치 확인

### C 테스트

```c
#include "dbx.h"
#include <stdio.h>

int main() {
    DbxDatabase* db = dbx_open_in_memory();
    if (db) {
        printf("DBX FFI loaded successfully!\n");
        dbx_close(db);
        return 0;
    }
    return 1;
}
```

### C++ 테스트

```cpp
#include "dbx.hpp"
#include <iostream>

int main() {
    try {
        auto db = dbx::Database::openInMemory();
        std::cout << "DBX FFI loaded successfully!" << std::endl;
        return 0;
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
}
```

## 문제 해결

### DLL을 찾을 수 없음

**원인**: `dbx_ffi.dll`이 실행 파일과 같은 폴더에 없음

**해결**:
1. DLL을 실행 파일 폴더로 복사
2. 또는 시스템 PATH에 DLL 경로 추가

### 링크 오류 (LNK2019)

**원인**: 임포트 라이브러리 경로 오류

**해결**:
```
링커 → 일반 → 추가 라이브러리 디렉터리: D:\dbx-ffi\lib
링커 → 입력 → 추가 종속성: dbx_ffi.lib
```

### MinGW에서 undefined reference

**원인**: 정적 라이브러리 사용 필요

**해결**:
```bash
gcc -I./include main.c ./lib/dbx_ffi.a -lws2_32 -lbcrypt -luserenv -o myapp.exe
```

## 다음 단계

- [빠른 시작](quickstart) - 첫 프로그램 작성
- [C API](c-api) - C 함수 레퍼런스
- [C++ API](cpp-api) - C++ 클래스 레퍼런스
