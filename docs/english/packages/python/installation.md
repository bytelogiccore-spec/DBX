---
layout: default
title: Installation
parent: Python (dbx-py)
grand_parent: Packages
great_grand_parent: English
nav_order: 1
---

# Installation

## Install from PyPI

```bash
pip install dbx-py
```

## Install with pip

```bash
python -m pip install dbx-py
```

## Install with uv (Recommended)

```bash
uv pip install dbx-py
```

## Requirements

- **Python**: 3.8 or higher
- **Platform**: Windows x64 (currently tested)
  - Linux x64: Planned
  - macOS (Intel/Apple Silicon): Planned

## Verify Installation

```python
from dbx_py import Database

db = Database.open_in_memory()
print("DBX Python loaded successfully!")
db.close()
```

## Virtual Environment

### venv

```bash
python -m venv venv
source venv/bin/activate  # Linux/macOS
venv\Scripts\activate     # Windows

pip install dbx-py
```

### conda

```bash
conda create -n dbx python=3.11
conda activate dbx
pip install dbx-py
```

## Troubleshooting

### Module Not Found

**Cause**: Installation failed or wrong Python environment

**Solution**:
```bash
# Verify installation
pip list | grep dbx-py

# Reinstall
pip uninstall dbx-py
pip install dbx-py
```

### Import Error on Windows

**Cause**: Missing Visual C++ Redistributable

**Solution**:
1. Download [Microsoft Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)
2. Install and restart

### Version Check

```bash
pip show dbx-py
```

## Next Steps

- [Quick Start](quickstart) - Get started in 5 minutes
- [SQL Guide](sql-guide) - SQL usage
- [API Reference](api-reference) - Complete API
