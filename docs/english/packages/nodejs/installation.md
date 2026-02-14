---
layout: default
title: Installation
parent: Node.js (dbx-py)
grand_parent: Packages
great_grand_parent: English
nav_order: 1
---

# Installation

## Install from npm

```bash
npm install dbx-py
```

## Install with yarn

```bash
yarn add dbx-py
```

## Install with pnpm

```bash
pnpm add dbx-py
```

## Requirements

- **Node.js**: 16.x or higher
- **Platform**: Windows x64 (currently tested)
  - Linux x64: Planned
  - macOS (Intel/Apple Silicon): Planned

## Verify Installation

```javascript
const { Database } = require('dbx-py');

const db = Database.openInMemory();
console.log('DBX Node.js loaded successfully!');
db.close();
```

## TypeScript Setup

### Install Type Definitions

Type definitions are included in the package.

### tsconfig.json

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "esModuleInterop": true,
    "strict": true
  }
}
```

### Example

```typescript
import { Database } from 'dbx-py';

const db: Database = Database.openInMemory();
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
db.close();
```

## Troubleshooting

### Module Not Found

**Cause**: Installation failed or wrong Node.js version

**Solution**:
```bash
# Check Node.js version
node --version  # Should be 16+

# Reinstall
npm uninstall dbx-py
npm install dbx-py
```

### Native Module Error on Windows

**Cause**: Missing Visual C++ Redistributable

**Solution**:
1. Download [Microsoft Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)
2. Install and restart

### Version Check

```bash
npm list dbx-py
```

## Next Steps

- [Quick Start](quickstart) - Get started in 5 minutes
- [SQL Guide](sql-guide) - SQL usage
- [API Reference](api-reference) - Complete API
