# Documentation Migration Matrix

| Source Path                      | Target Section | Target Page                       | Disposition         |
| :------------------------------- | :------------- | :-------------------------------- | :------------------ |
| `docs/user/quickstart.md`        | Guide          | `quickstart.mdx`                  | Move                |
| `docs/user/http.md`              | Guide / Docs   | `first-http-api.mdx` / `http.mdx` | Split/Refactor      |
| `docs/user/jsonrpc.md`           | Docs           | `jsonrpc.mdx`                     | Move                |
| `docs/user/xidlc.md`             | Docs           | `xidlc.mdx`                       | Move                |
| `docs/user/idl.md`               | Docs           | `idl.mdx`                         | Move                |
| `docs/user/language-basic.md`    | Docs           | `idl.mdx`                         | Merge               |
| `docs/user/install.md`           | Guide          | `quickstart.mdx`                  | Merge               |
| `docs/user/xidl-build.md`        | Docs           | `rust-integration.mdx`            | Move                |
| `docs/reference/idl-elements.md` | Docs           | `idl.mdx`                         | Merge               |
| `docs/reference/annotations.md`  | Docs           | `annotations.mdx`                 | Move                |
| `docs/reference/targets.md`      | Docs           | `targets.mdx`                     | Move                |
| `docs/reference/pragmas.md`      | Docs           | `pragmas.mdx`                     | Move                |
| `docs/rfc/http.md`               | RFC            | `http.mdx`                        | Move                |
| `docs/rfc/http-security.md`      | RFC            | `http-security.mdx`               | Move                |
| `docs/rfc/http-stream.md`        | RFC            | `http-stream.mdx`                 | Move                |
| `docs/rfc/jsonrpc.md`            | RFC            | `jsonrpc.mdx`                     | Move                |
| `docs/rfc/jsonrpc-stream.md`     | RFC            | `jsonrpc-stream.mdx`              | Move                |
| `docs/architecture.md`           | Docs           | `index.mdx`                       | Merge into Overview |
| `docs/rust-axum.md`              | Docs           | `rust-integration.mdx`            | Move                |
| `docs/rust-jsonrpc.md`           | Docs           | `jsonrpc.mdx`                     | Merge               |
| `docs/openapi.md`                | Docs           | `http.mdx`                        | Merge               |
| `docs/xcdr.md`                   | RFC            | `xcdr.mdx`                        | New RFC Page        |
| `docs/development/*`             | Defer          | -                                 | Internal use        |
| `docs/ai/*`                      | Defer          | -                                 | Internal use        |

## Notes

- **Guide** focuses on onboarding and "Happy Path" (Rust + HTTP).
- **Docs** provides comprehensive reference and feature deep-dives.
- **RFC** remains normative and stable.
- Many reference pages will be consolidated into `idl.mdx` to reduce navigation
  depth.
