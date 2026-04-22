# Axum Unary Parameter Expansion TODO

This checklist tracks the implementation of removing unary
`xidl_rust_axum::Request<T>` from generated `rust-axum` server traits and
expanding request data into explicit function parameters.

## Phase 0: Lock scope

- [ ] Confirm phase 1 only targets unary HTTP handlers.
- [ ] Confirm streaming handlers keep the current generated shape.
- [ ] Confirm auth is exposed as a generated extra parameter in phase 1.
- [ ] Confirm no `http_hir` schema change is required.

## Phase 1: Generator model updates

- [ ] Audit `xidlc/src/generate/rust_axum/interface.rs` for every field that
      only exists to support unary `Request<T>`.
- [ ] Separate unary trait-signature rendering needs from adapter-only transport
      helper needs.
- [ ] Mark unary aggregate request structs as internal-only or removable.
- [ ] Verify `MethodContext.params` is sufficient for rendering final unary
      trait signatures.
- [ ] Verify auth metadata can render a stable extra parameter shape.

## Phase 2: Trait generation

- [ ] Update `xidlc/src/generate/rust_axum/templates/interface.rs.j2` so unary
      methods render expanded parameters instead of `Request<T>`.
- [ ] Keep streaming methods unchanged in the same template.
- [ ] Remove unary public `FooRequest` exposure where it is no longer needed.
- [ ] Preserve deprecated attributes and rust passthrough attrs on the new
      method signatures.

## Phase 3: Server adapter rewrite

- [ ] Update `xidlc/src/generate/rust_axum/templates/interface/server.rs.j2` so
      unary route handlers call `svc.method(arg1, arg2, ...)`.
- [ ] Keep existing media-type validation behavior unchanged.
- [ ] Keep existing path/query extraction behavior unchanged.
- [ ] Keep existing header parsing behavior unchanged.
- [ ] Keep existing cookie parsing behavior unchanged.
- [ ] Keep existing body decoding behavior unchanged.
- [ ] Keep existing auth rejection behavior unchanged.
- [ ] Remove unary `Request::new(headers, data)` assembly.

## Phase 4: Cleanup

- [ ] Remove now-unused unary request-struct context fields from
      `xidlc/src/generate/rust_axum/interface.rs`.
- [ ] Remove dead template branches that only supported unary `Request<T>`.
- [ ] Check whether `xidl-rust-axum/src/request.rs` is now only needed for
      streaming paths.
- [ ] If `Request<T>` becomes streaming-only, document that explicitly instead
      of deleting it immediately.

## Phase 5: Snapshot and generator tests

- [ ] Regenerate snapshots that currently expect unary `Request<T>` signatures.
- [ ] Add or update a snapshot for pure path/query expansion.
- [ ] Add or update a snapshot for header parameters.
- [ ] Add or update a snapshot for cookie parameters.
- [ ] Add or update a snapshot for structured body parameters.
- [ ] Add or update a snapshot for flattened body parameters.
- [ ] Add or update a snapshot for auth-expanded unary handlers.
- [ ] Add or update a snapshot for zero-parameter unary handlers.
- [ ] Add or update a snapshot proving streaming handlers remain unchanged.

## Phase 6: User docs

- [ ] Update `docs/rust-axum.md` examples to show expanded unary parameters.
- [ ] Update `xidl-rust-axum/README.md` examples to stop presenting unary
      `Request<T>` as the default handler shape.
- [ ] Update any RFC or guide text that currently implies unary business logic
      receives `Request<T>`.
- [ ] Add a short migration note for generated server implementations.

## Phase 7: Validation

- [ ] Run focused generator tests for `rust-axum`.
- [ ] Run snapshot tests covering HTTP annotations, headers, cookies, auth, and
      body shaping.
- [ ] Run runtime tests needed to confirm streaming paths still compile.
- [ ] Run the repository validation flow required by the code-style skill before
      merging.

## Open questions

- [ ] Should auth extra parameters always be appended last for signature
      stability?
- [ ] Should generated auth parameters keep the name `xidl_auth` everywhere?
- [ ] Should unary helper request structs remain `pub`, become private, or be
      removed entirely?
- [ ] Should there be a follow-up change to remove `Request<T>` from streaming
      handlers too, or keep it as the long-term stream adapter API?
