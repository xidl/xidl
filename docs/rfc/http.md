# XIDL HTTP Mapping Specification (RFC Draft)

Reference specifications:

- <https://www.omg.org/spec/CORBA-REST>
- <https://www.omg.org/spec/DDS-RPC>

## 1. Scope

This document defines how XIDL `interface` declarations are mapped to HTTP APIs,
including:

- Method and route mapping
- Parameter source rules (path/query/body)
- Request and response encoding rules
- Attribute mapping rules

This document does not cover OpenAPI generation, migration, or compatibility
strategy.

## 2. Terminology

- `IDL method`: an operation declared in an `interface`.
- `HTTP method`: one of `GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS`.
- `Path variable`: a placeholder in a route template, such as `{name}`.
- `Body parameter`: a parameter encoded in the HTTP request body.

## 3. Method Annotations

The following annotations are supported:

- `@get(path = "...")`
- `@post(path = "...")`
- `@put(path = "...")`
- `@patch(path = "...")`
- `@delete(path = "...")`
- `@head(path = "...")`
- `@options(path = "...")`
- `@path("...")` (method-level route declaration)
- `@Consumes("mime/type")` (interface-level or method-level)
- `@Produces("mime/type")` (interface-level or method-level)

Parameter annotations:

- `@path` (parameter-level source declaration)
- `@path("name")` (parameter-level source declaration with explicit route/query
  name)
- `@query` (parameter-level source declaration)
- `@query("name")` (parameter-level source declaration with explicit route/query
  name)

Rules:

- The HTTP method is determined by HTTP verb annotations, and verb annotations
  are mutually exclusive:
  - A method can have only one of
    `@get/@post/@put/@patch/@delete/@head/@options`.
  - Using more than one verb annotation on the same method is invalid and should
    raise an error.
- If no HTTP verb annotation is present, behavior is equivalent to `@post`.
- Route path can come from a verb annotation `path` argument or from
  `@path(...)`.
- A method may define multiple paths, and repeated path declarations are
  allowed:
  - Multiple `@path(...)`
  - Multiple path declarations in supported annotation forms
  - Mixed usage of both sources
- Duplicate routes after normalization should be de-duplicated.
- `Consumes/Produces` define request/response media types (encoding format):
  - `@Consumes`: request payload decoding format.
  - `@Produces`: response payload encoding format.
  - Default is `application/json` when unspecified.
  - Scope and override:
    - `interface`-level annotations define defaults for all methods.
    - method-level annotations override interface-level defaults.
    - if neither is specified, use `application/json`.
  - This RFC defines the JSON mapping as the baseline profile. Other media types
    may be added in future revisions without changing the core HTTP source/route
    rules.
- `@head` has additional constraints:
  - return type MUST be `void`
  - parameters MUST be request-side only (`in` or omitted direction)
  - `out` and `inout` parameters are invalid
  - response is always `204 No Content` with no response body
  - request-side parameter source rules are unchanged (`@path` / `@query`
    annotations are allowed and follow section 5)

## 4. Route Resolution

### 4.1 Defaults

- Default HTTP method: `POST` (equivalent to `@post` when omitted).
- Default route:
  - If explicit method path is not provided, route is auto-generated from method
    and parameter annotations.
  - Base route is exactly `/{method_name}` (no interface/module prefix).
  - `@path` parameters are appended as path template segments.
  - `@query` parameters do not change the route path; they are resolved as query
    parameters at request handling time.
  - Example: `void get_name(@query string name)` -> `POST "/get_name"`.

### 4.2 Explicit Routes and Multi-Route

- If a method explicitly defines route paths (from a verb annotation `path` or
  from `@path(...)`), all defined paths are effective (one method can be bound
  to multiple routes).
- If both sources are present, they are merged.
- If no explicit route is defined, use the default route (see 4.1).
- Route strings are normalized by section 4.4 before binding and conflict
  checks.

Example (multiple routes for one method):

```idl
@get(path="/v1/users/{id}")
@path("/users/{id}")
@path("/u/{id}")
User get_user(uint32 id);
```

The method above is registered on 3 routes.

### 4.3 Route Templates

Route templates support placeholders, for example:

- `/users/{id}`
- `/orders/{order_id}/items/{item_id}`
- `/files/{*path}` (catch-all path variable)
- `/users/{id}{?lang,region}` (query-template variables)

Template variable names are used by parameter source resolution (see section 5).

`{*name}` means a catch-all path variable:

- it matches one or more path segments
- it is bound as Path source with bound name `name`
- it is intended for trailing path capture (for example `/files/{*path}`)

`{?name1,name2,...}` means query-template variables:

- it declares Query-source variable names on the route template
- it does not change the bound route path
- each listed name must be bound by exactly one request-side Query parameter
- example: `/users/{id}{?lang,region}` binds `lang` and `region` as query keys

### 4.4 Route Normalization

Route normalization is applied to every explicit route and every auto-generated
route before de-duplication and conflict checks.

Normalization rules:

1. Trim leading/trailing ASCII whitespace.
2. Ensure the route starts with `/`.
3. Collapse repeated `/` into a single `/`.
4. Remove trailing `/` unless the full route is exactly `/`.
5. Keep route path case as-is (case-sensitive match).
6. Do not percent-decode or percent-rewrite path segments.
7. If query-template suffix exists (`{?...}`), normalize only the path part;
   query-template variable names are preserved as declared.

Examples:

- `" users/{id} "` -> `"/users/{id}"`
- `"//users///{id}/"` -> `"/users/{id}"`
- `"/"` -> `"/"`

### 4.5 Auto Path Generation Algorithm

This section defines the algorithm used when a method has no explicit method
path (that is, no verb `path=...` and no method-level `@path(...)`).

Inputs:

- Method name
- Parameter list (including parameter annotations and names)
- Resolved HTTP method (from section 3/4.1)

Algorithm:

1. Start with base path `/{method_name}`.
2. Resolve each parameter's source and bound name:
   - Bound name uses section 5 rules:
     - `@path("x")` / `@query("x")` -> bound name is `x`
     - `@path` / `@query` -> bound name is parameter name
   - Source resolution priority is section 5.
3. Collect Path-bound parameter names in declaration order:
   - Append each as `/{name}`.
4. Query-bound parameters do not change the route path.
5. Auto-generated routes do not include query-template suffix (`{?...}`).
6. Normalize result:
   - Apply section 4.4 route normalization.
   - Preserve declaration order.

Notes:

- For `GET/DELETE/HEAD/OPTIONS`, unannotated parameters typically become query
  parameters (section 5), but they do not affect generated route path.
- For `POST/PUT/PATCH`, unannotated parameters typically become body parameters,
  so they do not contribute to generated route path.

Examples:

```idl
// Example 1: query-only, default POST
void get_name(@query string name);
// => POST "/get_name"

// Example 2: explicit query name
void get_user(@query("id") uint32 user_id);
// => POST "/get_user"

// Example 3: path + query
void find_user(@path uint32 id, @query string locale);
// => POST "/find_user/{id}"

// Example 4: explicit path and query names
void find_user2(@path("user_id") uint32 id, @query("lang") string locale);
// => POST "/find_user2/{user_id}"

// Example 5: GET default source + one explicit path
@get
void list_orders(@path("uid") uint32 user_id, uint32 page, uint32 size);
// => GET "/list_orders/{uid}"

// Example 6: mixed in/out/inout (only request-side params affect path template)
long add(in long a, in long b, out long sum);
// => POST "/add"

// Example 7: catch-all path variable
@get(path="/files/{*path}")
string get_file(@path("path") string rel_path);
// => GET "/files/{*path}"
```

## 5. Parameter Source Resolution

Each parameter is assigned to a source by the following priority:

1. If the parameter is explicitly annotated with `@path`, source is Path.
2. If the parameter is explicitly annotated with `@query`, source is Query.
3. If the parameter name appears in a route template `{name}`, source is Path.
4. If the parameter name appears in a query-template suffix `{?name,...}`,
   source is Query.
5. Otherwise, apply HTTP-method defaults:
   - `GET/DELETE/HEAD/OPTIONS` -> Query
   - `POST/PUT/PATCH` -> Body

Direction interaction (`in/out/inout`):

- `out` parameters are response-only and MUST NOT participate in request-side
  source resolution.
- `in` and `inout` parameters participate in request-side source resolution
  using the priority rules above.

Name binding rules for `@path` / `@query`:

- Without argument (`@path`, `@query`): the bound name is the parameter name.
- With argument (`@path("id")`, `@query("id")`): the bound name is the
  annotation argument.
- The bound name is used for route template variables and query keys.

Constraints:

- A Path bound name should match a route template variable name. Non-matching
  cases are invalid and should raise an error.
- A parameter can have only one source.
- If a method has multiple bound routes, a Path bound name must appear in every
  bound route template of that method.
- Every route template variable `{name}` must be bound by exactly one
  request-side parameter (`in`/`inout`) resolved to Path source.
- Catch-all template variable `{*name}` follows the same binding rule as
  `{name}`.
- A route template may contain at most one catch-all variable.
- Catch-all variable SHOULD appear at the end of route template.
- Query-template variable names in `{?name1,name2,...}` must be bound by exactly
  one request-side parameter resolved to Query source.
- A route template may contain at most one query-template suffix (`{?...}`), and
  it SHOULD appear at the end of route template.

## 6. Request Encoding

Encoding is selected by `@Consumes`, defaulting to `application/json`. This RFC
specifies JSON request encoding behavior. When media-type negotiation cannot be
satisfied, implementations MUST return an error response (see section 10).

### 6.1 Query Encoding

- Query parameters are encoded into the URL query string.
- Example: `?name=alice&age=18`.

### 6.2 Body Encoding

Body parameters are encoded by parameter count:

- No Body parameters: no request body.
- Exactly 1 Body parameter: encode that value directly.
- 2+ Body parameters: encode an object keyed by parameter names.
- `null`/empty-value encoding follows the selected `@Consumes` media type
  semantics.

Examples:

- Single parameter: `create(User req)` -> body is `{"id":1,"name":"a"}`
- Multiple parameters: `create(string name, uint32 age)` -> body is
  `{"name":"a","age":18}`

## 7. Response Encoding

Encoding is selected by `@Produces`, defaulting to `application/json`. This RFC
specifies JSON response encoding behavior. When response media-type negotiation
cannot be satisfied, implementations MUST return an error response (see section
10).

Response rules:

- Build the response outputs set from:
  - method return value (if return type is not `void`)
  - all `out` and `inout` parameters
- Output shaping:
  - if output count is `0`: no response body
  - if output count is `1`: return that value directly
  - if output count is `>1`: return an object
    - return value field name is fixed as `return` (when return value exists)
    - each `out/inout` parameter uses its parameter name as field name
- Status code and body contract:
  - output count `0` -> `204 No Content`, no response body
  - output count `>=1` -> `200 OK`, JSON body encoded by the rules above
  - `HEAD` is a special case: always `204 No Content`, no response body
- `null`/empty-value encoding follows the selected `@Produces` media type
  semantics.

Examples:

- `string hello()` -> `"ok"`
- `void get_count(out long count)` -> `3`
- `long add(long a, long b, out long sum)` -> `{"return":0,"sum":3}`

## 8. Attribute Mapping

`attribute` and `readonly attribute` map to HTTP operations as follows:

- `readonly attribute T x`
  - `GET /.../x`, returns `T`
- `attribute T x`
  - `GET /.../x`, returns `T`
  - `POST /.../set_x`, request body is single parameter `value: T`, and the
    response is `204 No Content` with no body

## 9. Examples

```idl
interface UserService {
    @get(path="/users/{id}")
    User get_user(uint32 id);

    @post(path="/users")
    User create_user(User req);

    @post(path="/users/search")
    sequence<User> search_user(string name, uint32 age);

    readonly attribute string version;
    attribute string name;
};
```

Behavior:

- `get_user`: `id` is from Path, returns `User`
- `create_user`: single Body parameter, body is direct `User` JSON
- `search_user`: multiple Body parameters, body is `{"name":"...","age":...}`
- `version`: readonly attribute generates `GET`
- `name`: generates `GET` + `POST set_name`

Multi-route example:

```idl
interface MultiPathService {
    @get(path="/hello")
    @path("/hi")
    @path("/greet")
    string greet();
};
```

Behavior:

- `greet` is bound to `/hello`, `/hi`, and `/greet`.

## 10. Error Handling and Validation

Implementations should fail fast at build/registration time for static mapping
issues, and return consistent HTTP errors for runtime request issues.

### 10.1 Build/Registration-Time Validation

The following are invalid and should raise mapping errors before serving
traffic:

- More than one HTTP verb annotation on one method.
- `@path` parameter names that do not appear in any bound route template.
- For multi-route methods, any `@path` parameter name missing from one or more
  bound route templates.
- Any route template variable that has no matching request-side parameter bound
  to Path source.
- A route template containing more than one catch-all variable.
- Any query-template variable that has no matching request-side parameter bound
  to Query source.
- A route template containing more than one query-template suffix (`{?...}`).
- Duplicate final route bindings (`HTTP method + normalized path`) across
  methods.
- Any `@head` method with non-`void` return type.
- Any `@head` method containing `out` or `inout` parameters.

### 10.2 Runtime Request Validation

- Missing required Path/Query parameters: `400 Bad Request`.
- Type conversion failures (for example `uint32` parse failure):
  `400 Bad Request`.
- Unsupported request media type for `@Consumes`: `415 Unsupported Media Type`.
- Requested response media type not satisfiable for `@Produces`:
  `406 Not Acceptable`.

### 10.3 Response Body Shape

- Success response body:
  - uses the output encoding rules from section 7.
  - successful payload is business data (direct value or object, depending on
    section 7 shaping rules).
- Failure response body:
  - MUST be an object with shape:
    - `code`: machine-readable error code.
    - `msg`: human-readable summary.
    - `details`: optional structured error details.

### 10.4 Recommended Status Codes

- Successful read operations: `200 OK`.
- Successful create operations: `201 Created` when a new resource is created,
  otherwise `200 OK`.
- Successful `void` responses with no body: `204 No Content`.
- Method not supported on an existing route: `405 Method Not Allowed`.
- Route not found: `404 Not Found`.

### 10.5 Error Response Example

Failure responses should follow section 10.3, for example:

```json
{
  "code": "INVALID_ARGUMENT",
  "msg": "field 'age' must be >= 0",
  "details": {
    "field": "age",
    "expected": ">= 0"
  }
}
```
