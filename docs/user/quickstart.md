# Quickstart

This page preserves the repository-level quickstart that used to live in the
root `README.md`.

## Install XIDL

### Release installer

macOS and Linux:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://xidl.github.io/xidl/public/install.sh | sh
```

Windows PowerShell:

```powershell
iwr -useb https://xidl.github.io/xidl/public/install.ps1 | iex
```

### Cargo

```bash
cargo install xidlc
```

### Cargo Binstall

```bash
cargo binstall xidlc
```

## Generate code

```bash
xidlc -l rust -o out your.idl
```

## Built-in targets

- `c`
- `cpp`
- `rust`
- `rust-jsonrpc`
- `rust-axum`
- `ts` / `typescript`
- `openapi`
- `openrpc`
- `hir`
- `typed-ast`

## Related guides

- [Installation](install.md)
- [Using xidlc](xidlc.md)
- [IDL Guide](idl.md)
- [HTTP Guide](http.md)
- [JSON-RPC Guide](jsonrpc.md)
- [Targets Reference](../reference/targets.md)
