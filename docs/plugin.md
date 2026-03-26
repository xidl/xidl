# Plugin Development Guide

XIDL supports external generators through a plugin model. A plugin is a process
that `xidlc` launches, then communicates with over JSON-RPC 2.0.

## How plugins fit into the compiler

The compiler pipeline is:

1. parse IDL
2. lower it to HIR
3. pick a built-in generator or external plugin
4. send generation requests
5. write returned artifacts

Plugins only replace the final generation stage. They do not replace parsing,
diagnostics, or HIR creation.

`xidlc` launches the plugin as a child process and passes an RPC endpoint with
`--endpoint <uri>`.

## Conventions

- Plugin executable name: `xidl-<lang>` (for example, `xidl-foo`).
- Invocation example: `xidlc --lang foo --out-dir out path/to/file.idl`
- Transport:
  - Unix: `ipc://...` over Unix domain sockets
  - Windows: `tcp://127.0.0.1:PORT`
- Protocol: JSON-RPC 2.0 over the endpoint passed in `--endpoint`.

The `<lang>` portion is the target string the user passes to `--lang`.

## Required Methods

Plugins must implement two required methods that the driver calls during setup
and generation.

1. `parser_properties`

Returns parser options. Currently:

```json
{ "expand_interface": true }
```

Use this to tell the compiler which parser-side behavior your plugin expects.

2. `generate`

Parameters:

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

This is the core generation call. The plugin receives HIR and returns one or
more generated files.

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

The repository already uses JSON-RPC runtime support, so Rust is the easiest way
to build a plugin.

### Step 1. Generate the IPC bindings

Generate the RPC bindings from `ipc.idl`, then use the generated `Codegen`
interface in your plugin:

```sh
xidlc --lang rust-jsonrpc --out-dir src ipc.idl
```

### Step 2. Implement the generated trait

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
        Ok(env!("CARGO_PKG_VERSION").to_string())
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

### Step 3. Invoke it through xidlc

If your plugin binary is named `xidl-foo`, users can run:

```bash
xidlc --lang foo --out-dir out path/to/file.idl
```

The compiler will resolve `foo` to the external executable, start it, and
deliver the generation request over the endpoint passed in `--endpoint`.

## End-to-end plugin workflow

1. Choose a target name such as `foo`.
2. Build an executable named `xidl-foo`.
3. Implement `parser_properties`.
4. Implement `generate`.
5. Return a file list from `generate`.
6. Run `xidlc --lang foo ...` against a sample IDL file.
7. Verify the produced files in the output directory.

## Data contract details

- `hir` is the JSON serialization of `xidl_parser::hir::Specification`
- `input` is the original input path
- returned files contain `filename` and `filecontent`
- file writing is handled by the compiler driver after the RPC response

## Design guidance for plugin authors

- treat HIR as the source of truth instead of reparsing source text
- keep output filenames deterministic
- fail with clear JSON-RPC errors when input is unsupported
- document any plugin-specific annotations or conventions you add
- keep parser property requirements minimal unless your generator truly needs
  them

## Related files in this repository

- `xidlc/src/driver/generate_session.rs`
- `xidlc/src/driver/lang.rs`
- `xidl-jsonrpc/`

## Notes

- Plugins usually derive output filenames from the `input` path.
- If you change parsing or HIR output, update snapshots with `make test-update`.
