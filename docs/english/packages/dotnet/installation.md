---
layout: default
title: Installation
parent: .NET (DBX.Dotnet)
grand_parent: Packages
great_grand_parent: English
nav_order: 1
---

# Installation

## Install from NuGet

```bash
dotnet add package DBX.Dotnet
```

## Package Manager Console

```powershell
Install-Package DBX.Dotnet
```

## Requirements

- **.NET Standard**: 2.0 or higher
  - .NET Framework 4.6.1+
  - .NET Core 2.0+
  - .NET 5, 6, 7, 8+
- **Platform**: Windows x64 (currently tested)
  - Linux x64: Planned
  - macOS: Planned

## Verify Installation

```csharp
using DBX.Dotnet;

var db = Database.OpenInMemory();
Console.WriteLine("DBX .NET loaded successfully!");
db.Dispose();
```

## Project Setup

### .NET 6+ (Minimal API)

```csharp
using DBX.Dotnet;

var db = Database.OpenInMemory();
db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
db.Dispose();
```

### .NET Framework

```csharp
using DBX.Dotnet;
using System.Text;

var db = Database.OpenInMemory();
db.Insert("users", Encoding.UTF8.GetBytes("user:1"), Encoding.UTF8.GetBytes("Alice"));
db.Dispose();
```

## Troubleshooting

### DllNotFoundException

**Cause**: Missing native library

**Solution**:
1. Ensure NuGet package is properly restored
2. Check platform target (x64)
3. Reinstall package

### Platform Not Supported

**Cause**: Running on unsupported platform

**Solution**:
- Currently only Windows x64 is supported
- Linux/macOS support is planned

## Next Steps

- [Quick Start](quickstart) - Get started in 5 minutes
- [SQL Guide](sql-guide) - SQL usage
- [API Reference](api-reference) - Complete API
