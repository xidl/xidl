# XIDLC Compiler Pragma Extensions

`xidlc` extends IDL preprocessor directives to control code generation behavior.

## Syntax

Use standard `#pragma` directives (case-insensitive):

```idl
#pragma xidlc ...
```

## Supported Pragmas

### 1. Serialization version

```idl
#pragma xidlc XCDR1
#pragma xidlc XCDR2
```

Equivalent form:

```idl
#pragma xidlc serialize(XCDR1)
#pragma xidlc serialize(XCDR2)
```

Effect:

- Sets `SerializeVersion` (`Xcdr1` or `Xcdr2`).
- Affects default `SerializeKind` inference for generated types.

### 2. Explicit serialization kind

```idl
#pragma xidlc serialize(CDR)
#pragma xidlc serialize(PLAIN_CDR)
#pragma xidlc serialize(PL_CDR)
#pragma xidlc serialize(PLAIN_CDR2)
#pragma xidlc serialize(DELIMITED_CDR)
#pragma xidlc serialize(PL_CDR2)
```

Effect:

- Directly sets `SerializeKind`.
- Has higher priority than `XCDR1`/`XCDR2` version-based inference.

### 3. OpenAPI title (`package`)

```idl
#pragma xidlc package rev-tunnel
#pragma xidlc package "rev-tunnel"
```

Effect:

- Sets OpenAPI `info.title`.

### 4. OpenAPI version (`version`)

```idl
#pragma xidlc version v1.0.0
#pragma xidlc version "1.0.0"
```

Effect:

- Sets OpenAPI `info.version`.

## Scope and precedence

- Pragmas are applied in file order; later values override earlier values of the
  same kind.
- `package` and `version` only affect OpenAPI generation.
- `XCDR1`, `XCDR2`, and `serialize(...)` affect serialization configuration.
- Values may be quoted with single or double quotes; outer quotes are trimmed.

## Example

```idl
#pragma xidlc package "xidlc"
#pragma xidlc version "1.2.0"
#pragma xidlc XCDR2

module demo {
    struct User {
        long id;
        string name;
    };
};
```
