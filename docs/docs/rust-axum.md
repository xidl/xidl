# xidlc `rust-axum`

The `rust-axum` target generates Rust HTTP server/client code from IDL.

## Minimal Flow

```bash
xidlc \
  --lang rust-axum \
  --out-dir out \
  your.idl
```

Then implement the generated trait and run the server with `xidl_rust_axum::Server`.
