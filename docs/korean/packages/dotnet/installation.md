---
layout: default
title: 설치
parent: .NET (DBX.Dotnet)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 1
---

# 설치

## NuGet에서 설치

### .NET CLI

```bash
dotnet add package DBX.Dotnet
```

### Package Manager Console

```powershell
Install-Package DBX.Dotnet
```

### PackageReference

```xml
<PackageReference Include="DBX.Dotnet" Version="0.0.3-beta" />
```

## 요구사항

- **.NET**: .NET Standard 2.0+
  - .NET Framework 4.6.1+
  - .NET Core 2.0+
  - .NET 5, 6, 7, 8+
- **플랫폼**: Windows x64 (현재 테스트 완료)
  - Linux x64: 계획됨
  - macOS (Intel/Apple Silicon): 계획됨

## 설치 확인

```csharp
using DBX.Dotnet;

using var db = Database.OpenInMemory();
Console.WriteLine("DBX.Dotnet loaded successfully!");
```

## 프로젝트 설정

### .NET 6+ (최소 프로젝트 파일)

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
  </PropertyGroup>

  <ItemGroup>
    <PackageReference Include="DBX.Dotnet" Version="0.0.3-beta" />
  </ItemGroup>
</Project>
```

### .NET Framework 4.6.1+

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net461</TargetFramework>
  </PropertyGroup>

  <ItemGroup>
    <PackageReference Include="DBX.Dotnet" Version="0.0.3-beta" />
  </ItemGroup>
</Project>
```

## 문제 해결

### DLL을 로드할 수 없음

**원인**: Visual C++ Redistributable 누락

**해결**:
1. [Microsoft Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe) 다운로드
2. 설치 후 애플리케이션 재시작

### .NET Framework에서 오류

**원인**: .NET Standard 2.0 호환성 문제

**해결**:
```xml
<PropertyGroup>
  <TargetFramework>net461</TargetFramework>
  <LangVersion>latest</LangVersion>
</PropertyGroup>
```

### 특정 버전 설치

```bash
# 최신 베타
dotnet add package DBX.Dotnet --prerelease

# 특정 버전
dotnet add package DBX.Dotnet --version 0.0.3-beta
```
