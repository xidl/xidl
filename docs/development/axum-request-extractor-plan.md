# Axum Request Extractor Plan

## Context

`xidlc/src/generate/http_hir/model.rs` stores request and response parameters in
direction buckets:

- `request_params`: everything that participates in the inbound HTTP request
- `response_params`: everything that participates in the outbound HTTP response

Each `HttpParam` still carries its HTTP source in `kind` (`Path`, `Query`,
`Header`, `Cookie`, `Body`).

The current `rust-axum` generator re-splits those buckets by source in
`xidlc/src/generate/rust_axum/interface.rs`, but the generated business trait
still wraps all request-side values inside `xidl_rust_axum::Request<T>`.

Now that XIDL parameters support `@header`, that wrapper no longer provides much
value for unary handlers:

- request headers can already be surfaced as explicit IDL parameters
- body/path/query/cookie values are already reified as normal parameters
- the wrapper hides the real handler shape behind an extra transport struct

## Decision

For unary HTTP handlers, the generated trait should stop using
`xidl_rust_axum::Request<T>` and instead expand request data back into function
parameters.

This makes the generated Rust trait align with the IDL interface shape much more
closely:

```idl
interface UserApi {
  @get(path="/users/{id}")
  User get_user(@path string id, @header("if-none-match") string etag);
}
```

should map to a server trait shape closer to:

```rust
async fn get_user(&self, id: String, etag: String) -> xidl_rust_axum::Result<User>;
```

instead of:

```rust
async fn get_user(
    &self,
    req: xidl_rust_axum::Request<GetUserRequest>,
) -> xidl_rust_axum::Result<User>;
```

## Feasibility

This is feasible with Axum extractors and simplifies the overall design.

### Axum constraints

- `FromRequestParts` is the correct trait for extractors that only read request
  parts and do not consume the body.
- `FromRequest` is required when the extractor also reads the body.
- Axum 0.8 allows custom extractors to compose other extractors with
  `parts.extract_with_state(...)` and `req.extract_parts_with_state(...)`.
- The request body can only be consumed once, so path/query/header/cookie/auth
  extraction must happen before body decoding.
- Headers can be read directly from `Parts`; cookies can continue to be decoded
  from the `cookie` header exactly as the current template does.

### Practical conclusion

The workable design is:

1. generated helper types continue to model HTTP sources (`Path`, `Query`,
   `Body`)
2. server route functions use those source-aware extractors to decode the HTTP
   request
3. route functions pass decoded values directly into the business trait method

No runtime bridge through `xidl_rust_axum::Request<T>` is needed for unary
handlers.

## Recommended design

### 1. Keep `http_hir` as-is

`HttpOperation.request_params` / `response_params` should stay direction-based.
They already preserve source information through `HttpParam.kind`, which is what
the renderer needs.

No `http_hir` schema change is required for this plan.

### 2. Change the generated business trait shape

Unary methods should render their trait signatures from expanded parameters,
using `MethodContext.params` directly.

That means moving from:

```rust
async fn op(&self, req: xidl_rust_axum::Request<OpRequest>) -> Result<...>;
```

to:

```rust
async fn op(&self, a: A, b: B, c: C) -> Result<...>;
```

This is the right abstraction boundary:

- the business trait is about domain inputs, not transport containers
- source annotations such as `@path`, `@query`, and `@header` stay in the
  generated adapter layer
- the generated trait becomes much easier to read and implement

### 3. Keep request structs only as transport helpers when needed

After this change, unary request structs should no longer be part of the public
business trait API.

They may still exist internally when useful for extraction:

- `Path` helper structs for route variables
- `Query` helper structs for query decoding
- `Body` helper structs for request-body decoding

The old aggregate `FooRequest` type should be removed for unary handler APIs
unless some remaining adapter path still needs it internally.

### 4. Let the server adapter perform extraction and argument expansion

The generated Axum route function should continue to own transport concerns:

- route/path matching
- query extraction
- body decoding
- header and cookie decoding
- auth extraction
- media-type validation

After extraction, the adapter should call:

```rust
svc.op(arg1, arg2, arg3).await
```

instead of assembling `Request<T>` first.

### 5. Keep response modeling separate

This change only removes the unary request wrapper from the server-facing API.
It does not require any response-side redesign.

Response structs can stay as they are for now because they still solve a real
transport-shaping problem:

- return value plus `out`/`inout` parameters
- header/cookie/body response assembly
- client-side response decoding

## Auth nuance

There is one important caveat: auth data is not part of the original IDL
parameter list.

Today the generator may surface auth as synthesized `xidl_auth` data. After
removing `Request<T>`, the implementation must choose one of these models:

1. render auth as a generated extra function parameter
2. keep auth hidden inside the adapter and not pass it to business logic
3. introduce a separate generated context parameter

The first option is the smallest delta relative to current behavior, but it
means the generated Rust trait is "IDL-shaped plus generated auth context"
rather than a byte-for-byte projection of the IDL signature.

That distinction should stay explicit in the implementation plan.

### Recommended auth choice

For the first implementation pass, prefer rendering auth as a generated extra
parameter.

Reasoning:

- it preserves current capability because business logic can still read auth
- it avoids introducing a second context abstraction while removing `Request<T>`
- it keeps the server adapter simple: extract auth, then call the business
  method directly

So unary handlers should move toward:

```rust
async fn get_user(
    &self,
    id: String,
    etag: String,
    xidl_auth: xidl_rust_axum::auth::bearer::BearerAuth,
) -> xidl_rust_axum::Result<User>;
```

when the operation declares auth, and toward plain IDL-shaped parameters when it
does not.

## Streaming scope

This plan is intentionally scoped to unary handlers first.

Streaming operations should remain on the current generated shape in phase 1:

- `@server_stream`
- `@client_stream`
- `@bidi_stream`

Reasoning:

- stream request objects are not ordinary scalar parameters
- WebSocket upgrade and streaming state are adapter-owned concerns
- removing `Request<T>` from unary handlers is already a user-visible breaking
  change and should be isolated

Once unary handlers are stable, streaming can be evaluated separately with its
own adapter shape.

## Implementation plan

1. Update `xidlc/src/generate/rust_axum/templates/interface.rs.j2` so unary
   trait methods use expanded parameters instead of `Request<T>`.
2. Remove unary `request_struct` usage from the public trait surface and keep
   only source-local helper structs that are still needed for extraction.
3. Rewrite `xidlc/src/generate/rust_axum/templates/interface/server.rs.j2` so
   unary handlers extract transport data and call `svc.method(...)` directly.
4. Preserve current parsing behavior for:
   - optional query fields
   - optional and multi-value headers
   - optional and multi-value cookies
   - flattened and structured body payloads
5. Decide how auth should be exposed after `Request<T>` removal:
   - generated explicit parameter
   - hidden adapter concern
   - separate generated context type
6. Keep streaming handlers on the current `Request<T>`-based path first if that
   reduces migration risk.
7. Update snapshots and focused generator tests for:
   - path + query only handlers
   - header and cookie parameters
   - body + header/query composition
   - auth-exposed handlers
   - zero-parameter methods
8. Update user-facing docs only after the generator actually changes.

## Risks and follow-up

- Some existing examples and snapshots assume every handler receives
  `Request<T>`; all of them will need regeneration.
- If auth is rendered as an extra generated parameter, the trait no longer maps
  exactly to raw IDL signatures.
- Streaming methods may still need a transport wrapper because stream objects
  and WebSocket upgrade state are adapter-centric, not ordinary IDL scalars.
- If some users relied on raw header access beyond declared `@header`
  parameters, that escape hatch disappears unless a separate context parameter
  is introduced.

## References

- Axum extractors index: <https://docs.rs/axum/0.8.9/axum/extract/index.html>
- `FromRequestParts`:
  <https://docs.rs/axum/0.8.9/axum/extract/trait.FromRequestParts.html>
- `RequestPartsExt::extract_with_state`:
  <https://docs.rs/axum/0.8.9/axum/trait.RequestPartsExt.html>
- `RequestExt::extract_parts_with_state`:
  <https://docs.rs/axum/0.8.9/axum/trait.RequestExt.html>
