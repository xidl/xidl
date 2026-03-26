# HTTP Guide

This guide explains how to use XIDL for HTTP APIs. It is user-oriented and
focuses on practical behavior. For exact rules, use the RFCs:

- [HTTP RFC](../rfc/http.md)
- [HTTP Stream RFC](../rfc/http-stream.md)
- [HTTP Security RFC](../rfc/http-security.md)

## What the HTTP stack includes

In this repository, the HTTP family covers three related concerns:

1. unary HTTP mapping
2. HTTP stream mapping
3. HTTP security mapping

Typical outputs are:

- generated Axum service and client code with `rust-axum`
- OpenAPI documents with `openapi`

## Basic HTTP mapping

Use HTTP verb annotations on interface methods.

```idl
interface HelloWorld {
    @post(path = "/hello")
    void say_hello(in string name);
};
```

Important practical rules:

- if no HTTP verb annotation is present, the default is equivalent to `POST`
- path parameters are declared with `@path`
- query parameters are declared with `@query`
- header and cookie inputs use `@header` and `@cookie`
- `POST`, `PUT`, and `PATCH` commonly place unannotated inputs in the request
  body
- `GET`, `DELETE`, `HEAD`, and `OPTIONS` commonly place unannotated inputs in
  the query string unless they are bound in the path

## Example: path, query, and body

```idl
interface UserApi {
    @get(path = "/users/{id}{?lang}")
    string get_user(
        @path("id") string id,
        @query("lang") @optional string locale,
        out string display_name
    );

    @post(path = "/users")
    string create_user(
        string display_name,
        string email,
        out string user_id
    );
};
```

## Generated artifacts

With `rust-axum`, each interface currently generates:

- a Rust trait that you implement
- a server wrapper that exposes routes
- a client wrapper
- request payload types used by handlers

With `openapi`, the compiler emits `openapi.json`.

## HTTP Stream

HTTP stream support extends HTTP mapping with long-lived request or response
streams.

Current annotations:

- `@server_stream`
- `@client_stream`
- `@bidi_stream`
- `@stream_codec("ndjson" | "sse")`

Important practical rules from current repository behavior:

- server streams commonly return `sequence<T>`
- client streams commonly use a streaming `sequence<T>` request item
- `sse` is for server streams
- `ndjson` is the default stream codec
- stream support exists, but parts of the stack are still implementation-led and
  should be treated as evolving

Example:

```idl
@http_basic
interface CityHttpStreamApi {
    @server_stream
    @stream_codec("sse")
    @path("/alerts/{district}{?lang}")
    string alerts(
        @path("district") string district,
        @query("lang") string lang
    );
};
```

## HTTP Security

The HTTP security model adds authentication declarations to interfaces or
operations.

Current annotations include:

- `@no_security`
- `@http_basic`
- `@http_bearer`
- `@api_key(...)`
- `@oauth2(...)`

Practical rules:

- interface-level declarations act as defaults
- method-level declarations replace inherited defaults
- `@no_security` clears inherited auth requirements
- security metadata affects request acceptance and generated schema/runtime
  helpers, but not your business parameter list

Example:

```idl
@http_bearer
interface SmartCityHttpApi {
    @get(path = "/v1/citizens/{citizen_id}")
    string get_profile(
        @path("citizen_id") string citizen_id,
        out string display_name
    );

    @head(path = "/v1/parking/lots/{lot_id}")
    @no_security
    void probe_lot(@path("lot_id") string lot_id);
};
```

## `@optional` in HTTP APIs

`@optional` matters in HTTP because omission and explicit values are different.

- optional query values can be omitted
- optional body fields can be absent or nullable depending on the target model
- optional path parameters are invalid

Use the [HTTP RFC](../rfc/http.md) when you need the exact normative rules for
omission, null handling, and validation.

## Suggested workflow

1. Write the interface with HTTP annotations.
2. Generate `rust-axum` output for server/client code.
3. Generate `openapi` output for schema publication.
4. Compare your design with the RFC when behavior is ambiguous.

## Related material

- [JSON-RPC Guide](jsonrpc.md)
- [Targets Reference](../reference/targets.md)
- [Annotations Reference](../reference/annotations.md)
