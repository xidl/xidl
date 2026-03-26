# Pragmas Reference

`xidlc` extends IDL `#pragma` directives for code generation settings.

## Syntax

```idl
#pragma xidlc ...
```

## Serialization version

Supported forms:

```idl
#pragma xidlc XCDR1
#pragma xidlc XCDR2
```

Equivalent forms:

```idl
#pragma xidlc serialize(XCDR1)
#pragma xidlc serialize(XCDR2)
```

Effect:

- controls serialization version inference

## Explicit serialization kind

Supported forms:

```idl
#pragma xidlc serialize(CDR)
#pragma xidlc serialize(PLAIN_CDR)
#pragma xidlc serialize(PL_CDR)
#pragma xidlc serialize(PLAIN_CDR2)
#pragma xidlc serialize(DELIMITED_CDR)
#pragma xidlc serialize(PL_CDR2)
```

Effect:

- directly selects serialization kind
- overrides version-based inference

## OpenAPI package metadata

Supported forms:

```idl
#pragma xidlc package rev-tunnel
#pragma xidlc package "rev-tunnel"
```

Effect:

- sets OpenAPI `info.title`

## OpenAPI version metadata

Supported forms:

```idl
#pragma xidlc version v1.0.0
#pragma xidlc version "1.0.0"
```

Effect:

- sets OpenAPI `info.version`

## Precedence and scope

- pragmas are applied in file order
- later values of the same kind win
- quoted and unquoted values are accepted
- `package` and `version` only affect OpenAPI generation

## Related material

- [HTTP Guide](../user/http.md)
- [Targets Reference](targets.md)
- [Original pragma note](../pragma.md)
