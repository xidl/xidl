## 1. Definition Format & Parsing

- [x] 1.1 Define a Hurl-like HTTP snapshot definition syntax and file extension
- [x] 1.2 Implement minijinja preprocessing to inject variables before parsing
- [x] 1.3 Build a parser that yields ordered HTTP test steps (method, path, headers, body)

## 2. Snapshot Runner

- [x] 2.1 Implement an HTTP runner that executes steps and captures full transcripts (request/response headers + body)
- [x] 2.2 Add normalization hooks for unstable headers (e.g., Date) and body filtering
- [x] 2.3 Generate one snapshot file per definition file (one file → one snapshot)

## 3. Test Integration

- [x] 3.1 Add discovery similar to `codegen_snapshots_from_idl_folders` for HTTP snapshot definitions
- [x] 3.2 Wire HTTP snapshot tests into `xidlc-examples` test suite

## 4. Example Coverage

- [x] 4.1 Add HTTP snapshot definition for `xidlc-examples/api/http/http_server.idl`
- [x] 4.2 Generate and commit snapshot output for http_server

## 5. Documentation

- [x] 5.1 Document required variables and how minijinja injects them for HTTP snapshot tests
- [x] 5.2 Document how to run/update snapshots locally
