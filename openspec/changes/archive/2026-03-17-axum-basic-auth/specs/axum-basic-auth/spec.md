## ADDED Requirements

### Requirement: Basic auth credentials extraction

The axum Basic Auth extractor SHALL parse the `Authorization: Basic` header and
expose a username and an optional password.

#### Scenario: Username and password provided

- **WHEN** a request includes `Authorization: Basic <base64(username:password)>`
- **THEN** the extractor returns a `BasicAuth` with the decoded username and
  password

#### Scenario: Password omitted

- **WHEN** a request includes `Authorization: Basic <base64(username)>`
- **THEN** the extractor returns a `BasicAuth` with the decoded username and a
  missing password

### Requirement: Auth payload injection

For auth-enabled endpoints, the generated request payload type `T` MUST include
an `xidl_auth: BasicAuth` field.

#### Scenario: Auth enabled request type

- **WHEN** an endpoint is generated with Basic Auth enabled
- **THEN** its request payload type includes an `xidl_auth` field carrying
  `BasicAuth`

### Requirement: Invalid or missing auth handling

For auth-enabled endpoints, requests missing a valid Basic Auth header MUST be
rejected with an unauthorized response.

#### Scenario: Missing authorization header

- **WHEN** a request to an auth-enabled endpoint omits the Authorization header
- **THEN** the request is rejected as unauthorized

#### Scenario: Invalid basic header

- **WHEN** a request to an auth-enabled endpoint includes a non-Basic or
  malformed Authorization header
- **THEN** the request is rejected as unauthorized

### Requirement: WWW-Authenticate header on 401

For auth-enabled endpoints, unauthorized responses MUST include a
`WWW-Authenticate` header indicating the auth type and a realm. If the
annotation does not provide a realm, the handler function name MUST be used.

#### Scenario: Unauthorized response with configured realm

- **WHEN** a request to an auth-enabled endpoint is rejected as unauthorized and
  a realm is configured via annotation
- **THEN** the response includes `WWW-Authenticate: Basic realm="<realm>"`

#### Scenario: Unauthorized response with default realm

- **WHEN** a request to an auth-enabled endpoint is rejected as unauthorized and
  no realm is configured
- **THEN** the response includes
  `WWW-Authenticate: Basic realm="<handler_function_name>"`
