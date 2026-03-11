# Plugin Development Guide

This project uses JSON-RPC 2.0 for plugins. `xidlc` launches the plugin as a
child process and communicates over stdin/stdout with one JSON message per line.

## Conventions

- Plugin executable name: `xidl-<lang>` (for example, `xidl-foo`).
- Invocation example: `idlc --lang foo --out-dir out path/to/file.idl`
- Protocol: JSON-RPC 2.0, UTF-8, one line per request/response.

## Required Methods

Plugins must implement two methods:

1. `parser_properties` Returns parser options. Currently:

```json
{ "expand_interface": true }
```

2. `generate` Params:

```json
{
  "hir": "<HIR JSON>",
  "input": "<input file path>"
}
```

Returns:

```json
{
  "files": [
    { "filename": "xx", "filecontent": "yy" }
  ]
}
```

## Request/Response Examples

`parser_properties` request:

```json
{ "jsonrpc": "2.0", "id": 1, "method": "parser_properties" }
```

Response:

```json
{ "jsonrpc": "2.0", "id": 1, "result": { "expand_interface": true } }
```

`generate` request:

```json
{"jsonrpc":"2.0","id":2,"method":"generate","params":{"hir":{...},"input":"path/to/file.idl"}}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": { "files": [{ "filename": "out.txt", "filecontent": "..." }] }
}
```

Error response:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "error": { "code": -32000, "message": "...", "data": null }
}
```

## Rust Plugin Example

Generate the IPC bindings from `ipc.idl`, then use the generated `Codegen`
interface in your plugin:

```sh
idlc --lang rust_jsonrpc --out-dir src ipc.idl
```

```rust
use std::io::{BufRead, Write};
use xidl_jsonrpc::Error;
use xidl_parser::hir::{ParserProperties, Specification};

mod ipc;
use ipc::{Codegen, CodegenServer, GeneratedFile};

struct MyCodegen;

impl Codegen for MyCodegen {
    fn parser_properties(&self) -> Result<ParserProperties, Error> {
        Ok(ParserProperties::default())
    }

    fn generate(
        &self,
        hir: Specification,
        input: String,
    ) -> Result<Vec<GeneratedFile>, Error> {
        let _ = (hir, input);
        Ok(Vec::new())
    }
}

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let handler = CodegenServer::new(MyCodegen);
    let _ = xidl_jsonrpc::serve(stdin.lock(), stdout.lock(), handler);
}
```

## Notes

- `hir` is the JSON serialization of `xidl_parser::hir::Specification`.
- Plugins usually derive output filenames from the `input` path.
- If you change parsing or HIR output, update snapshots with `make test-update`.
