## 1. Auth Core

- [x] 1.1 Add `BasicAuth` type and parsing helpers in
      `xidl-rust-axum/src/auth/basic.rs` with optional password
- [x] 1.2 Implement 401 responses to include `WWW-Authenticate` with optional
      realm

## 2. Codegen Integration

- [x] 2.1 Extend auth-enabled request payload types to include
      `xidl_auth: BasicAuth`
- [x] 2.2 Wire Basic Auth extraction into generated axum handlers and propagate
      unauthorized errors

## 3. Docs and Examples

- [x] 3.1 Update `xidlc-examples/api/http/http_server.idl` to use Basic Auth and
      `xidl_auth`
- [x] 3.2 Update `docs/rfc/http-security.md` with Basic Auth usage, realm
      config, and header behavior

## 4. Verification

- [x] 4.1 Build `http_server` example with `cargo b --example http_server`
