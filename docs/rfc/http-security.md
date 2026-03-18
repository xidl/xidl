# XIDL HTTP Security Mapping Specification (Draft)

**Status:** Implementation-oriented RFC. This document specifies runtime
behavior; generator and runtime support may vary by target.

Reference specifications:

- <https://www.rfc-editor.org/rfc/rfc9110>
- <https://www.rfc-editor.org/rfc/rfc6750>
- <https://www.rfc-editor.org/rfc/rfc7617>
- <https://www.rfc-editor.org/rfc/rfc7235>
- <https://www.rfc-editor.org/rfc/rfc6749>
- <https://spec.openapis.org/oas/latest.html>

## 1. Scope

This document defines a companion security profile for the XIDL HTTP mapping. It
extends unary HTTP APIs with security-related annotations and mapping rules.

This document defines:

- operation-level and interface-level security declarations
- basic authentication scheme mapping
- bearer token mapping
- API key mapping
- security inheritance and override rules
- documentation-facing security semantics

This document does not define:

- transport security negotiation
- session management
- stream security
- authorization policy evaluation language

## 2. Terminology

- `security requirement`: a declaration that a caller must satisfy one or more
  authentication schemes.
- `scheme`: one concrete authentication mechanism such as HTTP Basic or Bearer.
- `scope`: a named authorization capability attached to a scheme.
- `anonymous access`: an operation that does not require authentication.

## 3. Annotation Model

The following annotations are proposed:

- `@no_security`
- `@http_basic`
- `@http_bearer`
- `@api_key(in = "header", name = "X-API-Key")`
- `@api_key(in = "cookie", name = "sid")`
- `@api_key(in = "query", name = "api_key")`
- `@oauth2(scopes = ["scope1", "scope2"])`

Annotation roles:

- security annotations directly declare authentication requirements on an
  interface or operation
- `@no_security` explicitly disables inherited security requirements

This draft intentionally keeps the annotation surface small. It is designed to
cover common production HTTP APIs without reproducing every OpenAPI security
feature.

## 4. Declaring Security Requirements

### 4.1 HTTP Basic

```idl
@http_basic
```

Semantics:

- request authentication uses the HTTP `Authorization` header
- credential format is `Basic <base64(user:password)>`
- unauthorized responses include `WWW-Authenticate: Basic realm="<realm>"`
- `realm` MAY be supplied via `@http_basic(realm = "...")`; if omitted, the
  handler function name is used

### 4.2 HTTP Bearer

```idl
@http_bearer
```

Semantics:

- request authentication uses the HTTP `Authorization` header
- credential format is `Bearer <token>`
- bearer format is intentionally opaque in this RFC

Axum mapping guidance:

- generated axum handlers extract `TypedHeader<BearerHeader>` (via `axum_extra`)
  to allow empty tokens
- the request payload type `T` includes `xidl_auth: BearerAuth` and is carried
  inside `xidl_rust_axum::Request<T>`
- if the header is `Authorization: Bearer` (empty token), the token value is the
  default `String` value

### 4.3 API Key

```idl
@api_key(in = "header", name = "X-API-Key")
@api_key(in = "cookie", name = "sid")
@api_key(in = "query", name = "api_key")
```

## 7. Client Usage (rust-axum)

Generated Rust HTTP/stream clients can be configured with `ClientAuth` to
automatically attach required auth headers based on IDL annotations.

```rust
use xidl_rust_axum::{ClientAuth, ApiKeyAuth, ClientApiKeyLocation};
use xidl_rust_axum::auth::basic::BasicAuth;

let auth = ClientAuth {
    basic: Some(BasicAuth {
        username: "user".to_string(),
        password: Some("pass".to_string()),
    }),
    bearer: Some("token".to_string()),
    api_keys: vec![ApiKeyAuth {
        location: ClientApiKeyLocation::Header,
        name: "X-API-Key".to_string(),
        value: "secret".to_string(),
    }],
};

// Generated client (example)
// let client = MyApiClient::with_auth(base_url, auth);
```

For per-call overrides, callers can create a temporary client with
`with_auth_override(Some(...))` or `with_auth_override(None)` and use it for a
single request.

Semantics:

- `in` selects where credentials are supplied:
  - `header`
  - `cookie`
  - `query`
- `name` is the header field name, cookie name, or query key
- API key values are treated as opaque strings

### 4.4 OAuth2 / Scope-Carrying Schemes

```idl
@oauth2(scopes = ["read:users"])
```

Semantics:

- `oauth2` is a scope-carrying scheme
- multiple scopes on one annotation mean all listed scopes are required
- this RFC does not model flows, token URLs, or authorization URLs
- those details may be supplied by companion documentation or generator-specific
  extensions

## 5. Applying Security to Interfaces and Operations

Security requirements may be attached to an interface or to an operation.

Examples:

```idl
interface UserApi {
  @http_bearer
  User me();

  @no_security
  string health();
};
```

Rules:

- interface-level security annotations define the default security requirements
  for all operations in that interface
- operation-level security annotations replace inherited interface defaults
- `@no_security` on an operation clears inherited interface-level security
  requirements

Recommended override model:

- no operation-level security annotations -> inherit interface-level security
- one or more operation-level security annotations -> replace inherited security
  requirements
- `@no_security` -> require anonymous access

## 6. Requirement Semantics

Each security annotation represents one security requirement.

Default semantics:

- if both `@http_basic` and `@http_bearer` are present on the same target, they
  are interpreted as alternative requirements (logical OR)
- `@api_key(...)` and `@oauth2(...)` participate in the same alternative-set
  model as other security annotations
- if no operation-level security annotations are present, the effective
  requirement is inherited from the interface
- if one or more operation-level security annotations are present, the
  interface-level requirement is discarded and replaced by the operation-level
  requirement set
- if `@no_security` is present on an operation, the effective requirement is
  anonymous access and no other security annotation may appear on that operation

## 7. HTTP Mapping

This RFC defines only declaration-level mapping. Authentication and
authorization enforcement remain implementation responsibilities.

Request mapping expectations:

- HTTP Basic and Bearer use the `Authorization` header
- API key schemes use the declared request location and key name
- credentials are not represented as ordinary business parameters in the HTTP
  mapping
- authentication results SHOULD be exposed through implementation-specific
  request context rather than ordinary business parameters

Response guidance:

- missing or invalid credentials -> `401 Unauthorized`
- authenticated but insufficient privileges -> `403 Forbidden`
- when `401` is returned for HTTP-auth-based schemes, implementations SHOULD
  emit `WWW-Authenticate` where applicable

## 8. Interaction with Core HTTP Mapping

Security annotations do not change:

- route resolution
- parameter source resolution
- request/response body shaping
- deprecation metadata

Security annotations add preconditions on request acceptance only.

## 9. Documentation Mapping

Generators targeting documentation formats should preserve security metadata.

OpenAPI guidance:

- `@http_basic`, `@http_bearer`, `@api_key`, and `@oauth2` map naturally to
  `components.securitySchemes`
- interface and operation requirements map naturally to `security`
- `@http_security` maps to an explicitly empty security requirement set

This RFC does not require a generator to preserve every scheme-specific detail
if the target format lacks a direct equivalent.

## 10. Validation Rules

The following are invalid:

- `@no_security` combined with operation-level security annotations on the same
  operation
- duplicate `@http_basic` annotations on the same target
- duplicate `@http_bearer` annotations on the same target
- `@api_key` with empty `name`
- `@api_key` with `in` outside `header|cookie|query`

## 11. Non-Goals and Future Work

This draft intentionally does not yet define:

- mTLS
- OpenID Connect discovery
- OAuth2 flow metadata
- signed request schemes
- fine-grained authorization expressions
- per-response security metadata

These may be added in later revisions or companion RFCs without changing the
core unary HTTP mapping.
