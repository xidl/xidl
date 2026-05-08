# HTTP Integration Tests (Hurl)

HTTP integration tests are defined in `tests/rest_snapshots/defs/*.hurl` using the
[Hurl](https://hurl.dev/) format.

Tests are executed within a Rust `#[tokio::test]` that:
1. Starts a local instance of the `RestServer`.
2. Injects the dynamic `base_url` into Hurl via variables.
3. Invokes `pnpm hurl --test` to verify status codes, headers, and bodies.

## Prerequisites

Dependencies are managed via `pnpm`:

```bash
pnpm install
```

## Running Tests

Run via cargo:

```bash
cargo test -p xidlc-examples rest_snapshot_tests
```

Or run Hurl manually against a running server:

```bash
pnpm hurl --variable base_url=http://localhost:8080 --test tests/rest_snapshots/defs/rest_server.hurl
```
