# DBX C# 클라이언트 퀵스타트 가이드

DBX는 고성능 5-Tier Hybrid Storage 엔진을 C# 환경에서 사용할 수 있도록 최적화된 클라이언트를 제공합니다.

## 1. 데이터베이스 연결 및 기본 CRUD

가장 기본적인 데이터 삽입 및 조회 방법입니다.

```csharp
using DBX.Client;
using System.Text;

// 1. 데이터베이스 열기 (디렉토리 경로)
using (var db = new DbxDatabase("./my_database"))
{
    var key = Encoding.UTF8.GetBytes("user:1");
    var value = Encoding.UTF8.GetBytes("Alice");

    // 2. 데이터 삽입 (Fast Insert)
    db.Insert("users", key, value);

    // 3. 데이터 조회
    byte[] result = db.Get("users", key);
    if (result != null)
    {
        Console.WriteLine($"Found: {Encoding.UTF8.GetString(result)}");
    }
}
```

## 2. 성능 극대화: 트랜잭션 및 배치 삽입

DBX의 진정한 성능은 트랜잭션을 통한 배치 삽입에서 나옵니다. 내부적으로 고정 배열과 마샬링 최적화를 사용하여 수십만 OPS를 처리할 수 있습니다.

```csharp
using (var db = new DbxDatabase("./perf_db"))
{
    // 트랜잭션 시작 (자동으로 IDisposable을 통한 커밋 지원)
    using (var tx = db.BeginTransaction())
    {
        for (int i = 0; i < 10000; i++)
        {
            tx.Insert("bench", 
                Encoding.UTF8.GetBytes($"key_{i}"), 
                Encoding.UTF8.GetBytes($"value_{i}"));
        }
        
        // 커밋 시점에 모든 데이터가 단일 FFI 호출로 Rust 엔진에 전달됩니다.
        tx.Commit();
    }
}
```

## 3. 내구성 설정 (Durability Level)

애플리케이션의 특성에 따라 디스크 동기화 수준을 조절하여 성능을 추가로 높일 수 있습니다.

*   `Full`: 매 쓰기/커밋마다 WAL을 디스크에 물리적으로 동기화합니다. (가장 안전함)
*   `Lazy`: WAL을 백그라운드 레이턴시로 동기화합니다. 쓰기 성능이 대폭 향상됩니다. (SQLite보다 빠름)
*   `None`: WAL을 기록하지 않습니다. (최고 성능, 휘발성 가능성)

```csharp
using (var db = new DbxDatabase("./high_perf_db"))
{
    // 성능을 위해 Lazy 또는 None 설정 (기본값은 Full)
    db.SetDurability(DurabilityLevel.Lazy);

    using (var tx = db.BeginTransaction())
    {
        // ... 대량의 데이터 삽입 ...
        tx.Commit();
    }
}
```

## 4. 인메모리(In-Memory) 모드

테스트나 임시 데이터 처리를 위해 디스크 저장 없이 메모리에서만 작동하는 모드를 지원합니다.

```csharp
// 인메모리 데이터베이스 생성
using (var db = DbxDatabase.CreateInMemory())
{
    db.Insert("temp", b"key", b"value");
    // ...
}
```

## 5. 성능 비교 (Performance)

10,000건의 트랜잭션 처리 기준 벤치마크 결과입니다. (Windows 네이티브 환경 측정)

| 항목 | **DBX (Disk + Lazy)** | **DBX (In-Memory + Lazy)** | SQLite (Disk) | SQLite (In-Memory) |
| :--- | :--- | :--- | :--- | :--- |
| **Insert (10k)** | **47ms** (211k ops) | **55ms** (181k ops) | 72ms (137k ops) | 21ms (467k ops) |
| **Get (10k)** | **23ms** (433k ops) 🏆 | **23ms** (422k ops) 🏆 | 466ms (21k ops) | 63ms (156k ops) |

> **Tip**: DBX는 특히 **조회(Get)** 성능에서 SQLite 대비 **최대 20배** 이상의 압도적인성능 우위를 보입니다. 디스크 기반임에도 SQLite의 인메모리 조회 성능조차 2.7배 이상 앞지릅니다.

## 6. 주의사항

1.  **IDisposable**: `DbxDatabase`와 `DbxTransaction`은 비관리 리소스(Rust 엔진 핸들)를 포함하므로 반드시 `using` 구문이나 `Dispose()` 호출을 통해 자원을 해제해야 합니다.
2.  **Thread-Safety**: `DbxDatabase` 인스턴스는 멀티스레드 환경에서 안전하게 공유 가능합니다. 단, `DbxTransaction`은 단일 스레드에서 사용하는 것이 권장됩니다.
3.  **Key-Value 형식**: 모든 데이터는 `byte[]` 형태로 처리됩니다. 복잡한 객체는 JSON이나 Protobuf, FlatBuffers 등으로 직렬화하여 관리하세요.
