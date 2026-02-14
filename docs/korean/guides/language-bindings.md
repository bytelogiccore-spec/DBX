---
layout: default
title: 언어 바인딩
nav_order: 7
parent: Guides
---

# 언어 바인딩
{: .no_toc }

DBX는 고성능 임베디드 데이터베이스를 선호하는 개발 환경에서 사용할 수 있도록 다양한 프로그래밍 언어에 대한 공식 바인딩을 제공합니다.
{: .fs-6 .fw-300 }

---

## 🐍 Python
컨텍스트 매니저를 지원하는 고수준 Python 바인딩입니다.

```python
from dbx_py import Database

with Database("my_database.db") as db:
    db.insert("users", b"user:1", b"Alice")
    value = db.get("users", b"user:1")
    print(value.decode('utf-8'))
```

---

## 🔷 C#/.NET
RAII 패턴과 고성능 배치 작업을 지원하는 현대적인 .NET 바인딩입니다.

```csharp
using DBX.Client;
using (var db = new DbxDatabase("./my_database")) {
    db.Insert("users", key, value);
    byte[] result = db.Get("users", key);
}
```

---

## 🔧 C/C++
저수준 C API와 현대적인 C++17 래퍼를 제공합니다.

```cpp
#include "dbx.hpp"
using namespace dbx;

auto db = Database::openInMemory();
db.insert("users", "user:1", "Alice");
```

---

## 🟢 Node.js
N-API를 사용하여 성능을 극대화한 네이티브 Node.js 바인딩입니다.

```javascript
const { Database } = require('dbx-node');
const db = new Database('my_database.db');
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
```

---

## 📊 성능 비교
모든 바인딩은 제로 카피 FFI를 통해 네이티브에 가까운 성능을 제공합니다.

| 언어 | INSERT (1만건) | GET (1만건) | 오버헤드 |
|----------|--------------|-----------|----------|
| **Rust (Core)** | 25.37 ms | 17.28 ms | 0% (기준) |
| **C/C++** | ~26 ms | ~18 ms | ~3% |
| **C#/.NET** | ~27 ms | ~19 ms | ~6% |
| **Python** | ~28 ms | ~20 ms | ~10% |
| **Node.js** | ~29 ms | ~21 ms | ~12% |

---

## 🔗 공통 기능
- **인메모리 모드**: 빠른 임시 저장소
- **파일 기반 영속성**: 안전한 데이터 보관
- **CRUD 및 배치 작업**: 고성능 대량 삽입 지원
- **트랜잭션**: ACID 보장
- **언어별 최적화**: 각 언어의 숙어(Idiomatic)를 따르는 에러 처리

---

## 다음 단계
- [시작하기](../getting-started) — Rust 설치 가이드
- [저장소 계층](storage-layers) — 5계층 아키텍처 이해
- [벤치마크](../benchmarks) — 상세 성능 비교 확인
