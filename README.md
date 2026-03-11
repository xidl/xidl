XIDL (eXtensible IDL) is an OMG IDL-based code generator. It generates
C/C++/Rust/Typescript code and also supports
[HTTP](docs/rfc/http.md)/[JSONRPC](docs/rfc/jsonrpc.md).

> Currently,
> [HTTP-STREAM](docs/rfc/http-stream.md)/[JSONRPC-STREAM](docs/rfc/jsonrpc-stream.md) are still in an implementation/experimental stage.

[![publish-release](https://github.com/xidl/xidl/actions/workflows/publish-release.yml/badge.svg)](https://github.com/xidl/xidl/actions/workflows/publish-release.yml)
[![publish-crates](https://github.com/xidl/xidl/actions/workflows/publish-crates.yml/badge.svg)](https://github.com/xidl/xidl/actions/workflows/publish-crates.yml)
[![deploy-docs](https://github.com/xidl/xidl/actions/workflows/deploy-docs.yml/badge.svg)](https://github.com/xidl/xidl/actions/workflows/deploy-docs.yml)
![Crates.io Version](<https://img.shields.io/crates/v/xidlc?label=xidlc(crates.io)>)
![Crates.io Version](<https://img.shields.io/crates/v/xidl-build?label=xidl-build(crates.io)>)
![GitHub Release](https://img.shields.io/github/v/release/xidl/xidl)
[![GitHub](https://img.shields.io/badge/View_on-GitHub-181717?logo=github)](https://github.com/xidl/xidl)

## Installation

=== "Release (macOS / Linux)"

    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://xidl.github.io/xidl/public/install.sh | sh
    ```

=== "Release (Windows PowerShell)"

    ```powershell
    iwr -useb https://xidl.github.io/xidl/public/install.ps1 | iex
    ```

=== "Cargo"

    ```bash
    cargo install xidlc
    ```

=== "Cargo Binstall"

    ```bash
    cargo binstall xidlc
    ```

## Quick Start

```bash
# Generate code
xidlc -l rust -o out your.idl
```

## Built-in Targets

- `c`
- `cpp`
- `rust`
- `rust-jsonrpc`
- `rust-axum`
- `ts` / `typescript`
