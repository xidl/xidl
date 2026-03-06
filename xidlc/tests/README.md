# xidlc Snapshot Tests

This directory contains IDL-driven snapshot tests for code generation.

## Layout

- `c/`, `cpp/`, `rust/`, `ts/`, `axum/`, `openapi/`: input `.idl` test cases
- `snapshots/`: generated `insta` snapshots
- `codegen_snapshot.rs`: test runner that discovers cases automatically

Each `*.idl` file under a language folder is treated as one snapshot case.

## Run tests

```bash
cargo test -p xidlc --test codegen_snapshot
```

## Update snapshots

```bash
INSTA_UPDATE=always cargo test -p xidlc --test codegen_snapshot
```

Use `INSTA_UPDATE=new` if you only want to accept newly-added snapshots.
