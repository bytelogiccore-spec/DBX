---
layout: default
title: .NET (DBX.Dotnet)
parent: 패키지
grand_parent: 한국어
nav_order: 2
---

# .NET — DBX.Dotnet

[![NuGet](https://img.shields.io/nuget/v/DBX.Dotnet.svg)](https://www.nuget.org/packages/DBX.Dotnet)

CsBindgen을 통한 고성능 .NET 바인딩으로 오버헤드 거의 없이 Rust 코어에 접근합니다.

## 설치

```bash
dotnet add package DBX.Dotnet
```

## 빠른 시작

```csharp
using DBX.Dotnet;

// 데이터베이스 열기
using var db = Database.OpenInMemory();

// 삽입
db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());

// 조회
var value = db.Get("users", "user:1"u8.ToArray());
if (value != null)
    Console.WriteLine(Encoding.UTF8.GetString(value)); // Alice

// 삭제
db.Delete("users", "user:1"u8.ToArray());
```

## 파일 기반 데이터베이스

```csharp
using var db = Database.Open("my_database.db");

db.Insert("config", "key"u8.ToArray(), "value"u8.ToArray());
```

## 트랜잭션

```csharp
using var db = Database.OpenInMemory();

using var tx = db.BeginTransaction();
tx.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
tx.Insert("users", "user:2"u8.ToArray(), "Bob"u8.ToArray());
tx.Commit(); // 원자적 배치 쓰기
```

## API 레퍼런스

### Database

| 메서드 | 반환 | 설명 |
|--------|------|------|
| `Database.OpenInMemory()` | `Database` | 인메모리 DB 열기 |
| `Database.Open(path)` | `Database` | 파일 기반 DB 열기 |
| `Insert(table, key, value)` | `void` | 키-값 삽입 |
| `Get(table, key)` | `byte[]?` | 값 조회 (없으면 null) |
| `Delete(table, key)` | `void` | 키 삭제 |
| `BeginTransaction()` | `Transaction` | 트랜잭션 시작 |
| `Dispose()` | `void` | 닫기 및 해제 |

### Transaction

| 메서드 | 반환 | 설명 |
|--------|------|------|
| `Insert(table, key, value)` | `void` | 버퍼링된 삽입 |
| `Delete(table, key)` | `void` | 버퍼링된 삭제 |
| `Commit()` | `void` | 모든 작업 적용 |

## 요구 사항

- .NET Standard 2.0+
  - .NET Framework 4.6.1+
  - .NET Core 2.0+
  - .NET 5, 6, 7, 8, 9+
- Windows x64
