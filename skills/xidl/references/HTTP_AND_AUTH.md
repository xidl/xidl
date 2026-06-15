# HTTP Mapping & Security

XIDL provides first-class support for mapping IDL interfaces to HTTP/REST APIs.

## 1. HTTP Verbs and Routes

Annotate methods to define their HTTP endpoints.

```idl
interface UserApi {
    @get(path = "/users/{id}")
    string get_user(@path string id);

    @post(path = "/users")
    void create_user(string name); // Inferred as @body
};
```

**Supported Verbs:** `@get`, `@post`, `@put`, `@patch`, `@delete`, `@head`.

## 2. Parameter Bindings

Explicitly define where parameters come from.

| Annotation | Location                             |
| :--------- | :----------------------------------- |
| `@path`    | URL Path (e.g., `/users/{id}`)       |
| `@query`   | URL Query String (e.g., `?q=search`) |
| `@header`  | HTTP Header                          |
| `@cookie`  | HTTP Cookie                          |
| `@body`    | Request Body                         |

## 3. Security Requirements

Define how an interface or method should be secured.

- **Bearer Token**: `@http_bearer` (uses `Authorization: Bearer <token>`)
- **Basic Auth**: `@http_basic` (uses `Authorization: Basic ...`)
- **API Key**: `@api_key(in = "header", name = "X-Key")`
- **Anonymous**: `@no_security` (overrides interface defaults)

**Example:**

```idl
@http_bearer
interface SecureApi {
    @get(path = "/profile")
    Profile get_profile();

    @get(path = "/ping")
    @no_security
    string ping();
};
```

## 4. CORS

Enable Cross-Origin Resource Sharing.

```idl
@cors
interface PublicApi { ... };
```

## 5. Media Types

Override default JSON serialization.

```idl
@Consumes("application/x-www-form-urlencoded")
@Produces("application/octet-stream")
void upload_data(sequence<uint8> data);
```
