# IDL Elements Reference

This page summarizes the main IDL elements implemented in this repository.

## Modules

Syntax:

```idl
module demo {
    // declarations
};
```

Purpose:

- group declarations
- participate in fully qualified names
- affect generated module or namespace structure in targets such as Rust and C++

## Interfaces

Syntax:

```idl
interface HelloWorld {
    string hello(in string name);
};
```

Purpose:

- declare service operations
- drive HTTP, JSON-RPC, and plugin-oriented generation

See also:

- [HTTP Guide](../user/http.md)
- [JSON-RPC Guide](../user/jsonrpc.md)

## Structs

Purpose:

- model named record types
- generate concrete target-language data types

Important notes:

- optional members are meaningful for Rust, TypeScript, HTTP, OpenAPI, and
  related generators
- Rust passthrough attributes can affect generated source

## Unions

Purpose:

- model tagged alternatives
- generate target-specific tagged representations

Implementation note:

- Rust generation creates a discriminator plus payload model rather than a raw C
  union

## Enums, bitmasks, and bitsets

Purpose:

- express constrained symbolic values
- generate target-language enum or flag structures

## Typedefs

Purpose:

- create aliases over scalar, sequence, map, or array forms

## Constants

Purpose:

- define compile-time constants emitted into generated code

## Exceptions

Purpose:

- describe operation-level exceptional outcomes
- target support varies by generator

## Attributes

Syntax:

```idl
interface ConfigApi {
    readonly attribute string version;
    attribute boolean maintenance_mode;
};
```

Purpose:

- define getter/setter-like interface properties
- map to generated operations or stream watch APIs depending on the profile

## Parameter directions

Supported direction markers:

- `in`
- `out`
- `inout`

Behavior:

- HTTP and JSON-RPC generators use direction to decide request-side and
  response-side shaping
- if omitted, `in` is generally the default

## Template types

### `sequence<T>`

Purpose:

- ordered repeated values
- also used by stream profiles for stream item typing

### `map<K, V>`

Purpose:

- associative values
- target support depends on generator

### Arrays

Purpose:

- fixed-size dimensions attached through declarators

## Scalars

Common scalar families:

- integers
- floating point
- booleans
- characters and strings
- `octet`
- `fixed`
- `any`, `object`, `valuebase`

For deeper target mapping details, inspect:

- `xidlc/doc/idl2rust.md`
- `xidlc/doc/idl2rust-jsonrpc.md`

## Practical lookup strategy

- language syntax: this page
- generator modifiers: [Annotations](annotations.md)
- compiler directives: [Pragmas](pragmas.md)
- output selection: [Targets](targets.md)
