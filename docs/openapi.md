# OpenAPI Metadata Note

The `openapi` target emits `openapi.json` for HTTP-oriented interfaces.

## Generate OpenAPI

```bash
xidlc gen --out-dir generated openapi api.idl
```

## Metadata pragmas

The current OpenAPI generator reads the following `#pragma xidlc` values:

```idl
#pragma xidlc package "Smart City Public APIs"
#pragma xidlc version "v2.0.0"
```

Effects:

- `package` sets `info.title`
- `version` sets `info.version`

If omitted, the generator falls back to default metadata values.

## Related material

- [HTTP Guide](user/http.md)
- [Pragmas Reference](reference/pragmas.md)
- [Targets Reference](reference/targets.md)
