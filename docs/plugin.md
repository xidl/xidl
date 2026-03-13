# Plugin Development Guide

This project uses JSON-RPC 2.0 for plugins. `xidlc` launches the plugin as a
child process and passes an RPC endpoint with `--endpoint <uri>`.

## Conventions

- Plugin executable name: `xidl-<lang>` (for example, `xidl-foo`).
- Invocation example: `idlc --lang foo --out-dir out path/to/file.idl`
- Transport:
  - Unix: `ipc://...` over Unix domain sockets
  - Windows: `tcp://127.0.0.1:PORT`
- Protocol: JSON-RPC 2.0 over the endpoint passed in `--endpoint`.

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
use clap::Parser;
use xidl_jsonrpc::Error;
use xidl_parser::hir::{ParserProperties, Specification};

mod ipc;
use ipc::{Codegen, CodegenServer, GeneratedFile};

struct MyCodegen;

#[async_trait::async_trait]
impl Codegen for MyCodegen {
    async fn get_properties(&self) -> Result<ParserProperties, Error> {
        Ok(ParserProperties::default())
    }

    async fn get_engine_version(&self) -> Result<String, Error> {
        Ok(env!(\"CARGO_PKG_VERSION\").to_string())
    }

    async fn generate(
        &self,
        hir: Specification,
        path: String,
        props: ParserProperties,
    ) -> Result<Vec<GeneratedFile>, Error> {
        let _ = (hir, path, props);
        Ok(Vec::new())
    }
}

#[derive(Parser)]
struct Args {
    #[arg(long)]
    endpoint: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let handler = CodegenServer::new(MyCodegen);
    let _ = xidl_jsonrpc::Server::builder()
        .with_service(handler)
        .serve_on(&args.endpoint)
        .await;
}
```

## Notes

- `hir` is the JSON serialization of `xidl_parser::hir::Specification`.
- Plugins usually derive output filenames from the `input` path.
- If you change parsing or HIR output, update snapshots with `make test-update`.
