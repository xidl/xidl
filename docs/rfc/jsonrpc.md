# XIDL JSON-RPC Mapping Specification (RFC Draft)

Reference specifications:

- <https://www.jsonrpc.org/specification>
- <https://www.omg.org/spec/DDS-RPC>

## 1. Scope

This document defines how XIDL `interface` declarations are mapped to JSON-RPC
APIs, including:

- RPC method naming
- Request parameter mapping
- Response/result mapping
- Error mapping
- Attribute mapping rules

This document does not define a network transport protocol beyond the JSON-RPC
message model.

Conformance note:

- DDS-RPC defines topic-type synthesis rules (`In` / `Out` / `Result`) for
  DDS-based transport mappings.
- This document defines an XIDL JSON-RPC profile that preserves direction and
  attribute semantics, while mapping them to JSON-RPC `params` and `result`.

## 2. Terminology

- `IDL method`: an operation declared in an `interface`.
- `RPC method name`: the JSON-RPC `method` string.
- `Params object`: the JSON object used as JSON-RPC `params`.
- `Result payload`: the JSON value used as JSON-RPC `result`.

## 3. JSON-RPC Profile

This RFC targets JSON-RPC 2.0 request/response semantics.

Request shape:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "pkg.Interface.method",
  "params": { "arg": 1 }
}
```

Response shape:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": { "ok": true }
}
```

Error response shape:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32601,
    "message": "method not found",
    "data": null
  }
}
```

## 4. Method Name Resolution

RPC method names are derived from module/interface/method identifiers.

Rules:

- Base method name is the raw IDL operation name.
- Fully-qualified RPC method name is dot-joined:
  - `{module_path}.{interface}.{method}`
- If no module path exists:
  - `{interface}.{method}`
- Method-level HTTP annotations (such as `@get`, `@path`, `@query`) do not
  affect JSON-RPC method names.

Examples:

```idl
module math {
  interface Calc {
    long add(long a, long b);
  };
};
```

RPC method name: `math.Calc.add`.

## 5. Request Parameter Mapping

### 5.1 Params Encoding

Method input parameters are encoded as a JSON object in `params`.

Rules:

- Parameter object keys use IDL parameter names.
- Zero parameters -> `params` is an empty object (`{}`) in generated clients.
- One or more parameters -> `params` is an object with one property per
  parameter.

Example:

```idl
long add(long a, long b);
```

`params`:

```json
{ "a": 1, "b": 2 }
```

### 5.2 Direction Attributes (`in/out/inout`)

Direction interaction rules:

- `@in`, `@out`, and `@inout` annotations define parameter direction.
- `in` is the default direction; explicit `@in` is optional.
- `in` parameters are request-side only.
- `out` parameters are response-side only.
- `inout` parameters participate in both request and response sides.

Request mapping rules:

- Request `params` includes:
  - all `in` parameters
  - all `inout` parameters
- Request `params` MUST NOT include `out` parameters.

Response mapping rules (together with return value):

- Response outputs set is composed of:
  - method return value (if return type is not `void`)
  - all `out` parameters
  - all `inout` parameters
- Output shaping is defined in section 6.

## 6. Response Mapping

JSON-RPC `result` is always an object, shaped from the outputs set (return +
`out` + `inout`):

- if output count is `0`: `result` is `{}`
- if output count is `1` and it is return value only:
  - `result` is `{ "return": <value> }`
- if output count is `>=1` and includes `out/inout`:
  - return value field name is fixed as `return` (when return value exists)
  - each `out/inout` parameter uses its parameter name as field name

Examples:

- `void ping()` -> `{}`
- `string hello()` -> `{"return":"ok"}`
- `long add(in long a, in long b, out long sum)` -> `{"return":0,"sum":3}`
- `void get_count(out long count)` -> `{"count":3}`

Profile note:

- DDS-RPC basic topic mapping describes synthesized `In`/`Out` structures and a
  `Result` union (including names like `dummy`, `return_`, and `result`).
- This JSON-RPC RFC is an application-level JSON profile and intentionally maps
  directly to JSON `params` / `result` objects instead of mirroring those
  synthesized topic-type names verbatim.

## 7. Attribute Mapping

Every attribute in the interface maps to implied IDL operations using the
following rules:

1. Each attribute `<attribute-name>` maps to a pair of implied operations in the
   same interface:
   - `get_attribute_<attribute-name>`
   - `set_attribute_<attribute-name>` It is illegal to define user operations
     with the same implied names for the same attribute.
2. `get_attribute_<attribute-name>`:
   - return type is the attribute type
   - accepts no arguments
3. `set_attribute_<attribute-name>`:
   - return type is `void`
   - accepts exactly one argument:
     - argument type is the attribute type
     - argument name is `<attribute-name>`
4. Exception types listed in `getraises` (if any) are treated as if
   `get_attribute_<attribute-name>` had the same set in `raises`.
5. Exception types listed in `setraises` (if any) are treated as if
   `set_attribute_<attribute-name>` had the same set in `raises`.

JSON-RPC method names are then derived from these implied operation names using
section 4 naming rules.

## 8. Type Mapping (JSON-RPC-specific)

This profile follows Rust mapping with JSON-RPC-specific overrides:

- `any`, `object`, `valuebase` -> `serde_json::Value`
- `sequence<T>` -> `Vec<T>`
- `map<K, V>` -> `BTreeMap<K, V>`
- `string` / `wstring` -> `String`
- `fixed` -> `f64`

Other scalar and constructed type mappings follow Rust target mapping.

## 9. Error Handling

Standard JSON-RPC error codes should be used:

- Parse error: `-32700`
- Invalid request: `-32600`
- Method not found: `-32601`
- Invalid params: `-32602`
- Internal error: `-32603`
- Server error: `-32000`

Recommended mapping:

- Params decode/validation failure -> `Invalid params`
- Unknown RPC method name -> `Method not found`
- Internal handler failure -> `Server error` (or `Internal error`)

## 10. Validation Rules

Build/generation time validation:

- Duplicate generated RPC names in the same composed service set should be
  treated as invalid by integrators.
- Unsupported generator input should fail fast with clear diagnostics.

Runtime validation:

- Missing/invalid `method` -> invalid request error.
- Params decode failure -> invalid params error.
- Unknown method -> method not found error.
- Response/request ID mismatch -> protocol error.

## 11. Transport Notes

This RFC is transport-neutral at protocol level. A concrete runtime profile may
impose framing. For example, current Rust runtime in this repository uses:

- newline-delimited JSON messages over async streams
- one request/response per line

That framing is an implementation detail, not a JSON-RPC requirement.

## 12. Example

```idl
module demo {
  interface UserService {
    string get_user(string id);
    attribute string name;
  };
};
```

Behavior:

- `get_user` -> method `demo.UserService.get_user`, params `{ "id": "..." }`,
  result `{"return":"..."}`.
- `name` getter -> method `demo.UserService.get_attribute_name`.
- `name` setter -> method `demo.UserService.set_attribute_name`, params
  `{ "name": "..." }`, result `{}`.
