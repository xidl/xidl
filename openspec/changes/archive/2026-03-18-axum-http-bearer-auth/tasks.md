## 1. Runtime Auth Types

- [x] 1.1 Add `BearerAuth` type and extractor logic in
      `xidl-rust-axum/src/auth/bearer.rs` (including empty token default
      behavior)
- [x] 1.2 Move/shared auth structs (user/password extractor) into
      `xidl-rust-axum/src/auth/bearer.rs` and update
      `xidl-rust-axum/src/auth/mod.rs` exports

## 2. Codegen and Mapping

- [x] 2.1 Update HTTP security mapping to recognize `@http-bearer` for axum
      server generation
- [x] 2.2 Update axum server codegen to add `xidl_auth: BearerAuth` onto the
      request payload type used in `xidl_rust_axum::Request<T>`
- [x] 2.3 Ensure handler extraction uses `TypedHeader<BearerHeader>` (via
      `axum_extra`) and passes token into `BearerAuth`

## 3. Docs and Examples

- [x] 3.1 Update `docs/rfc/http-security.md` with Bearer auth guidance and
      example
- [x] 3.2 Update `xidlc-examples/api/http/http_server.idl` and any related
      examples to demonstrate Bearer auth
