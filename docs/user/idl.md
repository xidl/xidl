# IDL Guide

This guide explains the XIDL language surface in practical terms. It is not the
formal specification; use the reference section for exact syntax lookup and the
RFCs for transport mapping rules.

## What XIDL adds

XIDL starts from OMG IDL and adds generator-oriented features used by this
repository:

- transport annotations for HTTP and JSON-RPC workflows
- target-oriented annotations such as Rust passthrough attributes
- compiler pragmas such as package and version metadata

## Core building blocks

The repository supports the following families of declarations:

- `module`
- `interface`
- `struct`
- `union`
- `enum`
- `bitmask`
- `bitset`
- `typedef`
- `const`
- `exception`
- `attribute`
- `native`

## Scalar types

Common scalar types include:

- signed and unsigned integers such as `short`, `long`, `long long`, `int32`,
  `uint64`
- floating-point types such as `float`, `double`, `long double`
- `boolean`
- `char`, `wchar`
- `string`, `wstring`
- `octet`
- `any`, `object`, `valuebase`
- `fixed`

## Composite types

The repository also supports composite types such as:

- `sequence<T>`
- `map<K, V>`
- arrays
- nested constructed types

Example:

```idl
module city {
    struct Citizen {
        string id;
        string display_name;
        @optional string phone_number;
    };

    typedef sequence<Citizen> Citizens;
};
```

## Interfaces and methods

Interfaces declare operations and attributes.

```idl
interface HelloWorld {
    string hello(in string name);
    readonly attribute string version;
};
```

Direction rules used across generators:

- `in` is request-side input
- `out` is response-side output
- `inout` participates in both directions

These direction rules matter most for HTTP and JSON-RPC mappings.

## `@optional`

`@optional` is an important XIDL annotation because it preserves omission.

In this repository:

- Rust struct generation can represent optional members as `Option<T>`
- HTTP and OpenAPI generation use `@optional` to distinguish omitted values from
  required ones
- TypeScript generation uses `@optional` to mark optional fields or parameters

Practical guidance:

- use `@optional` on fields or parameters where absence is meaningful
- do not use `@optional` on HTTP path parameters
- read the [HTTP Guide](http.md) for transport-specific omission behavior

## Attributes

IDL attributes become generated accessor operations.

```idl
interface ConfigApi {
    readonly attribute string version;
    attribute boolean maintenance_mode;
};
```

Depending on the target, attributes may become:

- getter and setter methods
- schema entries
- stream watch methods for some stream profiles

## Annotations and pragmas

Annotations change mapping or generation behavior. Pragmas provide compiler
configuration.

Examples:

```idl
@rust-serde(rename_all = "camelCase")
struct User {
    @rust-serde(rename = "userId")
    string user_id;
};
```

```idl
#pragma xidlc package Smart City Public APIs
#pragma xidlc version v2.0.0
```

## How syntax affects generated code

- data declarations affect generated target types
- interfaces affect service, server, client, or schema outputs
- transport annotations affect route names, security metadata, and stream shape
- passthrough annotations may emit target-specific attributes

## Next steps

- For lookup material, read
  [IDL Elements Reference](../reference/idl-elements.md)
- For annotations, read [Annotations Reference](../reference/annotations.md)
- For transport usage, read [HTTP Guide](http.md) or
  [JSON-RPC Guide](jsonrpc.md)
