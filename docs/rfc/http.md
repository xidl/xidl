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
- `@Consumes("mime/type")`
- `@Produces("mime/type")`

Parameter annotations:

- `@path` (parameter-level source declaration)
- `@path("name")` (parameter-level source declaration with explicit route/query name)
- `@query` (parameter-level source declaration)
- `@query("name")` (parameter-level source declaration with explicit route/query name)

Rules:

- The HTTP method is determined by HTTP verb annotations, and verb annotations
  are mutually exclusive:
  - A method can have only one of
    `@get/@post/@put/@patch/@delete/@head/@options`.
  - Using more than one verb annotation on the same method is invalid and
    should raise an error.
- If no HTTP verb annotation is present, behavior is equivalent to `@post`.
- Route path can come from a verb annotation `path` argument or from `@path(...)`.
- A method may define multiple paths, and repeated path declarations are allowed:
  - Multiple `@path(...)`
  - Multiple path declarations in supported annotation forms
  - Mixed usage of both sources
- Duplicate routes after normalization should be de-duplicated.
- `Consumes/Produces` define request/response media types. Default is
  `application/json` when unspecified.

## 4. Route Resolution

### 4.1 Defaults

- Default HTTP method: `POST` (equivalent to `@post` when omitted).
- Default route:
  - If explicit method path is not provided, route is auto-generated from
    method and parameter annotations.
  - Base route is `/{method_name}`.
  - `@path` parameters are appended as path template segments.
  - `@query` parameters are appended as URI-template query expansion.
  - Example:
    `void get_name(@query string name)` -> `POST "/get_name{?name}"`.

### 4.2 Explicit Routes and Multi-Route

- If a method explicitly defines route paths (from a verb annotation `path` or
  from `@path(...)`), all defined paths are effective (one method can be bound
  to multiple routes).
- If both sources are present, they are merged.
- If no explicit route is defined, use the default route (see 4.1).
- Route strings should start with `/`.

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

Template variable names are used by parameter source resolution (see section 5).

### 4.4 Auto Path Generation Algorithm

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
4. Collect Query-bound parameter names in declaration order:
   - If non-empty, append URI-template query expansion:
     `{?name1,name2,...}`.
5. Normalize result:
   - Ensure path starts with `/`.
   - Preserve declaration order.
   - If duplicate query names appear, keep first occurrence.

Notes:

- For `GET/DELETE/HEAD/OPTIONS`, unannotated parameters typically become query
  parameters (section 5), so they appear in `{?...}`.
- For `POST/PUT/PATCH`, unannotated parameters typically become body parameters,
  so they do not contribute to generated URI path/query template.

Examples:

```idl
// Example 1: query-only, default POST
void get_name(@query string name);
// => POST "/get_name{?name}"

// Example 2: explicit query name
void get_user(@query("id") uint32 user_id);
// => POST "/get_user{?id}"

// Example 3: path + query
void find_user(@path uint32 id, @query string locale);
// => POST "/find_user/{id}{?locale}"

// Example 4: explicit path and query names
void find_user2(@path("user_id") uint32 id, @query("lang") string locale);
// => POST "/find_user2/{user_id}{?lang}"

// Example 5: GET default source + one explicit path
@get
void list_orders(@path("uid") uint32 user_id, uint32 page, uint32 size);
// => GET "/list_orders/{uid}{?page,size}"

// Example 6: mixed in/out/inout (only request-side params affect path template)
long add(in long a, in long b, out long sum);
// => POST "/add"
```

## 5. Parameter Source Resolution

Each parameter is assigned to a source by the following priority:

1. If the parameter is explicitly annotated with `@path`, source is Path.
2. If the parameter is explicitly annotated with `@query`, source is Query.
3. If the parameter name appears in a route template `{name}`, source is Path.
4. Otherwise, apply HTTP-method defaults:
   - `GET/DELETE/HEAD/OPTIONS` -> Query
   - `POST/PUT/PATCH` -> Body

Name binding rules for `@path` / `@query`:

- Without argument (`@path`, `@query`): the bound name is the parameter name.
- With argument (`@path("id")`, `@query("id")`): the bound name is the
  annotation argument.
- The bound name is used for route template variables and query keys.

Constraints:

- A Path bound name should match a route template variable name.
  Non-matching cases are invalid and should raise an error.
- A parameter can have only one source.

## 6. Request Encoding

Default media type: `application/json`.

### 6.1 Query Encoding

- Query parameters are encoded into the URL query string.
- Example: `?name=alice&age=18`.

### 6.2 Body Encoding

Body parameters are encoded by parameter count:

- No Body parameters: no request body.
- Exactly 1 Body parameter: encode that value directly.
- 2+ Body parameters: encode an object keyed by parameter names.

Examples:

- Single parameter: `create(User req)` -> body is `{"id":1,"name":"a"}`
- Multiple parameters: `create(string name, uint32 age)` ->
  body is `{"name":"a","age":18}`

## 7. Response Encoding

Default media type: `application/json`.

Response rules:

- If there are no `out/inout` parameters, the response body is the method return
  value directly.
- If `out/inout` parameters exist, the response body is encoded as an object:
  - Return value field name is fixed as `return`
  - Each `out/inout` parameter uses its parameter name as field name
- If the method return type is `void` and there are no `out/inout` parameters,
  the response has no body (recommended status: `204 No Content`).

Examples:

- `string hello()` -> `"ok"`
- `long add(long a, long b, out long sum)` -> `{"return":0,"sum":3}`

## 8. Attribute Mapping

`attribute` and `readonly attribute` map to HTTP operations as follows:

- `readonly attribute T x`
  - `GET /.../x`, returns `T`
- `attribute T x`
  - `GET /.../x`, returns `T`
  - `POST /.../set_x`, request body is single parameter `value: T`

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

The following are invalid and should raise mapping errors before serving traffic:

- More than one HTTP verb annotation on one method.
- `@path` parameter names that do not appear in any bound route template.
- Duplicate final route bindings (`HTTP method + normalized path`) across methods.

### 10.2 Runtime Request Validation

- Missing required Path/Query parameters: `400 Bad Request`.
- Type conversion failures (for example `uint32` parse failure): `400 Bad Request`.
- Unsupported request media type for `@Consumes`: `415 Unsupported Media Type`.
- Response media type negotiation failure for `@Produces`: `406 Not Acceptable`.

### 10.3 Recommended Status Codes

- Successful read operations: `200 OK`.
- Successful create operations: `201 Created` when a new resource is created,
  otherwise `200 OK`.
- Successful `void` responses with no body: `204 No Content`.
- Method not supported on an existing route: `405 Method Not Allowed`.
- Route not found: `404 Not Found`.

### 10.4 Error Response Body

Unless overridden by project conventions, runtime errors should return a JSON
object:

```json
{
  "code": "INVALID_ARGUMENT",
  "message": "field 'age' must be >= 0"
}
```

Where:

- `code` is a stable, machine-readable error identifier.
- `message` is a human-readable diagnostic.
