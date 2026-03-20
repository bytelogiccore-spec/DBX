# Contributing to DBX

Thank you for your interest in contributing to DBX! We welcome contributions from the community.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Contributor License Agreement (CLA)](#contributor-license-agreement-cla)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)

---

## 📜 Code of Conduct

We are committed to providing a welcoming and inclusive environment for all contributors. Please be respectful and professional in all interactions.

---

## 🚀 How to Contribute

### Reporting Bugs

If you find a bug, please open an issue on GitHub with:

- A clear description of the problem
- Steps to reproduce the issue
- Expected vs. actual behavior
- Your environment (OS, Rust version, DBX version)

### Suggesting Features

We welcome feature suggestions! Please open an issue with:

- A clear description of the feature
- Use cases and benefits
- Any implementation ideas (optional)

### Contributing Code

1. **Fork the repository** on GitHub
2. **Create a feature branch** (`git checkout -b feature/my-feature`)
3. **Make your changes** following our coding standards
4. **Add tests** for your changes
5. **Run tests** (`cargo test --workspace`)
6. **Commit your changes** following our commit message format
7. **Push to your fork** (`git push origin feature/my-feature`)
8. **Open a Pull Request** on GitHub

---

## 🛠️ Development Setup

### Prerequisites

- Rust 2024 edition or later
- CUDA 12.x+ (optional, for GPU features)

### Build

```bash
# Clone the repository
git clone https://github.com/ByteLogicStudio/DBX.git
cd DBX

# Build the project
cargo build --workspace

# Run tests
cargo test --workspace

# Run benchmarks
cd testing/benchmarks
cargo bench
```

### Build Optimization Tips (Recommended)

DBX is a large and complex project. You can maximize your compilation speed with the following setups:

#### Windows Environment (MSVC)
1. **Use LLD Linker**: The project's `.cargo/config.toml` is pre-configured to use `rust-lld.exe`.
2. **Windows Defender Exclusions (Important)**: Exclude the `DBX` project folder and `~/.cargo` folder from real-time scanning. This prevents severe bottlenecks caused by Defender scanning thousands of temporary files during compilation.
3. **Disk I/O**: An NVMe SSD is strongly recommended for smooth and fast building.

#### Linux and WSL2 Environment
1. **Use mold Linker**: We strongly recommend using `mold`, the fastest linker available.
   ```bash
   sudo apt-get install -y mold clang
   ```
   (The project's `.cargo/config.toml` is already configured to automatically use `mold` once installed.)

---

## 📝 Contributor License Agreement (CLA)

**By contributing to DBX, you agree to the following terms:**

### Grant of Copyright License

You hereby grant to ByteLogic Studio and to recipients of software distributed by ByteLogic Studio a perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable copyright license to:

- Reproduce, prepare derivative works of, publicly display, publicly perform, sublicense, and distribute your contributions and such derivative works.

### Grant of Patent License

You hereby grant to ByteLogic Studio and to recipients of software distributed by ByteLogic Studio a perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable patent license to:

- Make, have made, use, offer to sell, sell, import, and otherwise transfer your contributions.

### Why Do We Need This?

**Your contributions will always remain available under the MIT License** for the open-source community. The CLA ensures that the project can legally distribute your contributions and protects the project and its users from legal issues.

### Representation

You represent that:

- You are legally entitled to grant the above licenses
- Your contributions are your original creation
- Your contributions do not violate any third-party rights

### Sign the CLA

By submitting a pull request, you acknowledge that you have read and agree to this CLA.

For corporate contributors, please contact us at bytelogiccore@gmail.com to sign a Corporate CLA.

---

## 🔄 Pull Request Process

1. **Ensure your code passes all tests**
   ```bash
   cargo test --workspace
   cargo clippy --workspace -- -D warnings
   cargo fmt --check
   ```

2. **Update documentation** if you're adding new features

3. **Add tests** for new functionality

4. **Follow the commit message format** (see below)

5. **Wait for review** - We'll review your PR as soon as possible

6. **Address feedback** - Make requested changes and push updates

7. **Merge** - Once approved, we'll merge your PR

---

## 💻 Coding Standards

### Rust Style Guide

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Write idiomatic Rust code

### Commit Message Format

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
[type]: brief description

Detailed explanation (optional)

Fixes #123
```

**Types:**
- `[기능]`: New feature
- `[수정]`: Bug fix
- `[문서]`: Documentation changes
- `[스타일]`: Code style changes (formatting, etc.)
- `[리팩토링]`: Code refactoring
- `[테스트]`: Adding or updating tests
- `[빌드]`: Build system changes
- `[CI]`: CI/CD changes

**Example:**
```
[기능]: Add GPU-accelerated hash join

Implemented CUDA kernel for hash join operations,
achieving 2-3x speedup over CPU implementation.

Fixes #456
```

### Testing

- Write unit tests for all new functions
- Write integration tests for new features
- Aim for high test coverage
- Use property-based testing where appropriate

### Documentation

- Add rustdoc comments for all public APIs
- Include examples in documentation
- Update README.md if adding major features

---

## 🙏 Thank You!

Your contributions make DBX better for everyone. We appreciate your time and effort!

**Questions?** Contact us at dev@bytelogic.studio

---

**ByteLogic Studio**  
Seoul, Republic of Korea
