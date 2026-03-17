# HTTP Snapshot Tests

HTTP snapshot tests are defined in `tests/http_snapshots/defs/*.http` using a
Hurl-like format. Files are rendered with minijinja before execution.

Variables available to templates:

- `base_url`: injected by the test runner (defaults to the local server it
  starts).
- `basic_auth`: injected by the test runner.
- `env`: environment variables map (`env.MY_VAR`).

Update snapshots:

```bash
UPDATE_HTTP_SNAPSHOTS=1 RUSTC_WRAPPER= cargo test -p xidlc-examples http_snapshot_tests
```

Run without updating:

```bash
RUSTC_WRAPPER= cargo test -p xidlc-examples http_snapshot_tests
```
