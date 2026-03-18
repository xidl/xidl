## ADDED Requirements

### Requirement: Client auth configuration
Generated clients SHALL expose a configuration mechanism to supply credentials for `@http_basic`, `@http_bearer`, and `@api_key` schemes.

#### Scenario: Configure bearer token on client
- **WHEN** a caller provides a bearer token in the client auth configuration
- **THEN** the client stores the token and uses it for endpoints requiring bearer auth

### Requirement: Apply bearer auth to HTTP requests
For endpoints that require `@http_bearer`, generated HTTP clients SHALL attach an `Authorization: Bearer <token>` header on every request.

#### Scenario: Bearer header added to protected endpoint
- **WHEN** a client calls a `@http_bearer` endpoint with bearer credentials configured
- **THEN** the request includes `Authorization: Bearer <token>`

### Requirement: Apply basic auth to HTTP requests
For endpoints that require `@http_basic`, generated HTTP clients SHALL attach an `Authorization: Basic <base64(user:pass)>` header on every request.

#### Scenario: Basic header added to protected endpoint
- **WHEN** a client calls a `@http_basic` endpoint with basic credentials configured
- **THEN** the request includes `Authorization: Basic <base64(user:pass)>`

### Requirement: Apply api key auth to HTTP requests
For endpoints that require `@api_key`, generated HTTP clients SHALL attach the configured api key in the specified location (header or query) and name.

#### Scenario: Api key header added to protected endpoint
- **WHEN** a client calls an `@api_key` endpoint that specifies a header name
- **THEN** the request includes that header with the configured api key value

### Requirement: Honor no-security endpoints
For endpoints marked `@no_security`, generated clients SHALL NOT attach auth headers unless explicitly overridden by the caller.

#### Scenario: No auth added for no-security endpoint
- **WHEN** a client calls a `@no_security` endpoint
- **THEN** the request is sent without auth headers by default

### Requirement: Apply auth to websocket handshakes
For bidi stream endpoints that require auth, generated clients SHALL include the required auth headers in the websocket handshake request.

#### Scenario: Bearer header added to websocket handshake
- **WHEN** a client opens a bidi stream endpoint that requires bearer auth
- **THEN** the websocket handshake request includes `Authorization: Bearer <token>`

### Requirement: Explicit per-call override
Generated clients SHALL allow callers to override or disable auth headers for an individual call.

#### Scenario: Per-call override disables auth
- **WHEN** a caller disables auth for a single request
- **THEN** the request is sent without auth headers even if client auth is configured
