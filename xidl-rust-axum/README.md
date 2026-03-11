# Axum

The Axum generator implements the [HTTP](rfc/http.md) RFC and allows you to
generate Axum-based server and client code from IDL.

## Quick start

### 1) Write IDL

```idl
interface HelloWorld {
    @post(path = "/hello")
    void sayHello(
        in string name
    );
};
```

### 2) Generate Rust Axum code

```bash
xidlc --lang axum \
  --out-dir generated \
  hello_world.idl

xidlc --lang openapi \
  --out-dir generated \
  hello_world.idl
```

Example output: `xidlc-examples/examples/imp/hello_world.rs`.

### 3) Implement server

```rust
mod imp;

use imp::HelloWorld;
use imp::HelloWorldServer;

struct HelloWorldImpl;

#[async_trait::async_trait]
impl HelloWorld for HelloWorldImpl {
    async fn sayHello(
        &self,
        req: xidl_rust_axum::Request<imp::HelloWorldSayHelloRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        println!("Hello, {}!", req.data.name);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    xidl_rust_axum::Server::builder()
        .with_service(HelloWorldServer::new(HelloWorldImpl))
        .serve("127.0.0.1:3000")
        .await?;
    Ok(())
}
```

### 4) Call client

```rust
mod imp;

use imp::HelloWorldClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = HelloWorldClient::new("http://127.0.0.1:3000");
    client.sayHello("World".to_string()).await?;
    Ok(())
}
```

## More examples

### Path + query (GET)

By default, `GET/DELETE/HEAD/OPTIONS` parameters become query parameters unless
they appear in the path template.

```idl
struct User {
    string id;
    string name;
    string email;
};

interface UserApi {
    @get(path = "/users/{id}")
    User getUser(
        in string id,
        in string fields
    );
};
```

Server-side usage:

```rust
#[async_trait::async_trait]
impl UserApi for UserApiImpl {
    async fn getUser(
        &self,
        req: xidl_rust_axum::Request<imp::UserApiGetUserRequest>,
    ) -> Result<imp::User, xidl_rust_axum::Error> {
        let id = req.data.id;
        let fields = req.data.fields; // ?fields=...
        let _request_id = req
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_string());
        Ok(imp::User {
            id,
            name: "alice".to_string(),
            email: "alice@example.com".to_string(),
        })
    }
}
```

Client-side usage:

```rust
let client = imp::UserApiClient::new("http://127.0.0.1:3000");
let user = client
    .getUser("u1".to_string(), "name,email".to_string())
    .await?;
```

### Query only (GET)

```idl
interface SearchApi {
    @get(path = "/search")
    void search(
        in string q,
        in i32 limit,
        in i32 offset
    );
};
```

Client-side usage:

```rust
let client = imp::SearchApiClient::new("http://127.0.0.1:3000");
client.search("rust".to_string(), 10, 0).await?;
```

### JSON body (POST)

For `POST/PUT/PATCH`, parameters are encoded into the JSON body by default.

```idl
struct User {
    string id;
    string name;
    string email;
};

interface UserApi {
    @post(path = "/users")
    User createUser(
        in string name,
        in string email
    );
};
```

Client-side usage:

```rust
let user = client
    .createUser("alice".to_string(), "alice@example.com".to_string())
    .await?;
```

## Codegen details

`xidlc --lang rust-axum` generates, for each `interface`:

- `trait`: business interface definition (you implement it)
- `Server`: Axum router wrapper that wires routes and request parsing
- `Client`: HTTP client wrapper based on `reqwest`
- `Request<T>`: carries headers and parsed request data

To generate only server or client code:

```bash
cargo run -p xidlc -- --lang rust-axum --server ...
cargo run -p xidlc -- --lang rust-axum --client ...
cargo run -p xidlc -- --lang rust-axum --ts ...
```

## Error handling

`xidl_rust_axum::Error` is the common error type shared by generated code.

```rust
return Err(xidl_rust_axum::Error::new(400, "invalid name"));
```

Server behavior:

- Converts `Error` into an HTTP response with the same status code.
- Response body is JSON: `{ "code": <u16>, "msg": <string> }`.

Client behavior:

- For non-2xx responses, it tries to parse `ErrorBody` and returns it as
  `Error`.
- If the body is not a valid `ErrorBody`, it returns
  `Error::new(<status>, "http error: <status>")`.

## Notes

- Generated code references `xidl_rust_axum::axum` / `xidl_rust_axum::serde` to
  reduce dependency version conflicts.
- `Error` implements `IntoResponse`, so on the server side you can use `?` to
  return an HTTP error response.
