# Using xidlc

`xidlc` reads one or more `.idl` files and emits artifacts for a selected
target.

## Minimal command

```bash
xidlc --lang rust --out-dir out path/to/file.idl
```

Short form:

```bash
xidlc -l rust -o out path/to/file.idl
```

## CLI options

The current generator arguments are:

- `--lang`, `-l`: select the target language or generator
- `--out-dir`, `-o`: choose the output directory
- `--client`: request client-only output for generators that support it
- `--server`: request server output for generators that support it
- `--dry-run`: parse and generate without writing files

## Supported built-in targets

The repository currently recognizes these built-in targets:

- `c`
- `cpp`
- `rust`
- `rust-jsonrpc`
- `rust-axum`
- `ts` / `typescript`
- `openapi`
- `openrpc`
- `hir`
- `typed-ast`

Alias examples:

- `rust-axum`, `rust_axum`, and `axum`
- `rust-jsonrpc`, `rust_jsonrpc`
- `ts`, `typescript`

## Typical workflows

### Generate Rust types

```bash
xidlc --lang rust --out-dir src/generated api.idl
```

### Generate an Axum server/client surface

```bash
xidlc --lang rust-axum --out-dir src/generated api.idl
```

### Generate OpenAPI

```bash
xidlc --lang openapi --out-dir generated api.idl
```

### Generate OpenRPC

```bash
xidlc --lang openrpc --out-dir generated api.idl
```

## End-to-end flow

1. Write IDL definitions for your data and interfaces.
2. Choose a target based on the runtime or schema artifact you need.
3. Run `xidlc`.
4. Compile or publish the generated output with the target’s runtime crate or
   consumer toolchain.

## Choosing a target

- Choose `rust` when you need Rust data types or shared models.
- Choose `rust-axum` when your interface is HTTP-oriented and you want generated
  server/client scaffolding for Axum.
- Choose `rust-jsonrpc` when your interface is JSON-RPC oriented.
- Choose `openapi` when you need an OpenAPI document for HTTP-oriented
  interfaces.
- Choose `openrpc` when you need a JSON-RPC schema document.
- Choose `typescript` when you need TypeScript declarations and schemas from
  supported definitions.

For a detailed capability matrix, see
[Targets Reference](../reference/targets.md).

## Formatting

The compiler also exposes a formatter subcommand:

```bash
xidlc fmt path/to/file.idl
```

Use this when you want consistent formatting for supported languages and query
files.

## Related guides

- [Using xidl-build in Rust](xidl-build.md)
- [IDL Guide](idl.md)
- [HTTP Guide](http.md)
- [JSON-RPC Guide](jsonrpc.md)
