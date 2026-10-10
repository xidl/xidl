# xidlc

The XIDL compiler CLI. It turns `.idl` interface definitions into Rust,
TypeScript and Go code, OpenAPI and JSON-RPC schemas, and more.

## Install

```bash
npm install -g xidlc
```

The package installs a small launcher plus the prebuilt binary for the current
platform. macOS, Linux and Windows on arm64 and x64 are supported.

## Usage

```bash
xidlc --help
xidlc gen -o out rust your.idl
xidlc fmt --inplace api.idl
```

Set `XIDLC_BINARY_PATH` to run a locally built compiler through the same entry
point.

## Links

- Source, releases and documentation: <https://github.com/xidl/xidl>
