# HTTP receiver publication

This fixture uses the ordinary generated client and server. The marker changes
OpenAPI publication only. Types come from a second IDL module; the same URL has
an unmarked GET and a marked POST with a response and declared exception.

The application accepts any string event type, validates invoice amounts and
owns delivery identity. Equal replay returns the same receipt; conflicting
replay fails without changing state. Shared Hurl assertions cover malformed
JSON, strict body types, missing fields and business rejection.

The in-memory store demonstrates the application boundary. It is not durable
delivery infrastructure. Signing, destination selection and retries are outside
this fixture. Version projection and paired output comparisons live in
`xidlc/tests/webhook_publication.rs`.
